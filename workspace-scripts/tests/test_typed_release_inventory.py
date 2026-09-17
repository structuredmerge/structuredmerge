import importlib.util
from pathlib import Path
import unittest

MODULE = Path(__file__).resolve().parents[1] / "check_typed_release_inventory.py"
SPEC = importlib.util.spec_from_file_location("typed_release_inventory", MODULE)
AUDIT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(AUDIT)


def package(name, dependencies=(), publish=None):
    return dict(name=name, version="1.0.0", source=None, publish=publish,
                manifest_path=f"/kernel/{name}/Cargo.toml", dependencies=list(dependencies))


def dependency(name, **options):
    return dict(name=name, path=f"/kernel/{name}", req="^1.0", **options)


class ReleaseInventoryTest(unittest.TestCase):
    def test_dependency_order_alias_optional_target_and_build_edges(self):
        a = package("a", [dependency("b", rename="alias", optional=True, target="cfg(windows)"),
                          dependency("c", kind="build"), dependency("test-only", kind="dev")])
        result = AUDIT.inventory({"packages": [a, package("b"), package("c")]}, "/kernel", ("a",))
        self.assertEqual([p["package"] for p in result["dependency_first_order"]], ["b", "c", "a"])
        self.assertFalse(result["publication_authorized"])
        self.assertFalse(result["registry_state_checked"])

    def test_rejects_unpublishable_transitive_dependency(self):
        for publish in [[], ["private"]]:
            with self.subTest(publish=publish), self.assertRaisesRegex(ValueError, "cannot publish"):
                AUDIT.inventory({"packages": [package("a", [dependency("b")]), package("b", publish=publish)]}, "/kernel", ("a",))

    def test_rejects_missing_version_cycle_missing_package_and_prototype(self):
        no_version = dependency("b")
        no_version["req"] = "*"
        cases = [
            ([package("a", [no_version]), package("b")], "registry version"),
            ([package("a", [dependency("b")]), package("b", [dependency("a")])], "cycle"),
            ([package("a", [dependency("missing")])], "unresolved"),
            ([package("a", [dependency("host-prototype")]), package("host-prototype")], "prototype"),
        ]
        for packages, message in cases:
            with self.subTest(message=message), self.assertRaisesRegex(ValueError, message):
                AUDIT.inventory({"packages": packages}, "/kernel", ("a",))

    def test_rejects_external_local_package(self):
        external = package("a")
        external["manifest_path"] = "/external/a/Cargo.toml"
        with self.assertRaisesRegex(ValueError, "outside kernel"):
            AUDIT.inventory({"packages": [external]}, "/kernel", ("a",))


if __name__ == "__main__":
    unittest.main()
