#!/usr/bin/env python3
"""Unsigned scoped activation requires independent anchors and complete evidence."""

import importlib.util
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import scoped_candidate as metadata
import scoped_finalizer as scoped
import scoped_index as index


ROOT = scoped.campaign.ROOT
spec = importlib.util.spec_from_file_location("scoped_history_tests", ROOT / "scripts/test-scoped-finalizer.py")
history = importlib.util.module_from_spec(spec)
spec.loader.exec_module(history)


class AnchorTests(history.CandidateTests):
    def test_separate_review_must_name_exact_integration(self):
        self.assertEqual(scoped.verify_candidate(self.root, self.policy, self.candidate),
                         self.identity(self.candidate))
        with self.assertRaisesRegex(ValueError, "neither reviewed"):
            scoped.verify_candidate(self.root, self.policy, self.anchor)

    def test_report_only_child_is_explicit_and_cannot_change_other_files(self):
        self.write("security/pentest/v2.1.0.md", "report")
        child = self.commit("report only")
        self.assertEqual(scoped.verify_candidate(self.root, self.policy, self.candidate),
                         self.identity(child))
        with self.assertRaisesRegex(ValueError, "protected path"):
            scoped.verify_candidate(self.root, self.policy)
        self.write("docs/RELEASE_EVIDENCE.md", "other edit")
        self.commit("not report only")
        with self.assertRaisesRegex(ValueError, "neither reviewed"):
            scoped.verify_candidate(self.root, self.policy, self.candidate)

    def test_report_only_child_cannot_be_a_link_to_untracked_bytes(self):
        outside = self.root / ".git/untracked-report"
        outside.write_text("unsigned mutable report")
        report = self.root / "security/pentest/v2.1.0.md"
        report.parent.mkdir(parents=True, exist_ok=True)
        report.symlink_to(outside)
        self.commit("linked report")
        with self.assertRaises((ValueError, OSError)):
            scoped.verify_candidate(self.root, self.policy, self.candidate)


class IndexTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="base64-scoped-index-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.evidence = self.root / "target/release-evidence"
        self.evidence.mkdir(parents=True, mode=0o700)
        self.candidate = {"commit": "a" * 40, "tree": "b" * 40}
        self.env = {index.INTEGRATION_ENV: "c" * 40, index.WRITE_ENV: self.candidate["commit"]}
        self.write("scoped/historical.tar.gz", b"historical fixture")
        self.write("scoped/recapture.tar.gz", b"recapture fixture")
        jobs = [{"name": name, "status": "completed", "conclusion": "success"} for name in (
            "core", "2.1 feature gates", "Big-endian QEMU functional evidence",
            "AArch64 SVE QEMU candidate evidence", "RISC-V QEMU and RVV candidate evidence")]
        self.json("scoped/reviews/github-ci.json", {"jobs": jobs})
        self.json(metadata.CI, {"jobs": jobs, "headSha": self.candidate["commit"],
                              "event": "workflow_dispatch", "conclusion": "success",
                              "workflowName": "Rust CI",
                              "url": "https://github.com/valkyoth/base64-ng/actions/runs/123"})
        self.json(metadata.SBOMS[0], {"spdxVersion": "SPDX-2.3"})
        self.json(metadata.SBOMS[1], {"bomFormat": "CycloneDX", "specVersion": "1.4"})
        for pair in metadata.REPRO_PAIRS:
            for name in pair:
                self.write("reproducible/" + name, pair[0].encode())
        self.source_manifest("sbom-MANIFEST.txt", metadata.SBOMS)
        self.source_manifest("reproducible/MANIFEST.txt",
                             ["reproducible/" + name for pair in metadata.REPRO_PAIRS for name in pair],
                             "comparisons=release-rlib,package-file-list,crate-archive\n")
        (self.root / "release-crates.toml").write_text(
            '[release]\nversion="2.1.0"\npolicy="synced-family"\n'
            '[npm]\nname="@valkyoth/base64-ng-wasm-loader"\n'
            '[crates.base64-ng]\nversion="2.1.0"\n')
        self.report(metadata.RUST, packages={"base64-ng": {
            "version": "2.1.0", "archive": "candidate/rust/base64-ng-2.1.0.crate",
            "inventory": "candidate/rust/base64-ng-files.txt"}})
        self.write("candidate/rust/base64-ng-2.1.0.crate", b"crate fixture")
        self.write("candidate/rust/base64-ng-files.txt", b"Cargo.toml\n")
        self.report(metadata.NPM, package="@valkyoth/base64-ng-wasm-loader", version="2.1.0")
        for name in ("package.tgz", "files.txt"):
            self.write("candidate/npm/" + name, b"fixture")
        artifacts = {}
        for name in ("base64-ng-scalar.wasm", "base64-ng-simd128.wasm"):
            raw = name.encode()
            self.write("candidate/npm/" + name, raw)
            original = self.root / "packages/base64-ng-wasm-loader/artifacts" / name
            original.parent.mkdir(parents=True, exist_ok=True)
            original.write_bytes(raw)
            artifacts[name] = scoped.record(raw)["sha256"]
        self.json("candidate/npm/PROVENANCE.json", {
            "schema": "base64-ng-wasm-provenance-v1", "package": "@valkyoth/base64-ng-wasm-loader",
            "version": "2.1.0", "sourceCommit": self.candidate["commit"], "artifacts": artifacts})
        # These stubs isolate final indexing. Signed-history and real transport
        # verification are exercised by the scoped preparation suites.
        self.prepared = {"candidate": self.candidate, "claims": [{"source": "original"}],
                         "release_authorized": False}
        self.mocks = [patch.object(scoped, "clean_head", return_value=self.candidate["commit"]),
                      patch.object(scoped.campaign, "git", return_value=(self.candidate["tree"] + "\n").encode()),
                      patch.object(scoped, "prepare", return_value=self.prepared)]
        for mock in self.mocks:
            mock.start()
            self.addCleanup(mock.stop)
        self.approve()

    def write(self, name, raw):
        path = self.evidence / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(raw)

    def json(self, name, value):
        self.write(name, (json.dumps(value, indent=2) + "\n").encode())

    def report(self, name, **fields):
        self.json(name, dict(schema=1, source=self.candidate, status="passed", **fields))

    def source_manifest(self, name, artifacts, extra=""):
        text = "source:\ncommit=" + self.candidate["commit"] + "\ntree_state=clean\n" + extra
        for path in artifacts:
            text += scoped.record((self.evidence / path).read_bytes())["sha256"] + "  " + metadata.PREFIX + path + "\n"
        self.write(name, text.encode())

    def approve(self):
        value = {"schema": 1, "candidate": self.candidate,
                 "integration_commit": self.env[index.INTEGRATION_ENV],
                 "status": "approved-for-unsigned-index", "release_authorized": False,
                 "checks": dict.fromkeys(sorted(index.REQUIRED_CHECKS), "passed"),
                 "inventory": index.inventory_tree(self.evidence, index.OUTPUTS | {metadata.REVIEW})}
        self.json(metadata.REVIEW, value)
        self.env[index.RECORD_ENV] = scoped.record((self.evidence / metadata.REVIEW).read_bytes())["sha256"]

    def activate(self, write=True):
        index.activate(self.root, self.evidence, self.env, write=write)

    def mutate_json(self, name, mutate):
        value = json.loads((self.evidence / name).read_bytes())
        mutate(value)
        self.json(name, value)

    def test_complete_activation_and_revalidation_preserve_sources(self):
        self.activate()
        self.activate(write=False)
        value = json.loads((self.evidence / index.INDEX).read_bytes())
        self.assertEqual(value["scoped_review"], self.prepared)
        self.assertIs(value["release_authorized"], False)
        self.assertNotIn("campaign_commit", value)
        self.assertFalse((self.evidence / (index.FINAL + ".sig")).exists())
        self.assertEqual((self.evidence / index.FINAL).stat().st_mode & 0o777, 0o600)
        self.assertEqual((self.evidence / index.FINAL).stat().st_nlink, 1)
        with self.assertRaisesRegex(ValueError, "refusing to replace"):
            self.activate()

    def test_missing_or_wrong_authorization_never_writes(self):
        original = dict(self.env)
        for name in (index.WRITE_ENV, index.RECORD_ENV, index.INTEGRATION_ENV):
            for value in (None, "0" * 40):
                with self.subTest(name=name, value=value):
                    self.env = dict(original)
                    if value is None:
                        self.env.pop(name)
                    else:
                        self.env[name] = value
                    with self.assertRaises(ValueError):
                        self.activate()
                    self.assertFalse((self.evidence / index.FINAL).exists())
                    self.assertFalse((self.evidence / index.INDEX).exists())

    def test_legacy_and_dirty_flags_are_not_waivers(self):
        for name in ("BASE64_NG_REUSE_EVIDENCE_FROM", "BASE64_NG_CAMPAIGN_SOURCE_COMMIT",
                     "BASE64_NG_ALLOW_DIRTY_EVIDENCE"):
            with self.subTest(name=name):
                self.env[name] = "1"
                with self.assertRaisesRegex(ValueError, "combine"):
                    self.activate()
                self.env.pop(name)

    def test_preexisting_final_index_or_signature_is_never_replaced(self):
        for name in index.OUTPUTS:
            path = self.evidence / name
            path.write_bytes(b"sentinel")
            with self.assertRaisesRegex(ValueError, "refusing to replace"):
                self.activate()
            self.assertEqual(path.read_bytes(), b"sentinel")
            path.unlink()
        path = self.evidence / index.FINAL
        path.symlink_to(self.evidence / "nonexistent")
        with self.assertRaisesRegex(ValueError, "refusing to replace"):
            self.activate()

    def test_unsigned_review_mutation_does_not_reapprove_itself(self):
        self.mutate_json(metadata.REVIEW, lambda x: x.update(release_authorized=True))
        with self.assertRaisesRegex(ValueError, "record digest"):
            self.activate()

    def test_extra_empty_directory_missing_or_changed_file(self):
        extra = self.evidence / "extra"
        extra.mkdir()
        with self.assertRaisesRegex(ValueError, "inventory mismatch"):
            self.activate()
        extra.rmdir()
        path = self.evidence / metadata.CI
        raw = path.read_bytes()
        path.unlink()
        with self.assertRaisesRegex(ValueError, "inventory mismatch"):
            self.activate()
        path.write_bytes(raw + b" ")
        with self.assertRaisesRegex(ValueError, "inventory mismatch"):
            self.activate()

    def test_ci_rejects_old_push_skips_missing_duplicate_jobs_and_other_repositories(self):
        path = self.evidence / metadata.CI
        original = path.read_bytes()
        mutations = [lambda x: x.update(event="push"), lambda x: x.update(headSha="f" * 40),
                     lambda x: x.update(workflowName="Other CI"),
                     lambda x: x.update(url="https://github.com/other/repo/actions/runs/123"),
                     lambda x: x["jobs"][1].update(conclusion="skipped"),
                     lambda x: x["jobs"][1].update(status="in_progress"),
                     lambda x: x["jobs"].pop(), lambda x: x["jobs"].append(x["jobs"][0])]
        for mutation in mutations:
            with self.subTest(mutation=mutation):
                path.write_bytes(original)
                self.mutate_json(metadata.CI, mutation)
                self.approve()
                with self.assertRaisesRegex(ValueError, "full-release CI"):
                    self.activate()

    def test_metadata_identity_and_package_inventory(self):
        mutations = [(metadata.RUST, lambda x: x.update(status="failed")),
                     (metadata.RUST, lambda x: x["source"].update(commit="e" * 40)),
                     (metadata.RUST, lambda x: x["packages"].clear()),
                     (metadata.NPM, lambda x: x.update(version="2.0.4")),
                     ("candidate/npm/PROVENANCE.json", lambda x: x.update(sourceCommit="e" * 40)),
                     ("candidate/npm/PROVENANCE.json", lambda x: x["artifacts"].clear())]
        for name, mutation in mutations:
            with self.subTest(name=name):
                raw = (self.evidence / name).read_bytes()
                self.mutate_json(name, mutation)
                self.approve()
                with self.assertRaises(ValueError):
                    self.activate()
                (self.evidence / name).write_bytes(raw)

    def test_source_and_hash_manifest_fields_are_not_just_presence_checks(self):
        path = self.evidence / "sbom-MANIFEST.txt"
        original = path.read_bytes()
        for raw in (original.replace(b"a" * 40, b"e" * 40), original + b"tree_state=clean\n",
                    original.replace(b"  target/", b"f target/")):
            path.write_bytes(raw)
            self.approve()
            with self.assertRaises(ValueError):
                self.activate()

    def test_reproducibility_mismatch_rejected_even_with_new_digests(self):
        self.write("reproducible/release-b.rlib", b"different")
        self.source_manifest("reproducible/MANIFEST.txt",
                             ["reproducible/" + name for pair in metadata.REPRO_PAIRS for name in pair],
                             "comparisons=release-rlib,package-file-list,crate-archive\n")
        self.approve()
        with self.assertRaisesRegex(ValueError, "pair mismatch"):
            self.activate()

    def test_kernel_artifact_change_requires_new_review_not_only_new_provenance(self):
        name = "base64-ng-scalar.wasm"
        self.write("candidate/npm/" + name, b"new kernel")
        self.mutate_json("candidate/npm/PROVENANCE.json", lambda x:
                         x["artifacts"].update({name: scoped.record(b"new kernel")["sha256"]}))
        self.approve()
        with self.assertRaisesRegex(ValueError, "reviewed kernel"):
            self.activate()

    def test_failed_final_publication_removes_only_its_new_index(self):
        original = index.publish_new
        def fail_final(path, raw):
            if path.name == index.FINAL:
                raise OSError("injected publication failure")
            original(path, raw)
        with patch.object(index, "publish_new", side_effect=fail_final):
            with self.assertRaisesRegex(OSError, "injected"):
                self.activate()
        self.assertFalse((self.evidence / index.INDEX).exists())
        self.assertFalse((self.evidence / index.FINAL).exists())
        self.assertTrue((self.evidence / metadata.REVIEW).exists())
        self.assertFalse(list(self.evidence.glob(".scoped-index-*")))

    def test_verify_rejects_post_publication_tampering(self):
        self.activate()
        for name in (index.INDEX, index.FINAL):
            path = self.evidence / name
            raw = path.read_bytes()
            path.write_bytes(raw + b" ")
            with self.assertRaisesRegex(ValueError, "stale or altered"):
                self.activate(write=False)
            path.write_bytes(raw)
        (self.evidence / "extra").mkdir()
        with self.assertRaisesRegex(ValueError, "inventory mismatch"):
            self.activate(write=False)

    def test_preflight_failure_and_mid_scan_mutation_leave_no_output(self):
        with patch.object(scoped, "prepare", side_effect=ValueError("unapproved claims")):
            with self.assertRaisesRegex(ValueError, "unapproved claims"):
                self.activate()
        original = metadata.verify_metadata
        def mutate(*args):
            original(*args)
            self.write("after-validation", b"new")
        with patch.object(metadata, "verify_metadata", side_effect=mutate):
            with self.assertRaisesRegex(ValueError, "changed during"):
                self.activate()
        self.assertFalse((self.evidence / index.FINAL).exists())

    def test_inventory_rejects_links_special_files_and_nonprivate_root(self):
        path = self.evidence / "hostile"
        for kind in ("hardlink", "symlink", "fifo", "socket"):
            with self.subTest(kind=kind):
                sock = None
                if kind == "hardlink":
                    path.hardlink_to(self.evidence / metadata.CI)
                elif kind == "symlink":
                    path.symlink_to(self.evidence / metadata.CI)
                elif kind == "fifo":
                    os.mkfifo(path)
                else:
                    sock = socket.socket(socket.AF_UNIX)
                    sock.bind(str(path))
                try:
                    with self.assertRaisesRegex(ValueError, "linked or special"):
                        index.inventory_tree(self.evidence, set())
                finally:
                    if sock:
                        sock.close()
                    path.unlink()
        self.evidence.chmod(0o755)
        with self.assertRaisesRegex(ValueError, "private"):
            index.inventory_tree(self.evidence, set())

    def test_inventory_limits_and_path_representation(self):
        for key, value in (("MAX_ENTRIES", 1), ("MAX_BYTES", 1)):
            with patch.object(index, key, value):
                with self.assertRaises(ValueError):
                    index.inventory_tree(self.evidence, set())
        self.write("not representable", b"x")
        with self.assertRaisesRegex(ValueError, "representable"):
            index.inventory_tree(self.evidence, set())

    def test_generated_index_size_is_checked_before_writing(self):
        with patch.object(index, "MAX_MANIFEST", 1):
            with self.assertRaisesRegex(ValueError, "generated index"):
                self.activate()
        self.assertFalse((self.evidence / index.INDEX).exists())
        self.assertFalse((self.evidence / index.FINAL).exists())

    def test_atomic_writer_never_truncates_existing_file(self):
        path = self.evidence / "existing"
        path.write_bytes(b"sentinel")
        with self.assertRaises(FileExistsError):
            index.publish_new(path, b"replacement")
        self.assertEqual(path.read_bytes(), b"sentinel")
        self.assertFalse(list(self.evidence.glob(".scoped-index-*")))

    def test_temporary_cleanup_failure_does_not_leave_a_published_link(self):
        original = os.unlink
        failed = False
        def fail_once(path, *args, **kwargs):
            nonlocal failed
            if not failed and Path(path).name.startswith(".scoped-index-"):
                failed = True
                raise OSError("injected cleanup failure")
            return original(path, *args, **kwargs)
        path = self.evidence / "new-index"
        with patch.object(index.os, "unlink", side_effect=fail_once):
            with self.assertRaisesRegex(OSError, "cleanup failure"):
                index.publish_new(path, b"fixture")
        self.assertFalse(path.exists())


