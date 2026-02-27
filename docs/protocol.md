# winpane-host JSON-RPC 2.0 Protocol Reference

## Overview

`winpane-host` is a CLI binary that exposes the winpane SDK over a JSON-RPC 2.0 protocol using stdin/stdout. Any language that can spawn a subprocess and read/write lines can create Windows overlay surfaces, HUDs, panels, PiP thumbnails, and system tray icons.

**Transport:** Line-delimited JSON over stdin (requests) and stdout (responses and event notifications). Each message is a single JSON object terminated by a newline (`\n`). Stderr is reserved for diagnostics (startup banner, fatal errors).

**JSON-RPC 2.0 compliance:** All messages include `"jsonrpc": "2.0"`. Requests have an `id` field; responses echo it back. Event notifications have no `id`.

**Lifecycle:** The host process creates surfaces on demand. All surfaces are destroyed when the host exits. The host exits when stdin reaches EOF (parent process closed the pipe or died). There is no daemon mode.

**Event interleaving:** Event notifications may arrive on stdout at any time, interspersed with responses. Clients must inspect each line: messages with an `id` field are responses to requests; messages with a `method` field and no `id` are event notifications.

## Quick Start

Spawn the host and send line-delimited JSON:

```bash
winpane-host < requests.jsonl
```

Minimal session (create a HUD, add text, show it, destroy it):

```json
{"jsonrpc":"2.0","method":"create_hud","params":{"x":100,"y":100,"width":400,"height":200},"id":1}
```
```json
{"jsonrpc":"2.0","result":{"surface_id":"s1"},"id":1}
```

```json
{"jsonrpc":"2.0","method":"set_text","params":{"surface_id":"s1","key":"hello","text":"Hello World","x":20,"y":20,"font_size":24},"id":2}
```
```json
{"jsonrpc":"2.0","result":{},"id":2}
```

```json
{"jsonrpc":"2.0","method":"show","params":{"surface_id":"s1"},"id":3}
```
```json
{"jsonrpc":"2.0","result":{},"id":3}
```

```json
{"jsonrpc":"2.0","method":"destroy","params":{"surface_id":"s1"},"id":4}
```
```json
{"jsonrpc":"2.0","result":{},"id":4}
```

## Methods Reference

### Surface Creation

#### `create_hud`

Creates a click-through HUD overlay. Returns a surface ID.

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| `x` | integer | yes | X position in pixels |
| `y` | integer | yes | Y position in pixels |
| `width` | unsigned integer | yes | Width in pixels |
| `height` | unsigned integer | yes | Height in pixels |

**Request:**
```json
{"jsonrpc":"2.0","method":"create_hud","params":{"x":100,"y":100,"width":400,"height":200},"id":1}
```

**Response:**
```json
{"jsonrpc":"2.0","result":{"surface_id":"s1"},"id":1}
```

HUDs are always topmost, click-through, and have no taskbar entry. They support text, rect, and image elements via the scene graph.

#### `create_panel`

Creates an interactive panel surface with selective click-through.

| Param | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `x` | integer | yes | | X position |
| `y` | integer | yes | | Y position |
| `width` | unsigned integer | yes | | Width |
| `height` | unsigned integer | yes | | Height |
| `draggable` | boolean | no | `false` | Enable title-bar dragging |
| `drag_height` | unsigned integer | no | `0` | Height of the drag region from the top |

**Request:**
```json
{"jsonrpc":"2.0","method":"create_panel","params":{"x":200,"y":200,"width":300,"height":250,"draggable":true,"drag_height":40},"id":1}
```

**Response:**
```json
{"jsonrpc":"2.0","result":{"surface_id":"s1"},"id":1}
```

Panels support interactive elements (buttons, clickable regions). Non-interactive areas are click-through. Events are delivered as notifications.

#### `create_pip`

Creates a Picture-in-Picture surface showing a live DWM thumbnail of another window.

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| `source_hwnd` | integer | yes | Window handle (HWND) of the source window |
| `x` | integer | yes | X position |
| `y` | integer | yes | Y position |
| `width` | unsigned integer | yes | Width |
| `height` | unsigned integer | yes | Height |

