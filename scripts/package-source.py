"""Create reproducible source and dependency-notice artifacts, without build trees."""
from pathlib import Path
import subprocess
import json
import zipfile

root = Path(__file__).resolve().parent.parent
destination = root / "dist"
destination.mkdir(exist_ok=True)
metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--locked", "--format-version", "1"], cwd=root))
version = next(p["version"] for p in metadata["packages"] if p["name"] == "nothingx-desktop")
notices = []
for package in sorted(metadata["packages"], key=lambda p: (p["name"], p["version"])):
    if package["source"] is None:
        continue
    directory = Path(package["manifest_path"]).parent
    notices.append(f"\n{'=' * 70}\n{package['name']} {package['version']} — {package.get('license', '')}\n")
    files = [p for p in directory.iterdir() if p.is_file() and p.name.lower().startswith(("license", "copying", "notice", "copyright"))]
    for file in sorted(files):
        notices.append(f"\n{file.name}\n{file.read_text(errors='replace')}\n")
(destination / "DEPENDENCY_LICENSES.txt").write_text("".join(notices))
files = subprocess.check_output(["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"], cwd=root).decode().split("\0")
with zipfile.ZipFile(destination / f"NothingX-Desktop-{version}-source.zip", "w", zipfile.ZIP_DEFLATED) as archive:
    for name in sorted(set(files)):
        if name and (root / name).is_file():
            archive.write(root / name, Path(f"NothingX-Desktop-{version}") / name)