class DispatchTests(unittest.TestCase):
    def test_readiness_runs_scoped_verifier_after_signature_and_propagates_failure(self):
        with tempfile.TemporaryDirectory(prefix="base64-scoped-readiness-") as directory:
            root = Path(directory)
            files = {
                "release-crates.toml": 'policy = "synced-family"\n',
                "release-notes/RELEASE_NOTES_2.1.0.md": "fixture\n",
                "security/pentest/v2.1.0.md": "Status: PASS\nReviewed-Commit: " + "0" * 40
                    + "\nTester: fixture\nScope: fixture\nDate: 2026-10-13\n",
                "target/release-evidence/base64-ng.spdx.json": "{}",
                "target/release-evidence/base64-ng.cyclonedx.json": "{}",
                "scripts/verify-release-evidence-signature.sh": "#!/bin/sh\necho signature-stage\n",
                "scripts/scoped_index.py": "import sys\nprint('scoped-stage', sys.argv[1:])\nsys.exit(17)\n",
            }
            for name, content in files.items():
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(content)
            signature = root / "scripts/verify-release-evidence-signature.sh"
            signature.chmod(0o700)
            scoped.campaign.git(root, "init", "-q")
            scoped.campaign.git(root, "add", ".")
            scoped.campaign.git(root, "-c", "user.name=fixture", "-c", "user.email=fixture@example.invalid",
                                "-c", "commit.gpgsign=false", "commit", "-qm", "fixture")
            commit = scoped.campaign.git(root, "rev-parse", "HEAD").decode().strip()
            (root / "target/release-evidence/FINAL-MANIFEST.txt").write_text(
                f"commit={commit}\ntree_state=clean\nevidence_mode=scoped-claims-v1\nrelease_commit={commit}\n")
            command = ["sh", str(ROOT / "scripts/validate-release-readiness.sh"), "v2.1.0"]
            result = subprocess.run(command, cwd=root, capture_output=True, timeout=10)
            self.assertEqual(result.returncode, 17, result.stderr)
            self.assertEqual(result.stdout, b"signature-stage\nscoped-stage ['--verify']\n")
            signature.write_text("#!/bin/sh\nexit 19\n")
            result = subprocess.run(command, cwd=root, capture_output=True, timeout=10)
            self.assertEqual(result.returncode, 19)
            self.assertNotIn(b"scoped-stage", result.stdout)

    def test_finalizer_activation_requires_explicit_write_and_no_extra_arguments(self):
        with tempfile.TemporaryDirectory(prefix="base64-scoped-activation-") as directory:
            root = Path(directory)
            (root / "scripts").mkdir()
            (root / "scripts/scoped_index.py").write_text(
                "import sys\nprint(repr(sys.argv[1:]))\n")
            result = subprocess.run(["sh", str(ROOT / "scripts/finalize-release-evidence.sh"),
                                     "--scoped-finalize"], cwd=root, capture_output=True, timeout=10)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stdout, b"['--write']\n")
            self.assertFalse((root / "target").exists())
        result = subprocess.run(["python3", str(ROOT / "scripts/scoped_index.py"), "--write", "--verify"],
                                capture_output=True, timeout=10)
        self.assertNotEqual(result.returncode, 0)


if __name__ == "__main__":
    unittest.main()
