"""Shared version and signed-updater metadata contracts (no private-key access)."""
from __future__ import annotations

import argparse
import base64
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re
import tomllib
from urllib.parse import quote

REPOSITORY = "Cori-bot/Open-Delay"


def project_version(root: Path) -> str:
    version = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))["workspace"]["package"]["version"]
    if not isinstance(version, str) or not re.fullmatch(r"\d+\.\d+\.\d+", version):
        raise ValueError("Release version must be a stable major.minor.patch version")
    return version


def validate_versions(root: Path, tag: str | None = None) -> str:
    version = project_version(root)
    for relative in ("desktop/tauri.conf.json", "ui/package.json", "ui/package-lock.json"):
        value = json.loads((root / relative).read_text(encoding="utf-8"))
        if value.get("version") != version:
            raise ValueError(f"Version mismatch in {relative}: expected {version}")
        if relative.endswith("package-lock.json") and value.get("packages", {}).get("", {}).get("version") != version:
            raise ValueError(f"Root package version mismatch in {relative}")
    if tag is not None and tag != f"v{version}":
        raise ValueError(f"Release tag must match v{version}")
    return version


def read_signature(path: Path) -> str:
    signature = path.read_text(encoding="utf-8").strip()
    try:
        decoded = base64.b64decode(signature, validate=True).decode("utf-8")
        lines = decoded.splitlines()
        if len(lines) != 4 or not lines[0].startswith("untrusted comment:") or not lines[2].startswith("trusted comment:"):
            raise ValueError("Unexpected minisign envelope")
        if len(base64.b64decode(lines[1], validate=True)) != 74 or len(base64.b64decode(lines[3], validate=True)) != 64:
            raise ValueError("Unexpected minisign signature size")
    except (ValueError, UnicodeError) as error:
        raise ValueError(f"Invalid Tauri updater signature: {path.name}") from error
    return signature


def updater_manifest(version: str, repository: str, installer: Path, signature: Path) -> dict:
    if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repository):
        raise ValueError("Repository must be owner/name")
    if installer.name != f"Open-Delay_{version}_x64-setup.exe" or signature.name != installer.name + ".sig":
        raise ValueError("Updater artifacts must match the release version and Windows x64 installer")
    if not installer.is_file() or installer.read_bytes()[:2] != b"MZ":
        raise ValueError("Windows installer is missing or invalid")
    return {
        "version": version,
        "notes": f"Open-Delay {version}. See the GitHub release for changes.",
        "pub_date": datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z"),
        "platforms": {
            "windows-x86_64": {
                "signature": read_signature(signature),
                "url": f"https://github.com/{repository}/releases/download/v{version}/{quote(installer.name)}",
            }
        },
    }


def file_record(path: Path) -> dict:
    with path.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    return {"name": path.name, "bytes": path.stat().st_size, "sha256": digest}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tag")
    args = parser.parse_args()
    print(validate_versions(Path(__file__).resolve().parents[1], args.tag))


if __name__ == "__main__":
    main()
