import importlib.util
import unittest
from pathlib import Path


POLICY_PATH = Path(__file__).with_name("check-rust-policy.py")
SPEC = importlib.util.spec_from_file_location("rust_policy", POLICY_PATH)
assert SPEC is not None and SPEC.loader is not None
POLICY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(POLICY)


class ExpectInventoryTests(unittest.TestCase):
    def test_matching_path_and_attribute_pass(self) -> None:
        attribute = '#[expect(clippy::unwrap_used, reason = "fixed fixture") ]'
        observed = {("crates/example/src/lib.rs", attribute)}
        inventory = [{"path": "crates/example/src/lib.rs", "attribute": attribute}]

        self.assertEqual(POLICY._expect_inventory_errors(observed, inventory, "inventory.json"), [])

    def test_unregistered_expectation_fails(self) -> None:
        attribute = '#[expect(clippy::unwrap_used, reason = "fixed fixture") ]'
        observed = {("crates/example/src/lib.rs", attribute)}

        errors = POLICY._expect_inventory_errors(observed, [], "inventory.json")

        self.assertEqual(len(errors), 1)
        self.assertIn("crates/example/src/lib.rs", errors[0])
        self.assertIn("missing a path-specific entry", errors[0])

    def test_stale_path_or_changed_reason_fails(self) -> None:
        previous = '#[expect(clippy::unwrap_used, reason = "old reason") ]'
        current = '#[expect(clippy::unwrap_used, reason = "new reason") ]'
        observed = {("crates/example/src/lib.rs", current)}
        inventory = [{"path": "crates/example/src/lib.rs", "attribute": previous}]

        errors = POLICY._expect_inventory_errors(observed, inventory, "inventory.json")

        self.assertEqual(len(errors), 2)
        self.assertTrue(any("missing a path-specific entry" in error for error in errors))
        self.assertTrue(any("stale #[expect] inventory entry" in error for error in errors))


class LintDowngradeTests(unittest.TestCase):
    def test_direct_allow_and_warn_are_downgrades(self) -> None:
        self.assertTrue(POLICY._has_lint_downgrade_attribute("#![allow(unsafe_code)]"))
        self.assertTrue(POLICY._has_lint_downgrade_attribute("#[warn(clippy::all)]"))

    def test_cfg_attr_downgrades_are_rejected(self) -> None:
        self.assertTrue(POLICY._has_lint_downgrade_attribute("#![cfg_attr(test, warn(unsafe_code))]"))
        self.assertTrue(POLICY._has_lint_downgrade_attribute("#[cfg_attr(feature = \"x\", allow(clippy::all))]"))

    def test_strict_attributes_are_not_downgrades(self) -> None:
        self.assertFalse(POLICY._has_lint_downgrade_attribute("#![forbid(unsafe_code)]"))
        self.assertFalse(POLICY._has_lint_downgrade_attribute('#[expect(clippy::unwrap_used, reason = "fixture")]'))


if __name__ == "__main__":
    unittest.main()
