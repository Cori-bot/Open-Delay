import base64
import json
from pathlib import Path
import tempfile
import unittest
import zipfile

from package_source import package_source, source_files
from package_release import package_release
from release_metadata import read_signature, updater_manifest, validate_versions


def signature_fixture() -> str:
    # Structurally valid public-only fixture, not a cryptographic trust assertion.
    payload = "\n".join([
        "untrusted comment: fixture", base64.b64encode(b"x" * 74).decode(),
        "trusted comment: timestamp:1", base64.b64encode(b"y" * 64).decode(), "",
    ])
    return base64.b64encode(payload.encode()).decode()


class ReleaseMetadataTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        (self.root / "Cargo.toml").write_text('[workspace.package]\nversion="0.2.0"\n', encoding="utf-8")
        for relative in ("desktop/tauri.conf.json", "ui/package.json", "ui/package-lock.json"):
            path = self.root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(json.dumps({"version": "0.2.0", "packages": {"": {"version": "0.2.0"}}}), encoding="utf-8")

    def artifacts(self):
        installer = self.root / "Open-Delay_0.2.0_x64-setup.exe"
        installer.write_bytes(b"MZ fixture")
        signature = installer.with_name(installer.name + ".sig")
        signature.write_text(signature_fixture(), encoding="utf-8")
        return installer, signature

    def test_tag_and_every_application_version_must_agree(self):
        self.assertEqual(validate_versions(self.root, "v0.2.0"), "0.2.0")
        with self.assertRaises(ValueError):
            validate_versions(self.root, "v0.1.0")
        (self.root / "ui/package.json").write_text('{"version":"0.1.0"}', encoding="utf-8")
        with self.assertRaises(ValueError):
            validate_versions(self.root)

    def test_lockfile_root_version_is_checked(self):
        (self.root / "ui/package-lock.json").write_text('{"version":"0.2.0","packages":{"":{"version":"0.1.0"}}}', encoding="utf-8")
        with self.assertRaises(ValueError):
            validate_versions(self.root)

    def test_manifest_points_to_exact_signed_installer(self):
        installer, signature = self.artifacts()
        manifest = updater_manifest("0.2.0", "Cori-bot/Open-Delay", installer, signature)
        platform = manifest["platforms"]["windows-x86_64"]
        self.assertEqual(platform["signature"], signature_fixture())
        self.assertEqual(platform["url"], "https://github.com/Cori-bot/Open-Delay/releases/download/v0.2.0/Open-Delay_0.2.0_x64-setup.exe")
        self.assertTrue(manifest["pub_date"].endswith("Z"))

    def test_invalid_signature_or_wrong_artifact_never_produces_manifest(self):
        installer, signature = self.artifacts()
        signature.write_text("not a signature", encoding="utf-8")
        with self.assertRaises(ValueError):
            read_signature(signature)
        with self.assertRaises(ValueError):
            updater_manifest("0.3.0", "Cori-bot/Open-Delay", installer, signature)
        with self.assertRaises(ValueError):
            updater_manifest("0.2.0", "https://other.example", installer, signature)

    def test_source_archive_omits_private_directories_and_keys(self):
        for name in (".local/key", ".tmp/private.txt", ".agents/skills/a.md", "research-artifacts/private.txt", "NEXT.md", "scripts/.env", "scripts/signing.key", "ui/node_modules/private.js", "desktop/gen/private.json"):
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("PRIVATE", encoding="utf-8")
        (self.root / "scripts/public.py").write_text("print('public')", encoding="utf-8")
        selected = source_files(self.root)
        self.assertIn("scripts/public.py", selected)
        self.assertFalse(any("private" in name or name.endswith(".key") or ".env" in name or name == "NEXT.md" for name in selected))
        archive = package_source(self.root)
        self.assertEqual(archive.name, "Open-Delay_0.2.0-source.zip")
        with zipfile.ZipFile(archive) as zipped:
            self.assertIn("Open-Delay/scripts/public.py", zipped.namelist())
            self.assertFalse(any(b"PRIVATE" in zipped.read(name) for name in zipped.namelist()))

    def test_packaging_uses_current_version_and_never_collects_stale_dist(self):
        installer, signature = self.artifacts()
        nsis = self.root / "target/release/bundle/nsis"
        nsis.mkdir(parents=True)
        (nsis / installer.name).write_bytes(installer.read_bytes())
        (nsis / signature.name).write_text(signature.read_text(), encoding="utf-8")
        (self.root / "target/release/open-delay-desktop.exe").write_bytes(b"MZ...app-current.js...app-current.css")
        (self.root / "ui/dist").mkdir(parents=True)
        (self.root / "ui/dist/index.html").write_text('<script src="/assets/app-current.js"></script><link href="/assets/app-current.css">', encoding="utf-8")
        plugin = self.root / "integrations/streamdeck/dist/dev.opendelay.streamDeckPlugin"
        plugin.parent.mkdir(parents=True)
        plugin.write_bytes(b"plugin")
        (self.root / "dist").mkdir()
        (self.root / "dist/old-secret.txt").write_text("PRIVATE", encoding="utf-8")
        manifest = package_release(self.root, "Cori-bot/Open-Delay", "v0.2.0")
        self.assertFalse(manifest["authenticode_signed"])
        self.assertTrue(manifest["updater_signature"]["present"])
        names = [record["name"] for record in manifest["files"]]
        self.assertNotIn("old-secret.txt", names)
        self.assertIn("Open-Delay_0.2.0_x64-setup.exe.sig", names)
        self.assertIn("latest.json", names)
        with zipfile.ZipFile(self.root / "dist/Open-Delay_0.2.0-source.zip") as archive:
            self.assertFalse(any("old-secret" in name for name in archive.namelist()))


if __name__ == "__main__":
    unittest.main()
