"""
CGM glucose monitor overlay — draggable desktop panel showing blood glucose from Nightscout.

Connects to a Nightscout instance when NIGHTSCOUT_URL is set, otherwise
runs in simulated mode with random-walk glucose values. Stdlib only — no
pip dependencies.

Usage:
    python glucose_monitor.py

Requires winpane-host.exe in PATH or specify the path below.

Environment variables (optional):
    NIGHTSCOUT_URL   — base URL of your Nightscout site
    NIGHTSCOUT_TOKEN — API token for authenticated access

Note: This simple send/receive pattern works because the overlay has no
interactive elements beyond the native drag handle (no event polling needed).
"""
# ── winpane design tokens ──────────────────────────────────────
# Surface base: #121216  Glass: +e4  Solid: +ff  Muted: +f2
# Elevated:     #1c1c21  Interactive: #26262cff  Hover: #303038ff
# Border:       #ffffff12  Text:      #e8e8edff   Muted: #9494a0ff
# Accent:       #528bffff  Success:   #34d399ff   Warning: #fbbf24ff
# Danger:       #ef4444ff  Radius: 10/6 px
# ────────────────────────────────────────────────────────────────
import subprocess
import json
import time
import os
import random
import urllib.request
import urllib.parse

HOST_PATH = "winpane-host"

DIRECTION_ARROWS = {
    "DoubleUp": "⇈",
    "SingleUp": "↑",
    "FortyFiveUp": "↗",
    "Flat": "→",
    "FortyFiveDown": "↘",
    "SingleDown": "↓",
    "DoubleDown": "⇊",
}


def direction_to_arrow(direction):
    return DIRECTION_ARROWS.get(direction, "?")


def bg_color_for_sgv(sgv):
    if 70 <= sgv <= 180:
        return "#12281ee4"
    if 181 <= sgv <= 250:
        return "#282412e4"
    return "#281212e4"


# Staleness measures time since fetch, not CGM reading time.
def staleness_text(reading_time):
    elapsed = time.time() - reading_time
    if elapsed < 60:
        text = "just now"
    else:
        text = f"{int(elapsed // 60)} min ago"
    color = "#ef4444ff" if elapsed > 900 else "#9494a0cc"
    return text, color


def fetch_nightscout(url, token=None):
    try:
        endpoint = f"{url}/api/v1/entries/current.json"
        if token:
            endpoint += f"?token={urllib.parse.quote(token, safe='')}"
        req = urllib.request.Request(endpoint)
        with urllib.request.urlopen(req, timeout=10) as resp:
            data = json.loads(resp.read().decode())
        entry = data[0]
        return {
            "sgv": int(entry["sgv"]),
            "direction": entry.get("direction", "NONE"),
            "timestamp": time.time(),
        }
    except Exception:
        return None


def simulate_reading(prev_sgv):
    delta = random.randint(-15, 15)
    sgv = max(40, min(350, prev_sgv + delta))
    if delta > 10:
        direction = "SingleUp"
    elif delta > 5:
        direction = "FortyFiveUp"
    elif delta > -5:
        direction = "Flat"
    elif delta > -10:
        direction = "FortyFiveDown"
    else:
        direction = "SingleDown"
    return {"sgv": sgv, "direction": direction, "timestamp": time.time()}


def main():
    proc = subprocess.Popen(
        [HOST_PATH],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=1,
    )

    request_id = 0

    def send(method, params=None):
        nonlocal request_id
        request_id += 1
        msg = {
            "jsonrpc": "2.0",
            "method": method,
            "params": params or {},
            "id": request_id,
        }
        line = json.dumps(msg)
        proc.stdin.write(line + "\n")
        proc.stdin.flush()
        resp_line = proc.stdout.readline()
        return json.loads(resp_line)

    try:
        resp = send("create_panel", {"width": 140, "height": 93, "x": 1760, "y": 902, "draggable": True, "drag_height": 28})
        sid = resp["result"]["surface_id"]
        print(f"Created panel: {sid}")

        send("set_capture_excluded", {"surface_id": sid, "excluded": True})

        # Initial bg
        send("set_rect", {
            "surface_id": sid,
            "key": "bg",
            "x": 0, "y": 0, "width": 140, "height": 93,
            "fill": bg_color_for_sgv(120),
            "corner_radius": 10,
            "border_color": "#ffffff12",
            "border_width": 1,
        })

        # Title bar in drag region
        send("set_rect", {
            "surface_id": sid,
            "key": "title_bg",
            "x": 0, "y": 0, "width": 140, "height": 28,
            "fill": "#1c1c21ff",
            "corner_radius": 10,
        })
        send("set_text", {
            "surface_id": sid,
            "key": "title",
            "text": "Glucose",
            "x": 8, "y": 6,
            "font_size": 13,
            "bold": True,
            "color": "#9494a0ff",
        })

        send("show", {"surface_id": sid})

        nightscout_url = os.environ.get("NIGHTSCOUT_URL")
        nightscout_token = os.environ.get("NIGHTSCOUT_TOKEN")
        poll_interval = 5 * 60 if nightscout_url else 30

        if nightscout_url:
            print("winpane glucose_monitor: polling Nightscout every 5 min.")
        else:
            print("winpane glucose_monitor: simulated mode (set NIGHTSCOUT_URL for live data).")
        print("Press Ctrl+C to exit.")

        last_poll = 0.0  # force immediate first poll
        current_reading = {"sgv": 120, "direction": "Flat", "timestamp": time.time()}

        while True:
            now = time.time()
            if now - last_poll >= poll_interval:
                if nightscout_url:
                    reading = fetch_nightscout(nightscout_url, nightscout_token)
                    if reading:
                        current_reading = reading
                else:
                    current_reading = simulate_reading(current_reading["sgv"])
                last_poll = time.time()

            # Update bg
            send("set_rect", {
                "surface_id": sid,
                "key": "bg",
                "x": 0, "y": 0, "width": 140, "height": 93,
                "fill": bg_color_for_sgv(current_reading["sgv"]),
                "corner_radius": 10,
                "border_color": "#ffffff12",
                "border_width": 1,
            })

            # Update reading
            arrow = direction_to_arrow(current_reading["direction"])
            send("set_text", {
                "surface_id": sid,
                "key": "reading",
                "text": f"{current_reading['sgv']} {arrow}",
                "x": 12, "y": 34,
                "font_size": 30,
                "font_family": "Consolas",
                "bold": True,
                "color": "#e8e8edff",
            })

            # Update staleness
            stale_text, stale_color = staleness_text(current_reading["timestamp"])
            send("set_text", {
                "surface_id": sid,
                "key": "staleness",
                "text": stale_text,
                "x": 12, "y": 70,
                "font_size": 12,
                "color": stale_color,
            })

            time.sleep(1)

    except KeyboardInterrupt:
        print("\nShutting down...")
    finally:
        if proc.poll() is None:
            try:
                send("destroy", {"surface_id": sid})
            except (BrokenPipeError, OSError):
                pass
        try:
            proc.stdin.close()
        except (BrokenPipeError, OSError):
            pass
        proc.wait(timeout=5)


if __name__ == "__main__":
    main()
