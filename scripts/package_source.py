"""Package the public source allowlist; never local research or signing keys."""
from pathlib import Path
import os
import zipfile

from release_metadata import project_version

ROOT_FILES = [
    "Cargo.toml", "Cargo.lock", ".gitignore", "README.md", "LICENSE",
    "CONTRIBUTING.md", "DESIGN.md", "VALIDATION.md", "PRODUCT.md",
    "docs/API.md", "docs/BRAND-ASSETS.md", "docs/21ST-ATTRIBUTION.md", "docs/RELEASING.md",
    ".impeccable/design.json", ".impeccable/surfaces/dashboard.md",
]
FOLDERS = ["crates", "desktop", "ui", "integrations/streamdeck", "tests", "scripts", ".github", "dist/compliance"]
EXCLUDED = {"node_modules", "target", "dist", "gen", "__pycache__", ".git", ".local", ".agents", ".codex", ".tmp", "research", "research-artifacts"}
SECRET_SUFFIXES = {".key", ".pem", ".pfx", ".p12"}


def source_files(root: Path) -> list[str]:
    selected = {name for name in ROOT_FILES if (root / name).is_file() and not (root / name).is_symlink()}
    for folder in FOLDERS:
        for directory, children, names in os.walk(root / folder):
            children[:] = [name for name in children if name not in EXCLUDED and not (Path(directory) / name).is_symlink()]
            for name in names:
                path = Path(directory) / name
                if path.is_symlink() or name.endswith((".log", ".tmp", ".tsbuildinfo", ".pyc")):
                    continue
                if name.startswith(".env") or path.suffix.lower() in SECRET_SUFFIXES or name == "NEXT.md":
                    continue
                relative = path.relative_to(root).as_posix()
                if "/dev.opendelay.sdPlugin/bin/" in "/" + relative or "/dev.opendelay.sdPlugin/imgs/" in "/" + relative:
                    continue
                selected.add(relative)
    return sorted(selected)


def package_source(root: Path) -> Path:
    version = project_version(root)
    output = root / f"dist/Open-Delay_{version}-source.zip"
    selected = source_files(root)
    output.parent.mkdir(exist_ok=True, parents=True)
    with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        for relative in selected:
            archive.write(root / relative, "Open-Delay/" + relative)
    with zipfile.ZipFile(output) as archive:
        if archive.testzip() is not None:
            raise ValueError("Source archive integrity failed")
    print(f"{output}: {len(selected)} public source files, {output.stat().st_size} bytes")
    return output


if __name__ == "__main__":
    package_source(Path(__file__).resolve().parents[1])
