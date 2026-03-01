/**
 * Clock overlay — floating draggable clock showing current time.
 *
 * Setup:
 *   cd examples/typescript
 *   npm install winpane   (or: npm link ../../../bindings/node)
 *
 * Usage:
 *   npx tsx clock_overlay.ts
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

// Bottom-right corner, 20px inset
const panel = wp.createPanel({ width: 150, height: 88, monitor: 0, anchor: 'bottom_right', margin: 20, draggable: true, dragHeight: 28, positionKey: 'ts_clock_overlay' });

// Background card (Glass theme)
wp.setRect(panel, "bg", {
  x: 0, y: 0, width: 150, height: 88,
  fill: "#121216e4",
  cornerRadius: 10,
  borderColor: "#ffffff12",
  borderWidth: 1,
});

// Title bar in drag region
wp.setRect(panel, "title_bg", {
  x: 0, y: 0, width: 150, height: 28,
  fill: "#1c1c21ff",
  cornerRadius: 10,
});
wp.setText(panel, "title", {
  text: "Clock",
  x: 8, y: 6,
  fontSize: 13,
  bold: true,
  color: "#9494a0ff",
});

wp.show(panel);

console.log("winpane clock: ticking clock at bottom-right. Ctrl+C to exit.");

const DAYS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun",
                "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

setInterval(() => {
  const now = new Date();

  wp.setText(panel, "time", {
    text: now.toLocaleTimeString("en-US", { hour12: false }),
    x: 16, y: 36,
    fontSize: 28,
    fontFamily: "Consolas",
    bold: true,
    color: "#e8e8edff",
  });

  // Manual format to match Rust: "Thu Feb 27" (no comma, no zero-pad)
  const dateStr = `${DAYS[now.getDay()]} ${MONTHS[now.getMonth()]} ${now.getDate()}`;
  wp.setText(panel, "date", {
    text: dateStr,
    x: 16, y: 68,
    fontSize: 12,
    color: "#9494a0cc",
  });
}, 1000);

// Graceful cleanup on Ctrl+C
process.on("SIGINT", () => {
  wp.destroy(panel);
  wp.close();
  process.exit(0);
});
