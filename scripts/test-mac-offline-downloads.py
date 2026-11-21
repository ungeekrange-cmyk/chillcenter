"""Exercise the packaged UI and native player against a disposable HTTPS library."""

import json
import os
import pathlib
import socket
import sqlite3
import ssl
import subprocess
import sys
import tempfile
import time
import urllib.request
repo = pathlib.Path(__file__).resolve().parent.parent
target = pathlib.Path(os.environ.get("CARGO_TARGET_DIR", repo / "target")).resolve()
binary = target / "debug"
if not (binary / "chillcenter-desktop").is_file() or not (binary / "chill-server").is_file():
    sys.exit("Build chillcenter-desktop and chill-server in debug mode first.")
with tempfile.TemporaryDirectory(prefix="offline-mac-fixture-") as work:
    root = pathlib.Path(work).resolve()
    media, data, desktop = (root / part for part in ("film", "server", "client"))
    for folder in (media, data, desktop):
        folder.mkdir()
    fixture = media / "Offline Fixture.mp4"
    subprocess.run(["ffmpeg", "-v", "error", "-f", "lavfi", "-i", "testsrc2=size=160x100:rate=24:duration=10",
                    "-f", "lavfi", "-i", "sine=frequency=440:sample_rate=48000:duration=10", "-c:v", "libx264",
                    "-pix_fmt", "yuv420p", "-c:a", "aac", "-movflags", "+faststart", "-shortest", str(fixture)], check=True)
    (media / "Offline Fixture.en.srt").write_text("1\n00:00:00,000 --> 00:00:06,000\nOffline native caption\n")
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        port = sock.getsockname()[1]
    address = f"https://127.0.0.1:{port}"
    cfg = root / "config.json"
    cfg.write_text(json.dumps({"listen": f"127.0.0.1:{port}", "data_dir": str(data), "tls_names": ["127.0.0.1"], "video_root": str(media)}))
    env = dict(os.environ, CHILL_UI_QA_URL=address, CHILL_UI_QA_ROOT=str(media), CHILL_UI_QA_DATA=str(desktop))
    with open(root / "server.log", "w+") as log:
        server = subprocess.Popen([binary / "chill-server", "--config", cfg], stdout=log, stderr=subprocess.STDOUT)
        try:
            for _ in range(100):
                with socket.socket() as ready:
                    if ready.connect_ex(("127.0.0.1", port)) == 0:
                        break
                if server.poll() is not None:
                    log.seek(0)
                    sys.exit("fixture server did not start: " + log.read())
                time.sleep(.1)
            else:
                sys.exit("fixture server did not listen")
            context = ssl.create_default_context(cafile=str(data / "tls" / "certificate.pem"))
            scan_url = address + "/api/2026-10-14/admin/scan"
            with urllib.request.urlopen(urllib.request.Request(scan_url, data=b"{}", headers={"Content-Type": "application/json"}), context=context, timeout=5) as response:
                print("Fixture scan", response.status, flush=True)
            for _ in range(100):
                with urllib.request.urlopen(scan_url, context=context, timeout=5) as response:
                    status = json.load(response)
                if not status["running"]:
                    if status["count"] != 1:
                        sys.exit(f"fixture scan failed: {status}")
                    break
                time.sleep(.1)
            else:
                sys.exit("fixture scan did not finish")
            subprocess.run([binary / "chillcenter-desktop"], env=dict(env, CHILL_UI_QA_MODE="offline_prepare"), timeout=45, check=True)
        finally:
            server.terminate()
            server.wait(timeout=10)
    report = root / "desktop-offline.log"
    with report.open("w") as log:
        app = subprocess.Popen([binary / "chillcenter-desktop"],
                               env=dict(env, CHILL_UI_QA_MODE="offline_reopen"),
                               stdout=log, stderr=subprocess.STDOUT)
        try:
            deadline = time.monotonic() + 45
            downloads = desktop / "downloads"
            index = downloads / "index.sqlite3"
            while time.monotonic() < deadline:
                trace = report.read_text()
                if app.poll() is not None:
                    sys.exit(trace or "offline app exited before playback completed")
                if "TRACE cold playback with external subtitle selected" in trace:
                    with sqlite3.connect(index, timeout=2) as db:
                        entry = db.execute("SELECT payload FROM download_index WHERE kind = 'progress'").fetchone()
                        copies = db.execute("SELECT count(*) FROM download_index WHERE kind = 'copies'").fetchone()[0]
                    if entry and json.loads(entry[0])["watched"] and copies == 0 and not list(downloads.rglob("video-*")):
                        print(trace.strip(), flush=True)
                        print("PASS macOS cold reopen, native playback, local subtitles and EOF deletion", flush=True)
                        break
                time.sleep(.2)
            else:
                sys.exit(trace or "offline app timed out")
        finally:
            app.terminate()
            app.wait(timeout=10)
