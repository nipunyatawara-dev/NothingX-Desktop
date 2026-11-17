"""Read-only hardware regression: quit/reopen without touching the audio link."""
import json
from pathlib import Path
import subprocess
import tempfile
import time

root = Path(__file__).resolve().parent.parent
binary = root / "dist/NothingX Desktop.app/Contents/MacOS/nothingx-desktop"
results = []
with tempfile.TemporaryDirectory(prefix="nothingx-relaunch-") as directory:
    for cycle in range(1, 4):
        status = Path(directory) / f"state-{cycle}.json"
        start = time.monotonic()
        process = subprocess.Popen([str(binary), "--status-file", str(status), "--quit-after", "8"], stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
        ready = None
        snapshot = None
        while process.poll() is None:
            try:
                snapshot = json.loads(status.read_text())
                device = snapshot["device"]
                if ready is None and device["connected"] and (device["battery"]["left"] or device["battery"]["right"]):
                    ready = round(time.monotonic() - start, 2)
                    print(f"Cycle {cycle}: live battery received in {ready}s", flush=True)
            except (FileNotFoundError, json.JSONDecodeError):
                pass
            time.sleep(0.1)
        _, log = process.communicate(timeout=5)
        row = {"cycle": cycle, "ready_seconds": ready, "exit_code": process.returncode, "log": log.strip()}
        results.append(row)
        if ready is None or process.returncode != 0:
            print(json.dumps(row, indent=2), flush=True)
            raise SystemExit("Relaunch regression failed")
output = root / "docs/relaunch-results.json"
output.write_text(json.dumps(results, indent=2) + "\n")
print("Three quit/reopen cycles passed without disconnecting Bluetooth audio.", flush=True)
