/**
 * Sticky notes — tray app with floating note panel.
 *
 * Left-click the tray icon to toggle note visibility.
 * Right-click for a context menu with Show/Hide/Quit.
 * Demonstrates Tray + Panel composition.
 *
 * Setup:
 *   cd examples/typescript
 *   npm install winpane   (or: npm link ../../../bindings/node)
 *
 * Usage:
 *   npx tsx sticky_notes.ts
 */
// ── winpane design tokens ──────────────────────────────────────
// Surface base: #121216  Glass: +e4  Solid: +ff  Muted: +f2
// Elevated:     #1c1c21  Interactive: #26262cff  Hover: #303038ff
// Border:       #ffffff12  Text:      #e8e8edff   Muted: #9494a0ff
// Accent:       #528bffff  Success:   #34d399ff   Warning: #fbbf24ff
// Danger:       #ef4444ff  Radius: 10/6 px
// ────────────────────────────────────────────────────────────────
import { WinPane } from "winpane";

const wp = new WinPane();

// ── Tray ───────────────────────────────────────────────────────
const tray = wp.createTray({ tooltip: "Sticky Notes" });
wp.setMenu(tray, [
  { id: 1, label: "Show" },
  { id: 2, label: "Hide" },
  { id: 99, label: "Quit" },
]);

// ── Panel: 240×160, draggable title bar ────────────────────────
const panel = wp.createPanel({
  width: 240, height: 160, x: 200, y: 200,
  draggable: true, dragHeight: 28,
});
wp.setBackdrop(panel, "mica");

// Background (glass fallback — visible on Win10, tints Mica on Win11)
wp.setRect(panel, "bg", {
  x: 0, y: 0, width: 240, height: 160,
  fill: "#121216e4",
  cornerRadius: 10,
  borderColor: "#ffffff12",
  borderWidth: 1,
});

// Title
wp.setText(panel, "title", {
  text: "Notes", x: 12, y: 8,
  fontSize: 16, bold: true,
  color: "#e8e8edff",
});

// Close button — transparent hit-target rect
wp.setRect(panel, "close_btn", {
  x: 214, y: 4, width: 20, height: 20,
  fill: "#00000000",
  cornerRadius: 4,
  interactive: true,
});

// Close button — × glyph
wp.setText(panel, "close_x", {
  text: "×", x: 219, y: 4,
  fontSize: 14,
  color: "#9494a0ff",
});

// Separator
wp.setRect(panel, "sep", {
  x: 12, y: 28, width: 216, height: 1,
  fill: "#ffffff12",
});

// Note lines
const notes = [
  { key: "note_1", y: 38, text: "Remember to review PR" },
  { key: "note_2", y: 56, text: "Deploy staging at 3pm" },
  { key: "note_3", y: 74, text: "Call dentist" },
  { key: "note_4", y: 92, text: "Buy groceries" },
  { key: "note_5", y: 110, text: "Update dependencies" },
];
for (const n of notes) {
  wp.setText(panel, n.key, {
    text: n.text, x: 16, y: n.y,
    fontSize: 13, color: "#e8e8edff",
  });
}

// Wire popup + show panel on launch
wp.setPopup(tray, panel);
wp.show(panel);

console.log("Sticky Notes: tray icon created. Left-click to toggle, right-click for menu.");

// ── Event loop ─────────────────────────────────────────────────
setInterval(() => {
  let event = wp.pollEvent();
  while (event) {
    switch (event.eventType) {
      case "tray_clicked":
        console.log(`Tray clicked: ${event.button}`);
        break;
      case "tray_menu_item_clicked":
        if (event.itemId === 1) wp.show(panel);
        else if (event.itemId === 2) wp.hide(panel);
        else if (event.itemId === 99) {
          wp.destroy(tray);
          wp.destroy(panel);
          wp.close();
          process.exit(0);
        }
        break;
      case "element_clicked":
        if (event.key === "close_btn") wp.hide(panel);
        break;
      case "element_hovered":
        if (event.key === "close_btn") {
          wp.setRect(panel, "close_btn", {
            x: 214, y: 4, width: 20, height: 20,
            fill: "#ef444450",
            cornerRadius: 4,
            interactive: true,
          });
        }
        break;
      case "element_left":
        if (event.key === "close_btn") {
          wp.setRect(panel, "close_btn", {
            x: 214, y: 4, width: 20, height: 20,
            fill: "#00000000",
            cornerRadius: 4,
            interactive: true,
          });
        }
        break;
    }
    event = wp.pollEvent();
  }
}, 16);

// ── Cleanup ────────────────────────────────────────────────────
process.on("SIGINT", () => {
  wp.destroy(tray);
  wp.destroy(panel);
  wp.close();
  process.exit(0);
});
