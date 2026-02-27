/**
 * Clock overlay — floating digital clock showing current time.
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

// Bottom-right on 1080p, 20px inset
const hud = wp.createHud({ width: 150, height: 60, x: 1750, y: 1000 });

// Background card (Glass theme)
wp.setRect(hud, "bg", {
  x: 0, y: 0, width: 150, height: 60,
  fill: "#121216e4",
  cornerRadius: 10,
  borderColor: "#ffffff12",
  borderWidth: 1,
});

wp.show(hud);

console.log("winpane clock: ticking clock at bottom-right. Ctrl+C to exit.");

const DAYS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun",
                "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

setInterval(() => {
  const now = new Date();

  wp.setText(hud, "time", {
    text: now.toLocaleTimeString("en-US", { hour12: false }),
    x: 16, y: 8,
    fontSize: 28,
    fontFamily: "Consolas",
    bold: true,
    color: "#e8e8edff",
  });

  // Manual format to match Rust: "Thu Feb 27" (no comma, no zero-pad)
  const dateStr = `${DAYS[now.getDay()]} ${MONTHS[now.getMonth()]} ${now.getDate()}`;
  wp.setText(hud, "date", {
    text: dateStr,
    x: 16, y: 40,
    fontSize: 12,
    color: "#9494a0cc",
  });
}, 1000);

// Graceful cleanup on Ctrl+C
process.on("SIGINT", () => {
  wp.destroy(hud);
  wp.close();
  process.exit(0);
});
