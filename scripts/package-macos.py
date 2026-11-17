"""Create separate signed Apple Silicon and Intel DMGs from a universal app."""
import json
from pathlib import Path
import plistlib
import subprocess
import tempfile

import dmgbuild

root = Path(__file__).resolve().parent.parent
metadata = json.loads(subprocess.check_output([
    "cargo", "metadata", "--locked", "--no-deps", "--format-version", "1"
], cwd=root))
version = next(p["version"] for p in metadata["packages"] if p["name"] == "nothingx-desktop")
source = root / "dist/NothingX Desktop.app"
for arch, label in [("arm64", "arm64"), ("x86_64", "intel")]:
    with tempfile.TemporaryDirectory(prefix=f"dmg-{label}-", dir=root / "build") as temp:
        app = Path(temp) / "NothingX Desktop.app"
        subprocess.run(["ditto", str(source), str(app)], check=True)
        for relative in ["MacOS/nothingx-desktop", "Resources/nothingx-bluetooth"]:
            binary = app / "Contents" / relative
            architectures = subprocess.check_output(["lipo", "-archs", str(binary)], text=True).split()
            if arch not in architectures:
                raise SystemExit(f"{binary.name} is missing {arch}; build with --universal first")
            if len(architectures) > 1:
                thin = binary.with_suffix(".thin")
                subprocess.run(["lipo", str(binary), "-thin", arch, "-output", str(thin)], check=True)
                thin.replace(binary)
        for item in [app / "Contents/Resources/nothingx-bluetooth", app]:
            subprocess.run(["codesign", "--force", "--sign", "-", str(item)], check=True)
        subprocess.run(["codesign", "--verify", "--deep", "--strict", str(app)], check=True)
        output = root / "dist" / f"NothingX-Desktop-{version}-macos-{label}.dmg"
        dmgbuild.build_dmg(str(output), f"NothingX Desktop {version}", settings={
            "format": "UDZO",
            "files": [str(app)],
            "symlinks": {"Applications": "/Applications"},
            "badge_icon": str(root / "assets/NothingX.icns"),
            "icon_locations": {"NothingX Desktop.app": (160, 170), "Applications": (460, 170)},
            "window_rect": ((180, 160), (640, 360)),
            "icon_size": 112,
            "text_size": 14,
            "background": "#f0f0f0",
            "show_status_bar": False,
            "show_tab_view": False,
            "show_toolbar": False,
            "show_pathbar": False,
            "show_sidebar": False,
            "default_view": "icon-view",
            "include_icon_view_settings": True,
            "include_list_view_settings": False,
        })
        subprocess.run(["hdiutil", "verify", str(output)], check=True)
        # Validate the signed app after the DMG tool has applied its metadata.
        # Finder extension-hiding flags can otherwise invalidate strict checks.
        mount = Path(temp) / "verify"
        mount.mkdir()
        subprocess.run(["hdiutil", "attach", "-readonly", "-nobrowse", "-mountpoint", str(mount), str(output)], check=True, stdout=subprocess.DEVNULL)
        try:
            packaged = mount / "NothingX Desktop.app"
            subprocess.run(["codesign", "--verify", "--deep", "--strict", str(packaged)], check=True)
            info = plistlib.loads((packaged / "Contents/Info.plist").read_bytes())
            if info["CFBundleShortVersionString"] not in [version, version.removesuffix(".0")]:
                raise SystemExit("Application and package versions do not match")
            if not (mount / "Applications").is_symlink():
                raise SystemExit("DMG is missing its Applications link")
            for relative in ["MacOS/nothingx-desktop", "Resources/nothingx-bluetooth"]:
                actual = subprocess.check_output(["lipo", "-archs", str(packaged / "Contents" / relative)], text=True).split()
                if actual != [arch]:
                    raise SystemExit(f"{output.name}: unexpected architectures {actual}")
        finally:
            subprocess.run(["hdiutil", "detach", str(mount)], check=True, stdout=subprocess.DEVNULL)
        print(output.name, flush=True)
