# Visual Style

Default color palette, typography, and component patterns for winpane surfaces. See [design.md](../design.md) for architecture context.

## Color Palette

### Surface (RGB only — alpha comes from themes)

| Token | RGB | Hex | Role |
|-------|-----|-----|------|
| `surface.base` | `rgb(18, 18, 22)` | `#121216` | Primary background |
| `surface.elevated` | `rgb(28, 28, 33)` | `#1C1C21` | Cards, sections within a surface |
| `surface.interactive` | `rgb(38, 38, 44)` | `#26262C` | Button/control resting state |
| `surface.hover` | `rgb(48, 48, 56)` | `#303038` | Button/control hover state |
| `surface.active` | `rgb(55, 55, 64)` | `#373740` | Button/control pressed state |

### Border

| Token | RGBA | Hex | Role |
|-------|------|-----|------|
| `border.default` | `rgba(255, 255, 255, 18)` | `#FFFFFF12` | Standard surface border |
| `border.strong` | `rgba(255, 255, 255, 31)` | `#FFFFFF1F` | Emphasized border |
| `border.interactive` | `rgba(255, 255, 255, 23)` | `#FFFFFF17` | Button/control border |

### Text

| Token | RGBA | Hex | Role |
|-------|------|-----|------|
| `text.primary` | `rgba(232, 232, 237, 255)` | `#E8E8EDFF` | Headings, primary values |
| `text.secondary` | `rgba(148, 148, 160, 255)` | `#9494A0FF` | Labels, descriptions |
| `text.tertiary` | `rgba(96, 96, 107, 255)` | `#60606BFF` | Timestamps, hints |

### Accent

| Token | RGBA | Hex | Role |
|-------|------|-----|------|
| `accent.default` | `rgba(82, 139, 255, 255)` | `#528BFFFF` | Buttons, links, active indicators |
| `accent.hover` | `rgba(110, 160, 255, 255)` | `#6EA0FFFF` | Interactive hover state |
| `accent.muted` | `rgba(82, 139, 255, 48)` | `#528BFF30` | Subtle background tint |
| `accent.text` | `rgba(130, 170, 255, 255)` | `#82AAFFFF` | Accent-colored text |

### Semantic

| Token | RGBA | Hex | Role |
|-------|------|-----|------|
| `semantic.success` | `rgba(52, 211, 153, 255)` | `#34D399FF` | Healthy, positive, in-range |
| `semantic.warning` | `rgba(251, 191, 36, 255)` | `#FBBF24FF` | Attention, borderline |
| `semantic.danger` | `rgba(239, 68, 68, 255)` | `#EF4444FF` | Error, critical, out-of-range |

## Surface Themes

| Token | Glass (HUD) | Solid (Panel) | Muted (utility) |
|-------|-------------|---------------|------------------|
| `surface.base` | `rgba(18,18,22, 228)` `#121216E4` | `rgba(18,18,22, 255)` `#121216FF` | `rgba(18,18,22, 242)` `#121216F2` |
| `surface.elevated` | `rgba(28,28,33, 242)` `#1C1C21F2` | `rgba(28,28,33, 255)` `#1C1C21FF` | `rgba(28,28,33, 248)` `#1C1C21F8` |
| `border.default` | `rgba(255,255,255, 18)` `#FFFFFF12` | `rgba(255,255,255, 23)` `#FFFFFF17` | `rgba(255,255,255, 12)` `#FFFFFF0C` |

| Surface type | Theme | Why |
|-------------|-------|-----|
| HUD (click-through) | Glass | Ambient — transparency says "I'm not a window" |
| Panel (interactive) | Solid | Controls feel real on opaque background |
| Persistent utility | Muted | Visible all day without being loud |
| Tray popup | Solid | Brief, interactive — feels like a menu |
| Backdrop-enabled | Glass (low alpha) | Let Mica/Acrylic show through |

```rust
fill: Color::rgba(18, 18, 22, 228),   // glass
fill: Color::rgba(18, 18, 22, 255),   // solid
fill: Color::rgba(18, 18, 22, 242),   // muted
```

## Typography

**Segoe UI** — all UI text (system font, no install required). **Consolas** — numeric values, monospace data.

| Token | Size | Weight | Family | Usage |
|-------|------|--------|--------|-------|
| `display` | 32px | Bold | Consolas | Hero numbers |
| `heading` | 16px | Bold | Segoe UI | Surface titles |
| `body` | 13px | Regular | Segoe UI | Descriptions, status |
| `label` | 11px | Regular | Segoe UI | Timestamps, footnotes |
| `data` | 14px | Regular | Consolas | Inline data values |

## Spacing & Radius

