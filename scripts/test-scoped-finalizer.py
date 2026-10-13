#!/usr/bin/env python3
"""Claim-aware preparation cannot authorize or mutate final release evidence."""

import copy
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import scoped_campaign as campaign
import scoped_finalizer as finalizer


ROOT = campaign.ROOT
spec = importlib.util.spec_from_file_location("history_fixtures", ROOT / "scripts/test-scoped-campaign.py")
history = importlib.util.module_from_spec(spec)
spec.loader.exec_module(history)


class CandidateTests(unittest.TestCase):
    git = history.CampaignTests.git
    write = history.CampaignTests.write
    commit = history.CampaignTests.commit
    identity = history.CampaignTests.identity
    record = history.CampaignTests.record

    def setUp(self):
        history.CampaignTests.setUp(self)
        self.addCleanup(patch.stopall)
        patch.object(finalizer, "POLICY_COMMIT", self.anchor).start()
        patch.object(finalizer, "POLICY_TREE", self.identity(self.anchor)["tree"]).start()
        for name in finalizer.INTEGRATION_FILES:
            self.write(name, (ROOT / name).read_text())
        self.candidate = self.commit("integration awaiting separate review")

    def check(self):
        return finalizer.verify_candidate(self.root, self.policy)

    def test_exact_signed_integration(self):
        self.assertEqual(self.check(), self.identity(self.candidate))

    def test_unsigned_integration(self):
        self.git("-c", "commit.gpgsign=false", "commit", "--amend", "--no-edit")
        with self.assertRaisesRegex(ValueError, "git check failed"):
            self.check()

    def test_uncommitted_integration(self):
        self.write("scripts/scoped_finalizer.py", "replacement")
        with self.assertRaisesRegex(ValueError, "dirty"):
            self.check()

    def test_original_policy_is_not_the_integration(self):
        self.git("checkout", "--detach", self.anchor)
        with self.assertRaisesRegex(ValueError, "missing committed"):
            self.check()

    def test_wrong_tree_anchor(self):
        with patch.object(finalizer, "POLICY_TREE", "0" * 40):
            with self.assertRaisesRegex(ValueError, "tree mismatch"):
                self.check()

    def test_policy_parser_drift_even_when_signed(self):
        self.write("scripts/scoped_evidence.py", "replacement")
        self.commit("drift")
        with self.assertRaisesRegex(ValueError, "implementation drift"):
            self.check()

    def test_runtime_change_and_revert_cannot_disappear(self):
        original = (self.root / "src/lib.rs").read_text()
        self.write("src/lib.rs", "unreviewed runtime")
        self.commit("runtime change")
        self.write("src/lib.rs", original)
        self.commit("revert runtime")
        with self.assertRaisesRegex(ValueError, "protected path"):
            self.check()

    def test_unknown_path_and_package_changes(self):
        self.write("Cargo.toml", "unreviewed dependency")
        self.commit("manifest change")
        with self.assertRaisesRegex(ValueError, "protected path"):
            self.check()

    def test_hidden_index_flags(self):
        for flag, reset in (("--assume-unchanged", "--no-assume-unchanged"),
                            ("--skip-worktree", "--no-skip-worktree")):
            with self.subTest(flag=flag):
                self.git("update-index", flag, "src/lib.rs")
                with self.assertRaisesRegex(ValueError, "hidden"):
                    self.check()
                self.git("update-index", reset, "src/lib.rs")

    def test_unsigned_policy_anchor(self):
        self.git("checkout", "--detach", self.anchor)
        self.git("-c", "commit.gpgsign=false", "commit", "--amend", "--no-edit")
        unsigned = self.git("rev-parse", "HEAD")
        self.git("cherry-pick", self.candidate)
        with patch.object(finalizer, "POLICY_COMMIT", unsigned):
            with self.assertRaisesRegex(ValueError, "git check failed"):
                self.check()

    def test_merge_is_not_linear(self):
        self.git("checkout", "-b", "side", self.candidate)
        self.write("docs/RELEASE_EVIDENCE.md", "side documentation")
        self.commit("side")
        self.git("checkout", "--detach", self.candidate)
        self.write("docs/RELEASE_FREEZE_2.1.md", "main documentation")
        self.commit("main")
        self.git("merge", "--no-ff", "-m", "merge", "side")
        with self.assertRaisesRegex(ValueError, "not linear"):
            self.check()


class ReviewTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="base64-review-records-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.policy = campaign.load_policy(ROOT)
        self.handoff = {
            "policy_commit": finalizer.POLICY_COMMIT,
            "policy_tree": finalizer.POLICY_TREE,
            "release_authorized": False,
            "finalizer_integration_approved": False,
            "selected_records": {},
            "claim_dispositions": {name: "scoped fixture" for name in finalizer.CLAIM_REVIEWS},
            "conditions_preserved": ["bounded fixture only"],
        }
        for name in finalizer.REVIEW_RECORDS:
            raw = json.dumps({"fixture": name}).encode()
            (self.root / name).write_bytes(raw)
            self.handoff["selected_records"][name] = finalizer.record(raw)
        self.pin_handoff()

    def pin_handoff(self):
        raw = json.dumps(self.handoff).encode()
        (self.root / "review-handoff.json").write_bytes(raw)
        pin = patch.object(finalizer, "HANDOFF_SHA256", finalizer.record(raw)["sha256"])
        pin.start()
        self.addCleanup(pin.stop)

    def check(self):
        return finalizer.verify_reviews(self.root, self.policy)

    def test_exact_seven_records(self):
        self.assertEqual(self.check(), self.handoff)

    def test_changed_handoff_not_accepted_as_operator_approval(self):
        self.handoff["finalizer_integration_approved"] = True
        (self.root / "review-handoff.json").write_text(json.dumps(self.handoff))
        with self.assertRaisesRegex(ValueError, "handoff digest"):
            self.check()

    def test_each_record_missing_or_altered(self):
        for name in finalizer.REVIEW_RECORDS:
            with self.subTest(name=name):
                path = self.root / name
                raw = path.read_bytes()
                path.unlink()
                with self.assertRaises(FileNotFoundError):
                    self.check()
                path.write_bytes(raw + b" ")
                with self.assertRaisesRegex(ValueError, "record digest"):
                    self.check()
                path.write_bytes(raw)

    def test_symlink_hardlink_fifo_and_linked_ancestor(self):
        name = "assembly-review.json"
        path = self.root / name
        raw = path.read_bytes()
        for kind in ("symlink", "hardlink", "fifo"):
            with self.subTest(kind=kind):
                path.unlink()
                other = self.root / "other"
                other.write_bytes(raw)
                if kind == "symlink":
                    path.symlink_to(other)
                elif kind == "hardlink":
                    path.hardlink_to(other)
                else:
                    os.mkfifo(path)
                with self.assertRaises((ValueError, OSError)):
                    self.check()
                path.unlink()
                other.unlink()
                path.write_bytes(raw)
        link = self.root / "linked"
        link.symlink_to(self.root, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, "ancestor"):
            finalizer.verify_reviews(link, self.policy)

    def test_approval_never_covers_finalization(self):
        for field in ("release_authorized", "finalizer_integration_approved"):
            with self.subTest(field=field):
                self.handoff[field] = True
                self.pin_handoff()
                with self.assertRaisesRegex(ValueError, "authority mismatch"):
                    self.check()
                self.handoff[field] = False

    def test_record_and_claim_inventory(self):
        for collection in ("selected_records", "claim_dispositions"):
            original = copy.deepcopy(self.handoff[collection])
            self.handoff[collection].pop(next(iter(original)))
            self.pin_handoff()
            with self.assertRaisesRegex(ValueError, "required"):
                self.check()
            self.handoff[collection] = original

    def test_assembly_requires_both_followups(self):
        self.assertEqual(finalizer.CLAIM_REVIEWS["assembly"], [
            "recapture-review.json", "mechanical-assembly-review.json", "assembly-review.json"])

    def test_preparation_preserves_claims_and_all_limits_without_authorizing(self):
        candidate = {"commit": "c" * 40, "tree": "d" * 40}
        summaries = campaign.load_attestations(ROOT, self.policy)
        paths = {"historical": "fixture-old", "recapture": "fixture-new"}
        # Transport/attestation validation has separate mutation suites and a
        # real-packet run. These stubs isolate index construction and routing.
        with patch.object(finalizer, "verify_candidate", return_value=candidate), \
                patch.object(finalizer, "verify_packet", return_value={}) as packet, \
                patch.object(campaign, "verify_claims") as claims, \
                patch.object(finalizer, "clean_head", return_value=candidate["commit"]):
            result = finalizer.prepare(ROOT, self.root, paths)
        self.assertEqual(packet.call_count, 2)
        claims.assert_called_once_with(self.policy, {"historical": {}, "recapture": {}}, summaries)
        self.assertEqual(result["evidence_mode"], "scoped-review-only")
        for name in ("release_authorized", "finalization_permitted", "candidate_validated",
                     "finalizer_integration_approved"):
            self.assertIs(result[name], False)
        self.assertEqual(result["candidate"], candidate)
        self.assertEqual(result["conditions_preserved"], self.handoff["conditions_preserved"])
        self.assertIn("exact-candidate-full-release-ci", result["pending"])
        self.assertIn("separate-integration-review", result["pending"])
        self.assertNotIn("campaign_commit", result)
        for original, prepared in zip(self.policy["claims"], result["claims"], strict=True):
            self.assertEqual(prepared["claim"], original)
            self.assertEqual(prepared["original_source"], self.policy["sources"][original["source"]])
            self.assertEqual(prepared["attestation"], summaries["attestations"][original["id"]])

    def test_packet_failure_and_source_change_cannot_emit_index(self):
        candidate = {"commit": "c" * 40, "tree": "d" * 40}
        paths = {"historical": "old", "recapture": "new"}
        with patch.object(finalizer, "verify_candidate", return_value=candidate):
            with patch.object(finalizer, "verify_packet", side_effect=ValueError("corrupt packet")):
                with self.assertRaisesRegex(ValueError, "corrupt packet"):
                    finalizer.prepare(ROOT, self.root, paths)
            with patch.object(finalizer, "verify_packet", return_value={}), \
                    patch.object(campaign, "verify_claims"), \
                    patch.object(finalizer, "clean_head", return_value="e" * 40):
                with self.assertRaisesRegex(ValueError, "candidate changed"):
                    finalizer.prepare(ROOT, self.root, paths)


