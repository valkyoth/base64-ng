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
        self.packets = {name: {"files": {}, "directories": []} for name in self.policy["packets"]}
        for claim in self.policy["claims"]:
            for ref in claim["evidence"]:
                self.packets[ref["packet"]]["files"][ref["path"]] = {}

    def test_explicit_scopes_and_exclusions(self):
        campaign.verify_claims(self.policy, self.packets)
        self.assertEqual(len(self.policy["claims"]), 14)
        self.assertEqual(len(self.policy["sources"]), 3)

    def test_missing_or_empty_evidence(self):
        ref = self.policy["claims"][0]["evidence"][0]
        packet = self.packets[ref["packet"]]
        del packet["files"][ref["path"]]
        with self.assertRaisesRegex(ValueError, "missing evidence"):
            campaign.verify_claims(self.policy, self.packets)
        packet["directories"].append(ref["path"])
        with self.assertRaisesRegex(ValueError, "empty evidence"):
            campaign.verify_claims(self.policy, self.packets)

    def test_bad_claims(self):
        for field, value in (("source", "unknown"), ("disposition", "accepted-all"),
                             ("scope", ""), ("excludes", ""), ("evidence", [])):
            with self.subTest(field=field):
                policy = copy.deepcopy(self.policy)
                policy["claims"][0][field] = value
                with self.assertRaises(ValueError):
                    campaign.verify_claims(policy, self.packets)

    def test_duplicate_claims_or_removed_requirements(self):
        self.policy["claims"].append(self.policy["claims"][0])
        with self.assertRaisesRegex(ValueError, "duplicate claim"):
            campaign.verify_claims(self.policy, self.packets)
        self.policy["claims"].pop()
        self.policy["pending"].pop()
        with self.assertRaisesRegex(ValueError, "requirements"):
            campaign.verify_claims(self.policy, self.packets)

    def test_cannot_authorize_release(self):
        self.policy["release_authorized"] = True
        with self.assertRaisesRegex(ValueError, "non-authorizing"):
            campaign.verify_claims(self.policy, self.packets)

    def test_missing_or_additional_packet_is_rejected_before_git(self):
        for paths in ({}, {"historical": "old"}, {"historical": "old", "recapture": "new", "extra": "x"}):
            with self.subTest(paths=paths), patch.object(campaign, "git", side_effect=AssertionError):
                with self.assertRaisesRegex(ValueError, "exact historical"):
                    campaign.verify(Path("/missing"), self.policy, paths)


if __name__ == "__main__":
    unittest.main()