**Request:**
```json
{"jsonrpc":"2.0","method":"create_pip","params":{"source_hwnd":12345,"x":0,"y":0,"width":320,"height":240},"id":1}
```

**Response:**
```json
{"jsonrpc":"2.0","result":{"surface_id":"s1"},"id":1}
```

PiP surfaces do not support scene graph operations (`set_text`, `set_rect`, `set_image`, `remove_element`). They support `set_source_region` and `clear_source_region` for cropping.

#### `create_tray`

Creates a system tray icon.

| Param | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `tooltip` | string | yes | | Tooltip text |
| `icon_path` | string | no | 16x16 white | Path to icon image file (PNG/JPEG/BMP) |

**Request:**
```json
{"jsonrpc":"2.0","method":"create_tray","params":{"tooltip":"My App"},"id":1}
```

**Response:**
```json
{"jsonrpc":"2.0","result":{"surface_id":"t1"},"id":1}
```

Tray IDs use the `t` prefix (e.g., `"t1"`, `"t2"`). If no icon is provided, a default 16x16 white icon is used.

### Element Operations

These methods add or update elements in a surface's scene graph. They require a `surface_id` pointing to a HUD or Panel (not a PiP).

#### `set_text`

Adds or updates a text element.

| Param | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `surface_id` | string | yes | | Target surface |
| `key` | string | yes | | Element key (unique within the surface) |
| `text` | string | yes | | Text content |
| `x` | number | yes | | X position |
| `y` | number | yes | | Y position |
| `font_size` | number | yes | | Font size in pixels |
| `color` | string | no | `"#ffffff"` | Text color (hex) |
| `font_family` | string | no | system default | Font family name |
| `bold` | boolean | no | `false` | Bold text |
| `italic` | boolean | no | `false` | Italic text |
| `interactive` | boolean | no | `false` | Receives click/hover events (Panel only) |

**Request:**
```json
{"jsonrpc":"2.0","method":"set_text","params":{"surface_id":"s1","key":"title","text":"Hello","x":10,"y":10,"font_size":24,"color":"#ffffff","bold":true},"id":2}
```

**Response:**
```json
{"jsonrpc":"2.0","result":{},"id":2}
```

#### `set_rect`

Adds or updates a rectangle element.

| Param | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `surface_id` | string | yes | | Target surface |
| `key` | string | yes | | Element key |
| `x` | number | yes | | X position |
| `y` | number | yes | | Y position |
| `width` | number | yes | | Width |
| `height` | number | yes | | Height |
| `fill` | string | no | `"#ffffff"` | Fill color (hex) |
| `corner_radius` | number | no | `0` | Corner radius for rounded rects |
| `border_color` | string | no | none | Border color (hex) |
| `border_width` | number | no | `0` | Border width |
| `interactive` | boolean | no | `false` | Receives click/hover events (Panel only) |

**Request:**
```json
{"jsonrpc":"2.0","method":"set_rect","params":{"surface_id":"s1","key":"bg","x":0,"y":0,"width":400,"height":200,"fill":"#1a1a2eee","corner_radius":8},"id":2}
```

**Response:**
```json
{"jsonrpc":"2.0","result":{},"id":2}
```

#### `set_image`

Adds or updates an image element. The image is loaded from a file path accessible to the host process.

| Param | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `surface_id` | string | yes | | Target surface |
| `key` | string | yes | | Element key |
| `path` | string | yes | | Path to image file (PNG/JPEG/BMP) |
| `x` | number | yes | | X position |
| `y` | number | yes | | Y position |
| `width` | number | yes | | Display width |
| `height` | number | yes | | Display height |
| `interactive` | boolean | no | `false` | Receives click/hover events (Panel only) |

**Request:**
```json
{"jsonrpc":"2.0","method":"set_image","params":{"surface_id":"s1","key":"logo","path":"C:/icons/logo.png","x":10,"y":10,"width":64,"height":64},"id":2}
```

**Response:**
```json
{"jsonrpc":"2.0","result":{},"id":2}
```

The image is decoded and converted to premultiplied RGBA8 internally. Supported formats: PNG, JPEG, BMP.

#### `remove_element`

Removes an element by key.

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| `surface_id` | string | yes | Target surface |
| `key` | string | yes | Element key to remove |

