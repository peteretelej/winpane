"""
winpane host demo: Creates a HUD overlay via winpane-host.exe subprocess.

Usage:
    python hud_demo.py

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
        # Create a HUD
        resp = send("create_hud", {"width": 400, "height": 200, "placement": {"monitor": {"index": 0, "anchor": "top_left", "margin": 40}}})
        surface_id = resp["result"]["surface_id"]
        print(f"Created HUD: {surface_id}")

        # Add a dark background
        send("set_rect", {
            "surface_id": surface_id,
            "key": "bg",
            "x": 0, "y": 0, "width": 400, "height": 200,
            "fill": "#121216e4",
            "corner_radius": 10,
            "border_color": "#ffffff12",
            "border_width": 1,
        })

        # Add text
        send("set_text", {
            "surface_id": surface_id,
            "key": "title",
            "text": "Hello from Python!",
            "x": 20, "y": 20,
            "font_size": 16,
            "color": "#e8e8ed",
        })

        send("set_text", {
            "surface_id": surface_id,
            "key": "subtitle",
            "text": "winpane JSON-RPC host demo",
            "x": 20, "y": 60,
            "font_size": 13,
            "color": "#9494a0",
        })

        # Show it
        send("show", {"surface_id": surface_id})

        # Keep alive for 5 seconds with a counter
        for i in range(50):
            send("set_text", {
                "surface_id": surface_id,
                "key": "counter",
                "text": f"Elapsed: {i * 0.1:.1f}s",
                "x": 20, "y": 100,
                "font_size": 14,
                "color": "#34d399",
            })
            time.sleep(0.1)

        # Cleanup
        send("destroy", {"surface_id": surface_id})
        print("Done!")

    finally:
        proc.stdin.close()
        proc.wait(timeout=5)


if __name__ == "__main__":
    main()
