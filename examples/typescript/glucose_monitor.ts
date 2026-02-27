/**
 * CGM glucose monitor overlay — desktop HUD showing blood glucose from Nightscout.
 *
 * Connects to a Nightscout instance when NIGHTSCOUT_URL is set, otherwise
 * runs in simulated mode with random-walk glucose values.
 *
 * Setup:
 *   cd examples/typescript
 *   npm install winpane   (or: npm link ../../../bindings/node)
 *
 * Usage:
 *   npx tsx glucose_monitor.ts
 *
 * Environment variables (optional):
 *   NIGHTSCOUT_URL   — base URL of your Nightscout site
 *   NIGHTSCOUT_TOKEN — API token for authenticated access
 */
// ── winpane design tokens ──────────────────────────────────────
// Surface base: #121216  Glass: +e4  Solid: +ff  Muted: +f2
// Elevated:     #1c1c21  Interactive: #26262cff  Hover: #303038ff
// Border:       #ffffff12  Text:      #e8e8edff   Muted: #9494a0ff
// Accent:       #528bffff  Success:   #34d399ff   Warning: #fbbf24ff
// Danger:       #ef4444ff  Radius: 10/6 px
// ────────────────────────────────────────────────────────────────
import { WinPane } from "winpane";

// ── Types ──────────────────────────────────────────────────────

interface GlucoseReading {
  sgv: number;
  direction: string;
  timestamp: number; // Date.now() ms
}

// ── Helper functions ───────────────────────────────────────────

function directionToArrow(direction: string): string {
  const arrows: Record<string, string> = {
    DoubleUp: "⇈",
    SingleUp: "↑",
    FortyFiveUp: "↗",
    Flat: "→",
    FortyFiveDown: "↘",
    SingleDown: "↓",
    DoubleDown: "⇊",
  };
  return arrows[direction] ?? "?";
}

function bgColorForSgv(sgv: number): string {
  if (sgv >= 70 && sgv <= 180) return "#12281ee4"; // green-tinted
  if (sgv >= 181 && sgv <= 250) return "#282412e4"; // amber-tinted
  return "#281212e4"; // red-tinted (<70 or >250)
}

// Staleness measures time since fetch, not CGM reading time.
function stalenessText(timestampMs: number): { text: string; color: string } {
  const elapsed = Date.now() - timestampMs;
  const text = elapsed < 60000 ? "just now" : `${Math.floor(elapsed / 60000)} min ago`;
  const color = elapsed > 900000 ? "#ef4444ff" : "#9494a0cc";
  return { text, color };
}

async function fetchNightscout(url: string, token?: string): Promise<GlucoseReading | null> {
  try {
    let endpoint = `${url}/api/v1/entries/current.json`;
    if (token) endpoint += `?token=${token}`;
    const response = await fetch(endpoint);
    const data = await response.json();
    const entry = data[0];
    return {
      sgv: entry.sgv,
      direction: entry.direction,
      timestamp: Date.now(),
    };
  } catch {
    return null;
  }
}

function simulateReading(prevSgv: number): GlucoseReading {
  const delta = Math.trunc(Math.random() * 31 - 15);
  const sgv = Math.max(40, Math.min(350, prevSgv + delta));

  let direction: string;
  if (delta > 10) direction = "SingleUp";
  else if (delta > 5) direction = "FortyFiveUp";
  else if (delta > -5) direction = "Flat";
  else if (delta > -10) direction = "FortyFiveDown";
  else direction = "SingleDown";

  return { sgv, direction, timestamp: Date.now() };
}

// ── Main ───────────────────────────────────────────────────────

const wp = new WinPane();
const hud = wp.createHud({ width: 140, height: 65, x: 1760, y: 930 });
wp.setCaptureExcluded(hud, true);

// Initial bg
wp.setRect(hud, "bg", {
  x: 0, y: 0, width: 140, height: 65,
  fill: bgColorForSgv(120),
  cornerRadius: 10,
  borderColor: "#ffffff12",
  borderWidth: 1,
});
wp.show(hud);

const nightscoutUrl = process.env.NIGHTSCOUT_URL;
const nightscoutToken = process.env.NIGHTSCOUT_TOKEN;
const pollInterval = nightscoutUrl ? 5 * 60 * 1000 : 30 * 1000;

if (nightscoutUrl) {
  console.log("winpane glucose_monitor: polling Nightscout every 5 min.");
} else {
  console.log("winpane glucose_monitor: simulated mode (set NIGHTSCOUT_URL for live data).");
}
console.log("Press Ctrl+C to exit.");

let lastPoll = 0; // force immediate first poll
let currentReading: GlucoseReading = { sgv: 120, direction: "Flat", timestamp: Date.now() };

setInterval(async () => {
  const now = Date.now();
  if (now - lastPoll >= pollInterval) {
    if (nightscoutUrl) {
      const reading = await fetchNightscout(nightscoutUrl, nightscoutToken);
      if (reading) currentReading = reading;
    } else {
      currentReading = simulateReading(currentReading.sgv);
    }
    lastPoll = Date.now();
  }

  // Update bg
  wp.setRect(hud, "bg", {
    x: 0, y: 0, width: 140, height: 65,
    fill: bgColorForSgv(currentReading.sgv),
    cornerRadius: 10,
    borderColor: "#ffffff12",
    borderWidth: 1,
  });

  // Update reading
  const arrow = directionToArrow(currentReading.direction);
  wp.setText(hud, "reading", {
    text: `${currentReading.sgv} ${arrow}`,
    x: 12, y: 6,
    fontSize: 30,
    fontFamily: "Consolas",
    bold: true,
    color: "#e8e8edff",
  });

  // Update staleness
  const stale = stalenessText(currentReading.timestamp);
  wp.setText(hud, "staleness", {
    text: stale.text,
    x: 12, y: 42,
    fontSize: 12,
    color: stale.color,
  });
}, 1000);

// Graceful cleanup
process.on("SIGINT", () => {
  wp.destroy(hud);
  wp.close();
  process.exit(0);
});
