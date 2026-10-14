#!/usr/bin/env python3
"""Standard closeout preserves checks without per-commit approval recursion."""

import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import standard_release as standard


ROOT = standard.scoped.campaign.ROOT
spec = importlib.util.spec_from_file_location("index_fixtures", ROOT / "scripts/test-scoped-index.py")
fixtures = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fixtures)


class CloseoutTests(unittest.TestCase):
    def setUp(self):
        self.f = fixtures.IndexTests()
        self.addCleanup(self.f.doCleanups)
        self.f.setUp()
        self.f.prepared.update({"approved_policy": {"commit": "d" * 40},
                                "scoped_claim_approval": {"scope": "historical properties"},
                                "packets": {"historical": "original digest"},
                                "conditions_preserved": ["original source and scope"],
                                "assembly_boundary": "bounded review only"})
        self.root, self.evidence = self.f.root, self.f.evidence
        self.impact = {"baseline": {"commit": "d" * 40, "tree": "e" * 40},
                       "paths": ["scripts/check-companion-features.py"]}
        for mock in (patch.object(standard, "assessment", return_value=({}, self.impact)),
                     patch.object(standard.scoped, "prepare_claims", return_value=self.f.prepared)):
            mock.start()
            self.addCleanup(mock.stop)
        self.value = {"schema": 1, "candidate": self.f.candidate, "reviewer": "Maintainer",
                      "security_review": "candidate/security-review.md",
                      "checks": dict.fromkeys(sorted(standard.CHECKS), "passed"),
                      "change_assessment": dict(self.impact, rationale="Tooling only; cold-cache gate passed.")}
        self.f.json(standard.CHECKLIST, self.value)
        self.f.write("candidate/security-review.md", (
            "Status: PASS\nReviewed-Commit: " + self.f.candidate["commit"] +
            "\nReviewed-Tree: " + self.f.candidate["tree"] +
            "\nTester: Reviewer\nScope: Exact candidate tooling changes\n").encode())
        self.env = {standard.WRITE_ENV: self.f.candidate["commit"]}

    def run_mode(self, mode, env=None):
        standard.run(self.root, self.evidence, self.env if env is None else env, mode)

    def test_check_is_read_only_and_write_requires_one_explicit_decision(self):
        self.run_mode("check", {})
        self.assertFalse((self.evidence / standard.INDEX).exists())
        for env in ({}, {standard.WRITE_ENV: "f" * 40}):
            with self.assertRaisesRegex(ValueError, "explicit maintainer"):
                self.run_mode("write", env)
        self.run_mode("write")
        self.run_mode("verify", {})
        result = json.loads((self.evidence / standard.INDEX).read_bytes())
        self.assertEqual(result["retained_claims"],
                         {key: self.f.prepared[key] for key in standard.CLAIM_FIELDS})
        self.assertNotIn("pending", result["retained_claims"])
        self.assertIs(result["release_authorized"], False)
        self.assertFalse((self.evidence / "FINAL-MANIFEST.txt.sig").exists())
        with self.assertRaisesRegex(ValueError, "refusing to replace"):
            self.run_mode("write")

    def test_template_is_pending_and_does_not_overwrite_review(self):
        with self.assertRaises(FileExistsError):
            self.run_mode("template")
        (self.evidence / standard.CHECKLIST).unlink()
        self.run_mode("template")
        value = json.loads((self.evidence / standard.CHECKLIST).read_bytes())
        self.assertEqual(set(value["checks"].values()), {"pending"})
        with self.assertRaises(ValueError):
            self.run_mode("check")

    def test_schema_identity_and_complete_assessment_are_required(self):
        for field, wrong in (("schema", True), ("schema", 1.0), ("schema", "1"),
                             ("candidate", {"commit": "f" * 40}), ("reviewer", " "),
                             ("checks", {}), ("security_review", "../review.md"),
                             ("change_assessment", dict(self.impact, paths=[], rationale="x")),
                             ("change_assessment", dict(self.impact, rationale=""))):
            with self.subTest(field=field, value=wrong):
                self.f.json(standard.CHECKLIST, dict(self.value, **{field: wrong}))
                with self.assertRaises((ValueError, KeyError, TypeError)):
                    self.run_mode("check")
        self.f.json(standard.CHECKLIST, self.value)

    def test_exact_external_review_is_required_not_a_report_only_commit(self):
        path = self.evidence / "candidate/security-review.md"
        original = path.read_bytes()
        for changed in (original.replace(b"PASS", b"PENDING"),
                        original.replace(b"a" * 40, b"f" * 40),
                        original.replace(b"b" * 40, b"f" * 40),
                        original + b"Status: PASS\n", original.replace(b"Tester: Reviewer", b"Tester: ")):
            path.write_bytes(changed)
            with self.assertRaises(ValueError):
                self.run_mode("check")
        path.write_bytes(original)
        self.run_mode("check")

    def test_failed_skipped_incomplete_and_wrong_source_ci_are_rejected(self):
        path = self.evidence / standard.metadata.CI
        original = json.loads(path.read_bytes())
        for conclusion in ("failure", "cancelled", "skipped", ""):
            value = json.loads(json.dumps(original))
            value["jobs"][0]["conclusion"] = conclusion
            self.f.json(standard.metadata.CI, value)
            with self.assertRaisesRegex(ValueError, "full-release CI"):
                self.run_mode("check")
        for field, wrong in (("headSha", "f" * 40), ("event", "push"),
                             ("jobs", original["jobs"][:-1])):
            self.f.json(standard.metadata.CI, dict(original, **{field: wrong}))
            with self.assertRaisesRegex(ValueError, "full-release CI"):
                self.run_mode("check")

    def test_bad_packages_sbom_provenance_and_reproducibility_stay_blocking(self):
        for name in (standard.metadata.RUST, standard.metadata.NPM, "sbom-MANIFEST.txt",
                     "candidate/npm/PROVENANCE.json", "reproducible/release-b.rlib"):
            path = self.evidence / name
            original = path.read_bytes()
            path.write_bytes(b"{}")
            with self.assertRaises((ValueError, KeyError, TypeError)):
                self.run_mode("check")
            path.write_bytes(original)

    def test_claim_failure_cannot_be_overridden_by_checklist(self):
        with patch.object(standard.scoped, "prepare_claims", side_effect=ValueError("bad packet")):
            with self.assertRaisesRegex(ValueError, "bad packet"):
                self.run_mode("write")
        self.assertFalse((self.evidence / standard.INDEX).exists())

    def test_altered_inventory_and_links_are_rejected(self):
        self.run_mode("write")
        self.f.write("extra.log", b"extra")
        with self.assertRaisesRegex(ValueError, "stale or altered"):
            self.run_mode("verify")
        (self.evidence / "extra.log").unlink()
        for create in (lambda p: p.symlink_to("candidate/ci.json"),
                       lambda p: p.hardlink_to(self.evidence / "candidate/ci.json"),
                       lambda p: os.mkfifo(p)):
            path = self.evidence / "hostile"
            create(path)
            with self.assertRaisesRegex(ValueError, "linked or special"):
                self.run_mode("verify")
            path.unlink()

    def test_legacy_approval_variables_are_not_silently_reused(self):
        for name in standard.LEGACY_ENV:
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, "cannot mix"):
                self.run_mode("check", {name: "set"})

    def test_legacy_indexes_and_finalized_collections_cannot_be_mixed(self):
        for name in (standard.index.INDEX, "EQUIVALENCE-MANIFEST.txt"):
            self.f.write(name, b"legacy")
            with self.assertRaisesRegex(ValueError, "cannot mix"):
                self.run_mode("write")
            (self.evidence / name).unlink()
        self.run_mode("write")
        with self.assertRaisesRegex(ValueError, "finalized collection"):
            self.run_mode("template")

    def test_candidate_movement_before_publication_is_rejected(self):
        original = standard.candidate
        calls = 0
        def moving(root):
            nonlocal calls
            calls += 1
            if calls >= 4:
                return {"commit": "f" * 40, "tree": "f" * 40}
            return original(root)
        with patch.object(standard, "candidate", side_effect=moving):
            with self.assertRaisesRegex(ValueError, "source changed"):
                self.run_mode("write")
        self.assertFalse((self.evidence / standard.INDEX).exists())

    def test_partial_publication_rolls_back_without_overwriting(self):
        original = standard.index.publish_new
        def publish(path, raw):
            if path.name == "FINAL-MANIFEST.txt":
                raise OSError("simulated disk failure")
            original(path, raw)
        with patch.object(standard.index, "publish_new", side_effect=publish):
            with self.assertRaises(OSError):
                self.run_mode("write")
        self.assertFalse((self.evidence / standard.INDEX).exists())


class SourceTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory(prefix="base64-standard-source-")
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        self.git("init", "-q")
        self.git("config", "user.name", "Test")
        self.git("config", "user.email", "test@example.invalid")
        self.write("src/lib.rs", "runtime")
        self.base = self.commit()
        base_tree = self.git("rev-parse", "HEAD^{tree}")
        self.policy = {"sources": {"recapture": {"commit": self.base, "tree": base_tree}}}
        self.write(standard.scoped.campaign.POLICY_PATH, json.dumps(self.policy))
        self.write(standard.scoped.campaign.ATTESTATIONS_PATH, "{}")
        policy_commit = self.commit()
        mock = patch.object(standard.scoped, "POLICY_COMMIT", policy_commit)
        mock.start()
        self.addCleanup(mock.stop)

    def git(self, *args):
        result = subprocess.run(["git", *args], cwd=self.root, check=True,
                                capture_output=True, text=True)
        return result.stdout.strip()

    def write(self, path, content):
        file = self.root / path
        file.parent.mkdir(parents=True, exist_ok=True)
        file.write_text(content)

    def commit(self):
        self.git("add", ".")
        self.git("-c", "commit.gpgsign=false", "commit", "-qm", "fixture")
        return self.git("rev-parse", "HEAD")

    def assess(self):
        return standard.assessment(self.root, standard.candidate(self.root))

    def test_documentation_and_tooling_do_not_require_report_only_children(self):
        self.write("docs/process.md", "process")
        self.write("README.md", "documentation; package metadata must be refreshed")
        self.commit()
        self.write("scripts/test-gate.py", "test")
        self.commit()
        _, result = self.assess()
        self.assertIn("scripts/test-gate.py", result["paths"])
        self.assertIn("docs/process.md", result["paths"])

    def test_runtime_dependencies_build_workflow_and_unknown_inputs_fail_closed(self):
        for path in ("src/lib.rs", "Cargo.lock", "rust-toolchain.toml", "build.rs",
                     ".cargo/config.toml", ".github/workflows/ci.yml", "fuzz/src/lib.rs",
                     "packages/base64-ng-wasm-loader/src/index.js", "new-input"):
            with self.subTest(path=path):
                self.write(path, "changed")
                self.commit()
                with self.assertRaisesRegex(ValueError, "runtime/build/package"):
                    self.assess()
                self.git("revert", "--no-edit", "--no-gpg-sign", "HEAD")

    def test_modified_policy_and_dirty_sources_fail_closed(self):
        self.write(standard.scoped.campaign.POLICY_PATH, "{}")
        with self.assertRaisesRegex(ValueError, "dirty"):
            self.assess()
        self.commit()
        with self.assertRaisesRegex(ValueError, "retained campaign policy"):
            self.assess()


if __name__ == "__main__":
    unittest.main()
