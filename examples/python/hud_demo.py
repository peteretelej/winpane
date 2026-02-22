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
        resp = send("create_hud", {"width": 400, "height": 200, "x": 100, "y": 100})
        surface_id = resp["result"]["surface_id"]
        print(f"Created HUD: {surface_id}")

        # Add a dark background
        send("set_rect", {
            "surface_id": surface_id,
            "key": "bg",
            "x": 0, "y": 0, "width": 400, "height": 200,
            "fill": "#1a1a2eee",
            "corner_radius": 8,
        })

        # Add text
        send("set_text", {
            "surface_id": surface_id,
            "key": "title",
            "text": "Hello from Python!",
            "x": 20, "y": 20,
            "font_size": 24,
            "color": "#ffffff",
        })

        send("set_text", {
            "surface_id": surface_id,
            "key": "subtitle",
            "text": "winpane JSON-RPC host demo",
            "x": 20, "y": 60,
            "font_size": 14,
            "color": "#888888",
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
                "font_size": 16,
                "color": "#00ff88",
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