Spacing uses a 4px grid: `xs`=4, `sm`=8, `md`=12 (standard padding), `lg`=16 (section gaps), `xl`=24.

Corner radius: `sm`=6px (buttons), `md`=10px (surfaces, cards), `lg`=12px (large displays).

## Component Patterns

**Surface Card** — outer shell of every surface (Glass shown; Solid: alpha=255/border=23; Muted: alpha=242/border=12):

```rust
surface.set_rect("bg", RectElement {
    x: 0.0, y: 0.0, width: W, height: H,
    fill: Color::rgba(18, 18, 22, 228),       // surface.base @ glass
    corner_radius: 10.0,                        // radius.md
    border_color: Some(Color::rgba(255, 255, 255, 18)), // border.default
    border_width: 1.0, ..Default::default()
});
```

```js
wp.setRect(id, "bg", {
    x: 0, y: 0, width: W, height: H,
    fill: "#121216e4", cornerRadius: 10, borderColor: "#ffffff12", borderWidth: 1,
});
```

**Separator** — thin line between content sections:

```rust
surface.set_rect("sep", RectElement {
    x: 12.0, y: SEP_Y, width: W - 24.0, height: 1.0,
    fill: Color::rgba(255, 255, 255, 18), ..Default::default() // border.default
});
```

**Title Bar (Panel)** — bold heading at `space.md` padding:

```rust
surface.set_text("title", TextElement {
    text: "Title".into(), x: 12.0, y: 10.0,
    font_size: 16.0, bold: true,              // heading scale
    color: Color::rgba(232, 232, 237, 255), ..Default::default() // text.primary
});
```

**Interactive Button** — resting state (swap to `surface.hover` + `border.strong` on hover):

```rust
panel.set_rect("btn", RectElement {
    x: X, y: Y, width: BTN_W, height: 36.0,
    fill: Color::rgba(38, 38, 44, 255),       // surface.interactive
    corner_radius: 6.0,                        // radius.sm
    border_color: Some(Color::rgba(255, 255, 255, 23)), // border.interactive
    border_width: 1.0, interactive: true,
});
```

**Status Text** — semantic-colored inline indicator:

```rust
surface.set_text("status", TextElement {
    text: "● Connected".into(), x: X, y: Y,
    font_size: 11.0,                              // label scale
    color: Color::rgba(52, 211, 153, 255), ..Default::default() // semantic.success
});
```

**Data Row** — label (`text.secondary`) + value (`text.primary`, Consolas):

```rust
surface.set_text("cpu_label", TextElement {
    text: "CPU".into(), x: 12.0, y: ROW_Y,
    font_size: 13.0, color: Color::rgba(148, 148, 160, 255), ..Default::default()
});
surface.set_text("cpu_val", TextElement {
    text: "42%".into(), x: VALUE_X, y: ROW_Y,
    font_size: 14.0, color: Color::rgba(232, 232, 237, 255),
    font_family: Some("Consolas".into()), ..Default::default()
});
```

**Section Card** — elevated area within a surface:

```rust
surface.set_rect("section_bg", RectElement {
    x: 8.0, y: SEC_Y, width: W - 16.0, height: SEC_H,
    fill: Color::rgba(28, 28, 33, 255),       // surface.elevated
    corner_radius: 6.0, ..Default::default()  // radius.sm
});
```

## Example Comment Block

```rust
// ── winpane design tokens ──────────────────────────────────────
// Surface base:   rgb(18, 18, 22)  Glass: a=228  Solid: a=255  Muted: a=242
// Elevated:       rgb(28, 28, 33)  Interactive:  rgba(38, 38, 44, 255)
// Border:         rgba(255,255,255, 18)     Hover:       rgba(48, 48, 56, 255)
// Text primary:   rgba(232, 232, 237, 255)  Secondary:   rgba(148, 148, 160, 255)
// Accent:         rgba(82, 139, 255, 255)   Accent hover:rgba(110, 160, 255, 255)
// Success:        rgba(52, 211, 153, 255)   Warning:     rgba(251, 191, 36, 255)
// Danger:         rgba(239, 68, 68, 255)    Radius: 10/6 px
// ────────────────────────────────────────────────────────────────
```

```js
// ── winpane design tokens ──────────────────────────────────────
// Surface base: #121216  Glass: +e4  Solid: +ff  Muted: +f2
// Elevated:     #1c1c21  Interactive: #26262cff  Hover: #303038ff
// Border:       #ffffff12  Text:      #e8e8edff   Muted: #9494a0ff
// Accent:       #528bffff  Success:   #34d399ff   Warning: #fbbf24ff
// Danger:       #ef4444ff  Radius: 10/6 px
// ────────────────────────────────────────────────────────────────
```
