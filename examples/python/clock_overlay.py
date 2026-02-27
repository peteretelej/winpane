"""
Clock overlay — floating digital clock showing current time.

Usage:
    python clock_overlay.py

Requires winpane-host.exe in PATH or specify the path below.

Note: This simple send/receive pattern works because HUDs are click-through
(no events). Production clients should parse each stdout line and route by
presence of `id` (response) vs `method` (notification) to handle event
interleaving.
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
import datetime

HOST_PATH = "winpane-host"  # or full path to winpane-host.exe


def main():
    proc = subprocess.Popen(
        [HOST_PATH],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=1,  # line-buffered
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
        # Create a HUD — bottom-right on 1080p, 20px inset
        resp = send("create_hud", {"width": 150, "height": 60, "x": 1750, "y": 1000})
        sid = resp["result"]["surface_id"]
        print(f"Created HUD: {sid}")

        # Background card (Glass theme)
        send("set_rect", {
            "surface_id": sid,
            "key": "bg",
            "x": 0, "y": 0, "width": 150, "height": 60,
            "fill": "#121216e4",
            "corner_radius": 10,
            "border_color": "#ffffff12",
            "border_width": 1,
        })

        # Show it
        send("show", {"surface_id": sid})

        print("winpane clock: ticking clock at bottom-right. Ctrl+C to exit.")

        # Update loop
        while True:
            now = datetime.datetime.now()

            send("set_text", {
                "surface_id": sid,
                "key": "time",
                "text": now.strftime("%H:%M:%S"),
                "x": 16, "y": 8,
                "font_size": 28,
                "font_family": "Consolas",
                "bold": True,
                "color": "#e8e8edff",
            })

            send("set_text", {
                "surface_id": sid,
                "key": "date",
                "text": f"{now.strftime('%a %b')} {now.day}",
                "x": 16, "y": 40,
                "font_size": 12,
                "color": "#9494a0cc",
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
