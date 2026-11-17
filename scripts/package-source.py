"""Create reproducible source and dependency-notice artifacts, without build trees."""
from pathlib import Path
import os
import subprocess
import json
import zipfile

root = Path(__file__).resolve().parent.parent
destination = root / "dist"
destination.mkdir(exist_ok=True)
metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--locked", "--format-version", "1"], cwd=root))
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
with zipfile.ZipFile(destination / "NothingX-Desktop-source.zip", "w", zipfile.ZIP_DEFLATED) as archive:
    for base, directories, files in os.walk(root):
        directories[:] = sorted(d for d in directories if d not in {".git", "target", "build", "dist", ".DS_Store"})
        for name in sorted(files):
            file = Path(base) / name
            if name != ".DS_Store":
                archive.write(file, Path("NothingX-desktop") / file.relative_to(root))
