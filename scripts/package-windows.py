"""Package a linked Windows binary and the required WebView2 loader."""
import json
from pathlib import Path
import shutil
import subprocess
import sys
import zipfile

root = Path(__file__).resolve().parent.parent
binary = Path(sys.argv[1]) if len(sys.argv) > 1 else root / "target/release/nothingx-desktop.exe"
folder = root / "dist/NothingX-Desktop-windows-x64"
folder.mkdir(parents=True, exist_ok=True)
shutil.copy2(binary, folder / "nothingx-desktop.exe")
metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--locked", "--format-version", "1"], cwd=root))
package = next(p for p in metadata["packages"] if p["name"] == "webview2-com-sys")
loader = Path(package["manifest_path"]).parent / "x64/WebView2Loader.dll"
shutil.copy2(loader, folder / loader.name)
for name in ["README.md", "LICENSE", "THIRD_PARTY.md"]:
    shutil.copy2(root / name, folder / name)
shutil.copy2(root / "dist/DEPENDENCY_LICENSES.txt", folder / "DEPENDENCY_LICENSES.txt")
with zipfile.ZipFile(root / "dist/NothingX-Desktop-windows-x64.zip", "w", zipfile.ZIP_DEFLATED) as archive:
    for file in sorted(folder.iterdir()):
        archive.write(file, Path(folder.name) / file.name)