**Request:**
```json
{"jsonrpc":"2.0","method":"remove_element","params":{"surface_id":"s1","key":"counter"},"id":3}
```

**Response:**
```json
{"jsonrpc":"2.0","result":{},"id":3}
```

### Surface Control

These methods control surface visibility, position, size, and opacity. They work on all surface types (HUD, Panel, PiP).

#### `show`

Makes a surface visible.

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| `surface_id` | string | yes | Target surface |

**Request:**
```json
{"jsonrpc":"2.0","method":"show","params":{"surface_id":"s1"},"id":3}
```

#### `hide`

Makes a surface invisible.

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| `surface_id` | string | yes | Target surface |

**Request:**
```json
{"jsonrpc":"2.0","method":"hide","params":{"surface_id":"s1"},"id":3}
```

#### `set_position`

Moves a surface to a new position.

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| `surface_id` | string | yes | Target surface |
| `x` | integer | yes | New X position |
| `y` | integer | yes | New Y position |

**Request:**
```json
{"jsonrpc":"2.0","method":"set_position","params":{"surface_id":"s1","x":500,"y":300},"id":4}
```

#### `set_size`

Resizes a surface.

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| `surface_id` | string | yes | Target surface |
| `width` | unsigned integer | yes | New width |
| `height` | unsigned integer | yes | New height |

**Request:**
```json
{"jsonrpc":"2.0","method":"set_size","params":{"surface_id":"s1","width":600,"height":400},"id":4}
```

#### `set_opacity`

Sets surface opacity.

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| `surface_id` | string | yes | Target surface |
| `opacity` | number | yes | Opacity from 0.0 (invisible) to 1.0 (opaque) |

**Request:**
```json
{"jsonrpc":"2.0","method":"set_opacity","params":{"surface_id":"s1","opacity":0.8},"id":4}
```

### Anchoring

#### `anchor_to`

Anchors a surface to a corner of another window. The surface follows the target window as it moves.

| Param | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `surface_id` | string | yes | | Surface to anchor |
| `target_hwnd` | integer | yes | | Target window handle (HWND) |
| `anchor` | string | yes | | Anchor corner: `"top_left"`, `"top_right"`, `"bottom_left"`, `"bottom_right"` |
| `offset_x` | integer | no | `0` | Horizontal offset from anchor point |
| `offset_y` | integer | no | `0` | Vertical offset from anchor point |

**Request:**
```json
{"jsonrpc":"2.0","method":"anchor_to","params":{"surface_id":"s1","target_hwnd":65538,"anchor":"top_right","offset_x":10,"offset_y":0},"id":5}
```

**Response:**
```json
{"jsonrpc":"2.0","result":{},"id":5}
```

If the target window is closed, an `anchor_target_closed` event is emitted.

#### `unanchor`

Removes anchoring from a surface.

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| `surface_id` | string | yes | Surface to unanchor |

**Request:**
```json
{"jsonrpc":"2.0","method":"unanchor","params":{"surface_id":"s1"},"id":5}
```

### Backdrop

#### `set_backdrop`

Sets a DWM backdrop effect on a surface. Requires Windows 11 22H2 or later. Silent no-op on older versions.

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| `surface_id` | string | yes | Target surface |
| `backdrop` | string | yes | Backdrop type: `"none"`, `"mica"`, `"acrylic"` |

**Request:**
```json
{"jsonrpc":"2.0","method":"set_backdrop","params":{"surface_id":"s1","backdrop":"mica"},"id":10}
```

**Response:**
```json
{"jsonrpc":"2.0","result":{},"id":10}
```

#### `backdrop_supported`

Returns whether the current system supports DWM backdrop effects (Windows 11 22H2+).

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| (none) | | | |

**Request:**
```json
{"jsonrpc":"2.0","method":"backdrop_supported","params":{},"id":10}
```

**Response:**
```json
{"jsonrpc":"2.0","result":{"supported":true},"id":10}
```

### Animations

#### `fade_in`

Shows the surface and animates its opacity from 0 to 1 over the given duration using DirectComposition.

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| `surface_id` | string | yes | Target surface |
| `duration_ms` | unsigned integer | yes | Animation duration in milliseconds |