class DispatchTests(unittest.TestCase):
    def test_readiness_rejects_scoped_mode_even_after_signature_stage(self):
        with tempfile.TemporaryDirectory(prefix="base64-scoped-readiness-") as directory:
            root = Path(directory)
            files = {
                "release-crates.toml": 'policy = "synced-family"\n',
                "release-notes/RELEASE_NOTES_2.1.0.md": "fixture\n",
                "security/pentest/v2.1.0.md": "Status: PASS\nReviewed-Commit: " + "0" * 40
                    + "\nTester: fixture\nScope: fixture\nDate: 2026-10-13\n",
                "target/release-evidence/base64-ng.spdx.json": "{}",
                "target/release-evidence/base64-ng.cyclonedx.json": "{}",
                # Isolate mode acceptance after the signature stage. No real
                # evidence is signed and the production readiness code runs.
                "scripts/verify-release-evidence-signature.sh": "#!/bin/sh\nexit 0\n",
            }
            for name, content in files.items():
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(content)
            (root / "scripts/verify-release-evidence-signature.sh").chmod(0o700)
            campaign.git(root, "init", "-q")
            campaign.git(root, "add", ".")
            campaign.git(root, "-c", "user.name=fixture", "-c", "user.email=fixture@example.invalid",
                         "-c", "commit.gpgsign=false", "commit", "-qm", "fixture")
            commit = campaign.git(root, "rev-parse", "HEAD").decode().strip()
            (root / "target/release-evidence/FINAL-MANIFEST.txt").write_text(
                f"commit={commit}\ntree_state=clean\nevidence_mode=scoped-review-only\n"
                f"campaign_commit={commit}\nrelease_commit={commit}\n")
            result = subprocess.run(["sh", str(ROOT / "scripts/validate-release-readiness.sh"),
                                     "v2.1.0"], cwd=root, capture_output=True, timeout=10)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn(b"unsupported release evidence mode: scoped-review-only", result.stderr)

    def test_scoped_failure_and_invalid_options_never_touch_existing_index(self):
        with tempfile.TemporaryDirectory(prefix="base64-finalizer-dispatch-") as directory:
            root = Path(directory)
            evidence = root / "target/release-evidence"
            evidence.mkdir(parents=True)
            for name in ("FINAL-MANIFEST.txt", "FINAL-MANIFEST.txt.sig", "EQUIVALENCE-MANIFEST.txt"):
                (evidence / name).write_bytes(b"unchanged sentinel")
            (root / "scripts").mkdir()
            for name in ("scoped_finalizer.py", "scoped_campaign.py", "scoped_evidence.py"):
                (root / "scripts" / name).write_bytes((ROOT / "scripts" / name).read_bytes())
            for args in (["--scoped-review"], ["--unknown"], [""],
                         ["--scoped-review", "--finalize"],
                         ["--scoped-review", "--reviews", "missing", "--historical-packet", "old",
                          "--recapture-packet", "new"]):
                with self.subTest(args=args):
                    result = subprocess.run(["sh", str(ROOT / "scripts/finalize-release-evidence.sh"),
                                             *args], cwd=root, capture_output=True, timeout=10)
                    self.assertNotEqual(result.returncode, 0)
                    self.assertEqual(result.stdout, b"")
                    self.assertEqual(len(list(evidence.iterdir())), 3)
                    self.assertTrue(all(p.read_bytes() == b"unchanged sentinel" for p in evidence.iterdir()))

    def test_successful_scoped_dispatch_does_not_fall_through(self):
        with tempfile.TemporaryDirectory(prefix="base64-finalizer-dispatch-") as directory:
            root = Path(directory)
            (root / "scripts").mkdir()
            (root / "scripts/scoped_finalizer.py").write_text("print('review-only fixture')\n")
            result = subprocess.run(["sh", str(ROOT / "scripts/finalize-release-evidence.sh"),
                                     "--scoped-review"], cwd=root, capture_output=True, timeout=10)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stdout, b"review-only fixture\n")
            self.assertFalse((root / "target").exists())


if __name__ == "__main__":
    unittest.main()
