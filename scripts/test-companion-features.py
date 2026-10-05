#!/usr/bin/env python3
"""Negative tests for the dependency forwarding contract."""

import copy
import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("companion_features", Path(__file__).with_name("check-companion-features.py"))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class FeatureTests(unittest.TestCase):
    def test_actual_contract(self):
        module.audit()
        self.assertEqual(module.expected_features("serde", [], False), set())
        self.assertEqual(module.expected_features("bytes", ["simd"], False), {"alloc", "simd"})
        self.assertEqual(module.expected_features("multibase", ["checked-backend"], False), {"simd", "checked-backend"})
        self.assertEqual(module.expected_features("tokio", ["checked-backend"], False), {"std", "alloc", "simd", "checked-backend"})

    def test_missing_forwarding_and_changed_defaults_are_rejected(self):
        original = module.manifest
        for key, value in (("simd", []), ("simd", ["base64-ng/std"]),
                           ("checked-backend", ["simd"]), ("default", ["std", "simd"])):
            def changed(name):
                data = copy.deepcopy(original(name))
                if name == "bytes":
                    data["features"][key] = value
                return data
            with self.subTest(key=key, value=value), patch.object(module, "manifest", changed):
                with self.assertRaises(AssertionError):
                    module.audit()

    def test_implicit_core_defaults_and_secret_only_forwarding_are_rejected(self):
        original = module.manifest
        for mutation in ("defaults", "implicit-std", "secret-forwarding"):
            def changed(name):
                data = copy.deepcopy(original(name))
                if name == "serde" and mutation == "defaults":
                    data["dependencies"]["base64-ng"]["default-features"] = True
                if name == "serde" and mutation == "implicit-std":
                    data["dependencies"]["base64-ng"]["features"] = ["std"]
                if name == "subtle" and mutation == "secret-forwarding":
                    data["features"]["simd"] = ["base64-ng/simd"]
                return data
            with self.subTest(mutation=mutation), patch.object(module, "manifest", changed):
                with self.assertRaises(AssertionError):
                    module.audit()


if __name__ == "__main__":
    unittest.main()