**Request:**
```json
{"jsonrpc":"2.0","method":"fade_in","params":{"surface_id":"s1","duration_ms":300},"id":11}
```

**Response:**
```json
{"jsonrpc":"2.0","result":{},"id":11}
```

#### `fade_out`

Animates the surface opacity from 1 to 0 over the given duration, then hides the surface.

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| `surface_id` | string | yes | Target surface |
| `duration_ms` | unsigned integer | yes | Animation duration in milliseconds |

**Request:**
```json
{"jsonrpc":"2.0","method":"fade_out","params":{"surface_id":"s1","duration_ms":500},"id":12}
```

**Response:**
```json
{"jsonrpc":"2.0","result":{},"id":12}
```

### Capture Exclusion

#### `set_capture_excluded`

Excludes a surface from screen capture (screenshots, screen sharing).

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| `surface_id` | string | yes | Target surface |
| `excluded` | boolean | yes | `true` to exclude from capture, `false` to include |

**Request:**
```json
{"jsonrpc":"2.0","method":"set_capture_excluded","params":{"surface_id":"s1","excluded":true},"id":5}
```

Requires Windows 10 2004+ (`SetWindowDisplayAffinity`). Silently degrades on older versions.

### PiP-Specific

#### `set_source_region`

Crops the PiP source window to show only a specific region.

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| `surface_id` | string | yes | PiP surface ID |
| `x` | integer | yes | Source crop X |
| `y` | integer | yes | Source crop Y |
| `width` | integer | yes | Source crop width |
| `height` | integer | yes | Source crop height |

**Request:**
```json
{"jsonrpc":"2.0","method":"set_source_region","params":{"surface_id":"s1","x":0,"y":0,"width":800,"height":600},"id":6}
```

Only valid on PiP surfaces. Returns an error if called on HUD or Panel.

#### `clear_source_region`

Clears the source crop, showing the full source window.

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| `surface_id` | string | yes | PiP surface ID |

**Request:**
```json
{"jsonrpc":"2.0","method":"clear_source_region","params":{"surface_id":"s1"},"id":6}
```

### Tray-Specific

These methods operate on tray icons (IDs starting with `t`).

#### `set_tooltip`

Updates the tray icon tooltip text.

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| `surface_id` | string | yes | Tray ID |
| `tooltip` | string | yes | New tooltip text |

**Request:**
```json
{"jsonrpc":"2.0","method":"set_tooltip","params":{"surface_id":"t1","tooltip":"Updated status"},"id":7}
```

#### `set_tray_icon`

Updates the tray icon image.

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| `surface_id` | string | yes | Tray ID |
| `icon_path` | string | yes | Path to icon image file |

**Request:**
```json
{"jsonrpc":"2.0","method":"set_tray_icon","params":{"surface_id":"t1","icon_path":"C:/icons/active.png"},"id":7}
```

#### `set_popup`

Associates a Panel surface as the tray popup. Left-clicking the tray icon toggles the panel's visibility.

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| `surface_id` | string | yes | Tray ID |
| `panel_surface_id` | string | yes | Panel surface ID to use as popup |

**Request:**
```json
{"jsonrpc":"2.0","method":"set_popup","params":{"surface_id":"t1","panel_surface_id":"s2"},"id":7}
```

The panel must have been created with `create_panel`. Using a HUD or PiP surface returns an error.

#### `set_menu`

Sets the right-click context menu for a tray icon.

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| `surface_id` | string | yes | Tray ID |
| `items` | array | yes | Menu items (see below) |

Each menu item:

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `id` | unsigned integer | yes | | Unique item ID (returned in `tray_menu_item_clicked` events) |
| `label` | string | yes | | Display text |
| `enabled` | boolean | no | `true` | Whether the item is clickable |

**Request:**
```json
{"jsonrpc":"2.0","method":"set_menu","params":{"surface_id":"t1","items":[{"id":1,"label":"Settings"},{"id":2,"label":"Quit"}]},"id":7}
```

### Lifecycle

#### `destroy`

Destroys a surface or tray by ID.

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| `surface_id` | string | yes | Surface or tray ID to destroy |

**Request:**
```json
{"jsonrpc":"2.0","method":"destroy","params":{"surface_id":"s1"},"id":8}
```

