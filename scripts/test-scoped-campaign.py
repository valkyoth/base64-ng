#!/usr/bin/env python3
"""Signed-history, approval-boundary and claim-scope regression tests."""

import copy
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import scoped_campaign as campaign


REAL_ROOT = campaign.ROOT


class CampaignTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="base64-scoped-history-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.git("init", "-q")
        self.git("config", "user.name", "fixture")
        self.git("config", "user.email", "fixture@example.invalid")
        key = self.root / ".git/key"
        subprocess.run(["ssh-keygen", "-q", "-t", "ed25519", "-N", "", "-f", str(key)],
                       check=True, capture_output=True)
        self.git("config", "gpg.format", "ssh")
        self.git("config", "user.signingkey", str(key))
        self.git("config", "commit.gpgsign", "true")
        for name in campaign.POLICY_FILES - campaign.POLICY_CHANGE_FILES:
            self.write(name, (REAL_ROOT / name).read_text())
        self.write("security/release-signers", 'fixture@example.invalid namespaces="git" '
                   + key.with_suffix(".pub").read_text())
        self.write("src/lib.rs", "original codec\n")
        base = self.commit("frozen")
        self.write("src/lib.rs", "reviewed reporting correction\n")
        correction = self.commit("correction")
        self.write("docs/assessment.md", "assessment\n")
        assessment = self.commit("assessment")
        self.policy = copy.deepcopy(campaign.load_policy(REAL_ROOT))
        self.policy["history"] = [self.record(c) for c in (base, correction, assessment)]
        self.policy["sources"] = {"frozen": self.identity(base),
                                  "reporting": self.identity(correction),
                                  "recapture": self.identity(assessment)}
        # Every path in the initial reviewed policy commit is explicitly inventoried.
        for name in campaign.POLICY_FILES:
            data = (json.dumps(self.policy) if name == campaign.POLICY_PATH
                    else (REAL_ROOT / name).read_text())
            self.write(name, data)
        self.anchor = self.commit("reviewed scoped policy")

    def git(self, *args):
        return campaign.git(self.root, *args).decode().strip()

    def write(self, name, content):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)

    def commit(self, message, signed=True):
        self.git("add", ".")
        self.git("-c", "commit.gpgsign=" + str(signed).lower(), "commit", "-qm", message)
        return self.git("rev-parse", "HEAD")

    def identity(self, commit):
        return {"commit": commit, "tree": self.git("rev-parse", commit + "^{tree}")}

    def record(self, commit):
        return dict(self.identity(commit), paths=campaign.changed_paths(self.root, commit))

    def check(self):
        campaign.verify_history(self.root, self.policy, self.anchor)

    def test_exact_signed_history_and_policy(self):
        self.check()

    def test_proposal_does_not_claim_candidate_approval(self):
        campaign.verify_history(self.root, self.policy)
        with patch.object(campaign, "verify_packet", return_value={}), patch.object(campaign, "verify_claims"):
            report = campaign.verify(self.root, self.policy, {"historical": "old", "recapture": "new"})
        self.assertFalse(report["release_authorized"])
        self.assertFalse(report["candidate_validated"])
        self.assertIsNone(report["policy_commit"])
        self.assertEqual(report["status"], "proposal-preflight-not-approved")

    def test_unsigned_policy(self):
        self.git("-c", "commit.gpgsign=false", "commit", "--amend", "--no-edit")
        self.anchor = self.git("rev-parse", "HEAD")
        with self.assertRaisesRegex(ValueError, "git check failed"):
            self.check()

    def test_wrong_signer(self):
        key = self.root / ".git/other-key"
        subprocess.run(["ssh-keygen", "-q", "-t", "ed25519", "-N", "", "-f", str(key)],
                       check=True, capture_output=True)
        self.git("config", "user.signingkey", str(key))
        self.git("commit", "--amend", "--no-edit")
        self.anchor = self.git("rev-parse", "HEAD")
        with self.assertRaisesRegex(ValueError, "git check failed"):
            self.check()

    def test_unsigned_history_is_rejected_even_when_its_identity_is_pinned(self):
        self.write("docs/extra.md", "unsigned history\n")
        unsigned = self.commit("unsigned", signed=False)
        self.policy["history"].extend([self.record(self.anchor), self.record(unsigned)])
        with self.assertRaisesRegex(ValueError, "git check failed"):
            campaign.verify_history(self.root, self.policy)

    def test_alternative_policy_does_not_match_independent_anchor(self):
        self.git("commit", "--amend", "-qm", "different policy identity")
        with self.assertRaisesRegex(ValueError, "exact current HEAD"):
            self.check()

    def test_drift_is_detected_even_if_git_status_hides_it(self):
        self.git("update-index", "--assume-unchanged", "scripts/scoped_campaign.py")
        self.write("scripts/scoped_campaign.py", "unreviewed replacement\n")
        self.assertEqual(self.git("status", "--porcelain"), "")
        with self.assertRaisesRegex(ValueError, "differs from reviewed"):
            self.check()

    def test_wrong_tree_inventory_or_order(self):
        for mutation in ("tree", "paths", "order", "duplicate"):
            with self.subTest(mutation=mutation):
                policy = copy.deepcopy(self.policy)
                if mutation == "tree":
                    policy["history"][1]["tree"] = "0" * 40
                elif mutation == "paths":
                    policy["history"][1]["paths"] = []
                elif mutation == "order":
                    policy["history"][0:2] = reversed(policy["history"][0:2])
                else:
                    policy["history"].insert(1, policy["history"][1])
                with self.assertRaises(ValueError):
                    campaign.verify_history(self.root, policy, self.anchor)

    def test_unknown_source(self):
        self.policy["sources"]["recapture"]["commit"] = "0" * 40
        with self.assertRaisesRegex(ValueError, "outside pinned"):
            self.check()

    def test_dirty_policy_and_protected_source(self):
        for name in ("src/lib.rs", "scripts/scoped_campaign.py", "untracked"):
            with self.subTest(name=name):
                path = self.root / name
                original = path.read_bytes() if path.exists() else None
                path.write_text("changed\n")
                with self.assertRaisesRegex(ValueError, "dirty"):
                    self.check()
                if original is None:
                    path.unlink()
                else:
                    path.write_bytes(original)

    def test_extra_runtime_path_in_policy(self):
        self.write("src/lib.rs", "unapproved change\n")
        self.git("add", ".")
        self.git("commit", "--amend", "--no-edit")
        self.anchor = self.git("rev-parse", "HEAD")
        with self.assertRaisesRegex(ValueError, "path inventory"):
            self.check()

    def test_no_post_policy_changes_even_if_reverted(self):
        self.write("src/lib.rs", "unapproved change\n")
        self.commit("runtime change")
        self.write("src/lib.rs", "reviewed reporting correction\n")
        self.commit("runtime revert")
        with self.assertRaisesRegex(ValueError, "exact current HEAD"):
            self.check()
        with self.assertRaisesRegex(ValueError, "directly follow"):
            campaign.verify_history(self.root, self.policy, self.git("rev-parse", "HEAD"))

    def test_missing_policy_file(self):
        self.git("rm", "scripts/scoped_evidence.py")
        self.git("commit", "--amend", "--no-edit")
        self.anchor = self.git("rev-parse", "HEAD")
        with self.assertRaisesRegex(ValueError, "path inventory"):
            self.check()

    def test_replace_objects_and_inherited_git_environment_are_ignored(self):
        with patch.dict(os.environ, {"GIT_DIR": "/does-not-exist", "GIT_NO_REPLACE_OBJECTS": "0"}):
            self.check()
        self.git("replace", self.policy["history"][1]["commit"], self.anchor)
        self.check()

    def test_cli_missing_or_conflicting_anchor_fails_before_packet_access(self):
        command = ["python3", str(REAL_ROOT / "scripts/validate-scoped-campaign.py"),
                   "--historical-packet", "/missing-old", "--recapture-packet", "/missing-new"]
        env = dict(os.environ)
        env.pop(campaign.POLICY_ENV, None)
        result = subprocess.run(command, env=env, capture_output=True, text=True, timeout=10)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("must come from independent review", result.stderr)
        env[campaign.POLICY_ENV] = self.anchor
        result = subprocess.run(command + ["--inspect-proposal"], env=env,
                                capture_output=True, text=True, timeout=10)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("must not receive", result.stderr)


