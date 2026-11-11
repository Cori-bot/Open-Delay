"""Verify current embedded UI and collect signed Windows release deliverables."""
import argparse
import json
import os
from pathlib import Path
import re
import shutil

from package_source import package_source
from release_metadata import REPOSITORY, file_record, updater_manifest, validate_versions


def package_release(root: Path, repository: str, tag: str | None = None) -> dict:
    version = validate_versions(root, tag)
    dist = root / "dist"
    binary = root / "target/release/open-delay-desktop.exe"
    installer = root / f"target/release/bundle/nsis/Open-Delay_{version}_x64-setup.exe"
    signature = installer.with_name(installer.name + ".sig")
    plugin = root / "integrations/streamdeck/dist/dev.opendelay.streamDeckPlugin"
    html = (root / "ui/dist/index.html").read_text(encoding="utf-8")
    assets = re.findall(r'/assets/([^"\s]+\.(?:js|css))', html)
    image = binary.read_bytes()
    if image[:2] != b"MZ" or not assets:
        raise ValueError("Native executable or built frontend missing")
    for asset in assets:
        if asset.encode() not in image:
            raise ValueError(f"Native frontend is stale: {asset} missing. Rebuild Tauri.")
    latest = updater_manifest(version, repository, installer, signature)
    if not plugin.is_file():
        raise ValueError("Build and pack the Stream Deck plugin first")
    dist.mkdir(exist_ok=True)
    shutil.copy2(installer, dist / installer.name)
    shutil.copy2(signature, dist / signature.name)
    plugin_output = dist / f"Open-Delay_{version}.streamDeckPlugin"
    shutil.copy2(plugin, plugin_output)
    source = package_source(root)
    latest_path = dist / "latest.json"
    latest_path.write_text(json.dumps(latest, indent=2) + "\n", encoding="utf-8")
    files = [dist / installer.name, dist / signature.name, plugin_output, source, latest_path]
    manifest = {
        "name": "Open-Delay", "version": version, "repository": repository,
        "updater_signature": {"algorithm": "minisign", "file": signature.name, "present": True},
        "authenticode_signed": False,
        "embedded_assets": assets,
        "files": [file_record(path) for path in files],
    }
    release_path = dist / "release.json"
    release_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    records = manifest["files"] + [file_record(release_path)]
    (dist / "SHA256SUMS.txt").write_text("\n".join(f"{item['sha256']}  {item['name']}" for item in records) + "\n", encoding="utf-8")
    return manifest


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", default=os.environ.get("GITHUB_REPOSITORY", REPOSITORY))
    parser.add_argument("--tag")
    args = parser.parse_args()
    print(json.dumps(package_release(Path(__file__).resolve().parents[1], args.repository, args.tag), indent=2))