**Response:**
```json
{"jsonrpc":"2.0","result":{},"id":8}
```

The surface window is destroyed immediately. Elements and resources are freed. Using a destroyed surface ID in subsequent calls returns an error.

## Events

Events are delivered as JSON-RPC notifications (no `id` field) with `"method": "event"`. The `params` object contains the event details.

### `element_clicked`

An interactive element was clicked.

```json
{"jsonrpc":"2.0","method":"event","params":{"type":"element_clicked","surface_id":"s2","key":"btn1"}}
```

### `element_hovered`

The mouse entered an interactive element.

```json
{"jsonrpc":"2.0","method":"event","params":{"type":"element_hovered","surface_id":"s2","key":"btn1"}}
```

### `element_left`

The mouse left an interactive element.

```json
{"jsonrpc":"2.0","method":"event","params":{"type":"element_left","surface_id":"s2","key":"btn1"}}
```

### `tray_clicked`

A tray icon was clicked.

```json
{"jsonrpc":"2.0","method":"event","params":{"type":"tray_clicked","button":"left"}}
```

`button` is one of: `"left"`, `"right"`, `"middle"`.

Note: `tray_clicked` does not include a tray ID (SDK limitation). If multiple trays exist, events cannot distinguish which tray was clicked.

### `tray_menu_item_clicked`

A tray context menu item was selected.

```json
{"jsonrpc":"2.0","method":"event","params":{"type":"tray_menu_item_clicked","item_id":2}}
```

Note: Like `tray_clicked`, this event does not include a tray ID.

### `pip_source_closed`

The source window of a PiP surface was closed.

```json
{"jsonrpc":"2.0","method":"event","params":{"type":"pip_source_closed","surface_id":"s3"}}
```

### `anchor_target_closed`

The target window of an anchored surface was closed.

```json
{"jsonrpc":"2.0","method":"event","params":{"type":"anchor_target_closed","surface_id":"s1"}}
```

### `device_recovered`

The GPU device was lost and all surfaces have been automatically recovered from the scene graph. No action is required from the client, but this event can be used for logging or to refresh custom draw content.

```json
{"jsonrpc":"2.0","method":"event","params":{"type":"device_recovered"}}
```

## Color Format

Colors are specified as CSS-style hex strings:

| Format | Example | Description |
|--------|---------|-------------|
| `#rgb` | `"#f00"` | 3-digit hex, expanded to 6 digits, alpha 255 |
| `#rrggbb` | `"#ff0000"` | 6-digit hex, alpha 255 |
| `#rrggbbaa` | `"#ff000080"` | 8-digit hex with explicit alpha |

The `#` prefix is optional. Alpha `00` is fully transparent, `ff` is fully opaque.

Default colors when omitted:
- Text color: `#ffffff` (white)
- Rect fill: `#ffffff` (white)
- Border color: none (no border)

## Error Codes

Standard JSON-RPC 2.0 error codes:

| Code | Name | Description |
|------|------|-------------|
| `-32700` | Parse error | Invalid JSON |
| `-32600` | Invalid request | Missing `jsonrpc: "2.0"` |
| `-32601` | Method not found | Unknown method name |
| `-32602` | Invalid params | Missing required params, wrong types, unknown surface ID |
| `-32603` | Internal error | Surface creation failure or other internal error |

**Example error response:**
```json
{"jsonrpc":"2.0","error":{"code":-32602,"message":"unknown surface_id: s99"},"id":5}
```

## Limitations

- **Custom draw not available:** The custom draw escape hatch (Direct2D drawing operations) requires in-process access to the GPU context. It is not exposed over the JSON-RPC protocol. Use the Rust or C API for custom draw.
- **Image paths must be local:** The `path` parameter in `set_image` and `icon_path` in tray methods must point to files accessible from the host process. URLs are not supported.
- **No batch operations:** Each request is processed individually. There is no transaction or batch mode.
- **Tray event ambiguity:** `tray_clicked` and `tray_menu_item_clicked` events do not include a tray ID. If multiple trays exist, events cannot be attributed to a specific tray.
- **Sequential processing:** Requests are processed one at a time in the order received. High-frequency updates (more than ~100/sec) may cause latency.