class ClaimTests(unittest.TestCase):
    def setUp(self):
        self.policy = copy.deepcopy(campaign.load_policy(REAL_ROOT))
        self.summaries = copy.deepcopy(campaign.load_attestations(REAL_ROOT, self.policy))
        self.packets = {name: {"files": {}, "directories": []} for name in self.policy["packets"]}
        for claim in self.policy["claims"]:
            for ref in claim["evidence"]:
                self.packets[ref["packet"]]["files"][ref["path"]] = {"bytes": 0, "sha256": "0" * 64}
        for claim in self.policy["claims"]:
            attestation = self.summaries["attestations"][claim["id"]]
            for ref in attestation["evidence"]:
                packet = self.packets[ref["packet"]]
                ref["inventory"] = campaign.json_record(campaign.selected_inventory(packet, ref["path"]))
                if "nested_summary" in ref:
                    ref["nested_summary"]["archive"] = packet["files"][ref["path"]]
            claim["attestation_record"] = campaign.json_record(attestation)

    def check(self, policy=None):
        campaign.verify_claims(self.policy if policy is None else policy, self.packets, self.summaries)

    def test_explicit_scopes_and_exclusions(self):
        self.check()
        self.assertEqual(len(self.policy["claims"]), 14)
        self.assertEqual(len(self.policy["sources"]), 3)

    def test_missing_or_empty_evidence(self):
        ref = self.policy["claims"][0]["evidence"][0]
        packet = self.packets[ref["packet"]]
        del packet["files"][ref["path"]]
        with self.assertRaisesRegex(ValueError, "missing or empty"):
            self.check()
        packet["directories"].append(ref["path"])
        with self.assertRaisesRegex(ValueError, "missing or empty"):
            self.check()

    def test_bad_claims(self):
        for field, value in (("source", "unknown"), ("disposition", "accepted-all"),
                             ("scope", ""), ("excludes", ""), ("evidence", [])):
            with self.subTest(field=field):
                policy = copy.deepcopy(self.policy)
                policy["claims"][0][field] = value
                with self.assertRaises(ValueError):
                    self.check(policy)

    def test_duplicate_claims_or_removed_requirements(self):
        self.policy["claims"].append(self.policy["claims"][0])
        with self.assertRaisesRegex(ValueError, "duplicate claim"):
            self.check()
        self.policy["claims"].pop()
        self.policy["pending"].pop()
        with self.assertRaisesRegex(ValueError, "requirements"):
            self.check()

    def test_cannot_authorize_release(self):
        self.policy["release_authorized"] = True
        with self.assertRaisesRegex(ValueError, "non-authorizing"):
            self.check()

    def test_missing_or_additional_packet_is_rejected_before_git(self):
        for paths in ({}, {"historical": "old"}, {"historical": "old", "recapture": "new", "extra": "x"}):
            with self.subTest(paths=paths), patch.object(campaign, "git", side_effect=AssertionError):
                with self.assertRaisesRegex(ValueError, "exact historical"):
                    campaign.verify(Path("/missing"), self.policy, paths)

    def test_reported_native_abi_to_fuzz_substitution(self):
        claim = next(c for c in self.policy["claims"] if c["id"] == "native-abi-admission")
        claim["source"] = "frozen"
        claim["disposition"] = "historical-scoped-reuse"
        claim["evidence"] = [{"packet": "historical", "path": "historical-staging/fuzz"}]
        with self.assertRaisesRegex(ValueError, "source commit/tree mismatch"):
            self.check()

    def test_redirect_only_does_not_satisfy_required_attestation(self):
        claim = self.policy["claims"][0]
        claim["evidence"] = self.policy["claims"][1]["evidence"]
        with self.assertRaisesRegex(ValueError, "reference mismatch"):
            self.check()

    def test_attestation_fields_and_hash(self):
        claim = self.policy["claims"][0]
        original = copy.deepcopy(self.summaries["attestations"][claim["id"]])
        for field, value in (("source_commit", "0" * 40), ("source_tree", "0" * 40),
                             ("status", "failed"), ("campaign", "native-abi-admission"),
                             ("disposition", "fresh-recapture"), ("scope", "all code"),
                             ("excludes", "no exclusions")):
            with self.subTest(field=field):
                altered = dict(original, **{field: value})
                self.summaries["attestations"][claim["id"]] = altered
                # Even a newly computed hash cannot legitimize contradictory semantics.
                claim["attestation_record"] = campaign.json_record(altered)
                with self.assertRaises(ValueError):
                    self.check()
        self.summaries["attestations"][claim["id"]] = original
        claim["attestation_record"]["sha256"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "digest mismatch"):
            self.check()

    def test_required_report_inventory_and_empty_extra_directory(self):
        ref = self.policy["claims"][0]["evidence"][0]
        packet = self.packets[ref["packet"]]
        for kind in ("file", "directory"):
            with self.subTest(kind=kind):
                name = ref["path"] + "/unreviewed"
                if kind == "file":
                    packet["files"][name] = {"bytes": 0, "sha256": "0" * 64}
                else:
                    packet["directories"].append(name)
                with self.assertRaisesRegex(ValueError, "required file/directory"):
                    self.check()
                packet["files"].pop(name, None)

    def test_missing_attestation_or_packet_manifest_pin(self):
        claim = self.policy["claims"][0]
        summary = self.summaries["attestations"].pop(claim["id"])
        with self.assertRaisesRegex(ValueError, "campaign inventory"):
            self.check()
        self.summaries["attestations"][claim["id"]] = summary
        self.policy["packets"]["historical"]["manifest"]["sha256"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "packet manifest"):
            self.check()

    def test_opaque_supplements_require_complete_source_bound_summary(self):
        claim = next(c for c in self.policy["claims"] if c["id"] == "reporting-native")
        original = copy.deepcopy(self.summaries["attestations"][claim["id"]])
        for mutation in ("missing", "archive", "source", "status", "report", "inventory"):
            with self.subTest(mutation=mutation):
                summary = copy.deepcopy(original)
                ref = summary["evidence"][0]
                nested = ref["nested_summary"]
                if mutation == "missing":
                    del ref["nested_summary"]
                elif mutation == "archive":
                    nested["archive"]["sha256"] = "1" * 64
                elif mutation == "source":
                    nested["source_commit"] = "1" * 40
                elif mutation == "status":
                    nested["status"] = "failed"
                elif mutation == "report":
                    nested["passed_reports"][0]["status"] = "failed"
                else:
                    nested["complete_inventory"]["sha256"] = "not-a-digest"
                self.summaries["attestations"][claim["id"]] = summary
                claim["attestation_record"] = campaign.json_record(summary)
                with self.assertRaises(ValueError):
                    self.check()

    def test_top_level_summary_bytes_are_pinned(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / campaign.ATTESTATIONS_PATH
            path.parent.mkdir()
            path.write_bytes((REAL_ROOT / campaign.ATTESTATIONS_PATH).read_bytes())
            campaign.load_attestations(root, self.policy)
            path.write_bytes(path.read_bytes() + b" ")
            with self.assertRaisesRegex(ValueError, "summary digest mismatch"):
                campaign.load_attestations(root, self.policy)


if __name__ == "__main__":
    unittest.main()
