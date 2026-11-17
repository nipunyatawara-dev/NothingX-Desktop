"""Package Windows x64 app, WebView2 loader and optional per-user NSIS setup."""
import argparse
import json
from pathlib import Path
import shutil
import struct
import subprocess
import tempfile
import urllib.request
import zipfile

root = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("binary", type=Path, nargs="?", default=root / "target/release/nothingx-desktop.exe")
parser.add_argument("--installer", action="store_true")
args = parser.parse_args()
metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--locked", "--format-version", "1"], cwd=root))
version = next(p["version"] for p in metadata["packages"] if p["name"] == "nothingx-desktop")
package = next(p for p in metadata["packages"] if p["name"] == "webview2-com-sys")
loader = Path(package["manifest_path"]).parent / "x64/WebView2Loader.dll"

def require_x64(path):
    data = path.read_bytes()
    if data[:2] != b"MZ":
        raise SystemExit(f"Invalid Windows executable: {path}")
    offset = struct.unpack_from("<I", data, 0x3c)[0]
    if data[offset:offset+4] != b"PE\0\0" or struct.unpack_from("<H", data, offset+4)[0] != 0x8664:
        raise SystemExit(f"Expected a Windows x64 image: {path}")

for path in [args.binary, loader]:
    require_x64(path)
folder = root / f"dist/NothingX-Desktop-{version}-windows-x64"
folder.mkdir(parents=True, exist_ok=True)
shutil.copy2(args.binary, folder / "nothingx-desktop.exe")
shutil.copy2(loader, folder / loader.name)
for name in ["README.md", "LICENSE", "THIRD_PARTY.md"]:
    shutil.copy2(root / name, folder / name)
shutil.copy2(root / "dist/DEPENDENCY_LICENSES.txt", folder / "DEPENDENCY_LICENSES.txt")
archive_path = root / f"dist/NothingX-Desktop-{version}-windows-x64.zip"
with zipfile.ZipFile(archive_path, "w", zipfile.ZIP_DEFLATED) as archive:
    for file in sorted(folder.iterdir()):
        if file.is_file():
            archive.write(file, Path(folder.name) / file.name)
print(archive_path.name, flush=True)
if args.installer:
    compiler = shutil.which("makensis")
    if compiler is None:
        raise SystemExit("Install NSIS and put makensis on PATH to build the Windows installer")
    downloads = root / "build/downloads"
    downloads.mkdir(parents=True, exist_ok=True)
    bootstrapper = downloads / "MicrosoftEdgeWebView2Setup.exe"
    if not bootstrapper.exists():
        url = "https://go.microsoft.com/fwlink/p/?LinkId=2124703"
        with urllib.request.urlopen(url, timeout=60) as response:
            data = response.read()
        if data[:2] != b"MZ":
            raise SystemExit("Microsoft's WebView2 bootstrapper download was not an executable")
        with tempfile.NamedTemporaryFile(dir=downloads, delete=False) as output:
            output.write(data)
            temporary = Path(output.name)
        temporary.replace(bootstrapper)
    output = root / f"dist/NothingX-Desktop-{version}-windows-x64-setup.exe"
    prefix = "/D" if __import__("os").name == "nt" else "-D"
    subprocess.run([compiler, prefix+"APP_VERSION="+version, prefix+"APP_DIR="+str(folder),
                    prefix+"WEBVIEW2_BOOTSTRAPPER="+str(bootstrapper), prefix+"OUTPUT_EXE="+str(output),
                    str(root / "packaging/windows.nsi")], check=True, cwd=root)
    print(output.name, flush=True)
