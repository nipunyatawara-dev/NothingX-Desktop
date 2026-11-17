"""Hash the exact versioned release assets and record their source revision."""
import datetime
import hashlib
import json
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parent.parent
metadata = json.loads(subprocess.check_output([
    "cargo", "metadata", "--locked", "--no-deps", "--format-version", "1"
], cwd=root))
version = next(p["version"] for p in metadata["packages"] if p["name"] == "nothingx-desktop")
names = [f"NothingX-Desktop-{version}-{suffix}" for suffix in [
    "macos-arm64.dmg", "macos-intel.dmg", "windows-x64-setup.exe", "windows-x64.zip", "source.zip"
]]
hashes = {}
for name in names:
    hashes[name] = hashlib.sha256((root / "dist" / name).read_bytes()).hexdigest()
report = {
    "version": version,
    "built_at_utc": datetime.datetime.now(datetime.UTC).isoformat(),
    "source_commit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip(),
    "source_dirty": bool(subprocess.check_output(["git", "status", "--porcelain"], cwd=root)),
    "built_locally": True,
    "github_actions": False,
    "sha256": hashes,
}
(root / "dist/release-manifest.json").write_text(json.dumps(report, indent=2) + "\n")
(root / "dist/SHA256SUMS.txt").write_text("".join(f"{digest}  {name}\n" for name, digest in hashes.items()))
print(json.dumps(report, indent=2))
