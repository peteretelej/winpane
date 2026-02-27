# Python Guide

winpane is accessible from Python (or any language) through the `winpane-host` CLI binary, which speaks JSON-RPC 2.0 over stdin/stdout.

## Setup

Build the host binary:

```sh
cargo build -p winpane-host --release
```

This produces `target/release/winpane-host.exe`. Add it to your PATH or reference it by full path.

## Hello world

```python
import subprocess
import json
import time

proc = subprocess.Popen(
    ["winpane-host"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    text=True,
    bufsize=1,
)

def rpc(method, params, req_id):
    msg = json.dumps({"jsonrpc": "2.0", "method": method, "params": params, "id": req_id})
    proc.stdin.write(msg + "\n")
    proc.stdin.flush()
    return json.loads(proc.stdout.readline())

# Create a HUD
result = rpc("create_hud", {"x": 100, "y": 100, "width": 300, "height": 100}, 1)
sid = result["result"]["surface_id"]

# Add elements
rpc("set_rect", {
    "surface_id": sid, "key": "bg",
    "x": 0, "y": 0, "width": 300, "height": 100,
    "fill": "#14141ec8", "corner_radius": 8,
}, 2)

rpc("set_text", {
    "surface_id": sid, "key": "msg",
    "text": "Hello from Python",
    "x": 16, "y": 16, "font_size": 18,
}, 3)

rpc("show", {"surface_id": sid}, 4)

# Keep alive
time.sleep(10)
proc.terminate()
```

The host process manages all surfaces. When the process exits (or stdin closes), all surfaces are destroyed.

## How the protocol works

Each request is a single-line JSON object sent to stdin. Each response is a single-line JSON object read from stdout. Messages are newline-delimited.

Requests must include `"jsonrpc": "2.0"`, a `"method"`, `"params"`, and an `"id"`. Responses echo the `id` back. Event notifications arrive on stdout with a `"method"` field but no `"id"`.

```python
# Request
{"jsonrpc": "2.0", "method": "create_hud", "params": {"x": 0, "y": 0, "width": 300, "height": 100}, "id": 1}

# Response
{"jsonrpc": "2.0", "result": {"surface_id": "s1"}, "id": 1}

# Event notification (no id)
{"jsonrpc": "2.0", "method": "event", "params": {"type": "element_clicked", "surface_id": "s2", "key": "btn"}}
```

## Surface IDs

Surface IDs are strings prefixed by type: `"s1"`, `"s2"` for surfaces, `"t1"`, `"t2"` for trays. Pass these back in subsequent calls.

## Elements

**Text:**

```python
rpc("set_text", {
    "surface_id": sid, "key": "label",
    "text": "CPU: 42%",
    "x": 16, "y": 50, "font_size": 14,
    "color": "#64dc9f",
    "font_family": "Consolas",  # optional
    "bold": True,               # optional
    "interactive": False,       # optional
}, 5)
```

**Rect:**

```python
rpc("set_rect", {
    "surface_id": sid, "key": "card",
    "x": 10, "y": 10, "width": 280, "height": 80,
    "fill": "#1e1e2ddc",
    "corner_radius": 6,         # optional
    "border_color": "#505078aa", # optional
    "border_width": 1,          # optional
}, 6)
```

**Image:**

```python
rpc("set_image", {
    "surface_id": sid, "key": "icon",
    "path": "C:/icons/logo.png",  # local file path
    "x": 10, "y": 10,
    "width": 32, "height": 32,
}, 7)
```

**Remove:**

```python
rpc("remove_element", {"surface_id": sid, "key": "label"}, 8)
```

## Panels and events

```python
result = rpc("create_panel", {
    "x": 200, "y": 200, "width": 260, "height": 100,
    "draggable": True, "drag_height": 30,
}, 10)
panel_id = result["result"]["surface_id"]

rpc("set_rect", {
    "surface_id": panel_id, "key": "btn",
    "x": 20, "y": 40, "width": 220, "height": 40,
    "fill": "#32508cc8", "corner_radius": 6,
    "interactive": True,
}, 11)
rpc("show", {"surface_id": panel_id}, 12)
```

Events arrive as notifications on stdout. You need to read lines and check whether each line is a response (has `id`) or an event (has `method`):

```python
import select
import sys

def read_events():
    while True:
        line = proc.stdout.readline().strip()
        if not line:
            break
        msg = json.loads(line)
        if "id" in msg:
            # Response to a request
            continue
        if msg.get("method") == "event":
            params = msg["params"]
            if params["type"] == "element_clicked":
                print(f"Clicked: {params['key']}")
```

For a real application, you would run the reader in a separate thread or use non-blocking I/O.

## Surface operations

```python
rpc("show", {"surface_id": sid}, 20)
rpc("hide", {"surface_id": sid}, 21)
rpc("set_position", {"surface_id": sid, "x": 500, "y": 300}, 22)
rpc("set_size", {"surface_id": sid, "width": 400, "height": 200}, 23)
rpc("set_opacity", {"surface_id": sid, "opacity": 0.8}, 24)
rpc("set_capture_excluded", {"surface_id": sid, "excluded": True}, 25)
rpc("set_backdrop", {"surface_id": sid, "backdrop": "mica"}, 26)
rpc("fade_in", {"surface_id": sid, "duration_ms": 300}, 27)
rpc("fade_out", {"surface_id": sid, "duration_ms": 500}, 28)
rpc("anchor_to", {
    "surface_id": sid,
    "target_hwnd": 65538,
    "anchor": "top_right",
    "offset_x": 8, "offset_y": 0,
}, 29)
rpc("unanchor", {"surface_id": sid}, 30)
```

## Tray icons

```python
result = rpc("create_tray", {"tooltip": "My App", "icon_path": "icon.png"}, 40)
tray_id = result["result"]["surface_id"]  # e.g., "t1"

rpc("set_menu", {
    "surface_id": tray_id,
    "items": [
        {"id": 1, "label": "Settings"},
        {"id": 99, "label": "Quit"},
    ],
}, 41)

# Associate a panel as popup
rpc("set_popup", {
    "surface_id": tray_id,
    "panel_surface_id": panel_id,
}, 42)
```

## Cleanup

```python
rpc("destroy", {"surface_id": sid}, 50)
proc.terminate()  # or close stdin to let the host exit
```

The host exits when stdin reaches EOF. All surfaces are destroyed automatically.

## Limitations

- Custom draw is not available over JSON-RPC (requires in-process GPU access)
- Image paths must be local files accessible to the host process
- No batch operations; each request is processed individually
- Tray events do not include a tray ID (cannot distinguish which tray was clicked if multiple exist)

## Next steps

- [Protocol reference](../protocol.md) - Full JSON-RPC method documentation
- [Cookbook](../cookbook.md) - Recipes with JSON-RPC equivalents
- [Limitations](../limitations.md) - Known constraints
