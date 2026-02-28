/**
 * Pomodoro timer overlay — interactive panel with work/break countdown.
 *
 * 25-minute work sessions and 5-minute breaks with start/pause,
 * break toggle, and reset buttons. Timer color transitions through
 * green, yellow, and pulsing red as time runs out.
 *
 * Setup:
 *   cd examples/typescript
 *   npm install winpane   (or: npm link ../../../bindings/node)
 *
 * Usage:
 *   npx tsx pomodoro.ts
 */
// ── winpane design tokens ──────────────────────────────────────
// Surface base: #121216  Glass: +e4  Solid: +ff  Muted: +f2
// Elevated:     #1c1c21  Interactive: #26262cff  Hover: #303038ff
// Border:       #ffffff12  Text:      #e8e8edff   Muted: #9494a0ff
// Accent:       #528bffff  Success:   #34d399ff   Warning: #fbbf24ff
// Danger:       #ef4444ff  Radius: 10/6 px
// ────────────────────────────────────────────────────────────────
import { WinPane } from "winpane";

// ── State ──────────────────────────────────────────────────────

type TimerState = "idle" | "running" | "paused";
type TimerMode = "work" | "break";

let state: TimerState = "idle";
let mode: TimerMode = "work";
let remainingSecs = 25 * 60;
let lastTickMs = 0;

const WORK_SECS = 25 * 60;
const BREAK_SECS = 5 * 60;

// ── Helpers ────────────────────────────────────────────────────

function formatTime(totalSecs: number): string {
  const mins = Math.floor(totalSecs / 60);
  const secs = totalSecs % 60;
  return `${String(mins).padStart(2, "0")}:${String(secs).padStart(2, "0")}`;
}

function timerColor(remaining: number, st: TimerState, elapsedMs: number): string {
  if (st === "idle") return "#e8e8edff";
  if (st === "paused") return "#9494a0ff";
  if (remaining > 30) return "#34d399ff";
  if (remaining > 10) return "#fbbf24ff";
  const pulse = Math.sin((elapsedMs % 1000) / 1000 * Math.PI * 2);
  const alpha = Math.round(191.5 + 63.5 * pulse);
  return `#ef4444${alpha.toString(16).padStart(2, "0")}`;
}

function normalRect() {
  return { fill: "#26262cff", borderColor: "#ffffff17" };
}

function hoverRect() {
  return { fill: "#303038ff", borderColor: "#ffffff1f" };
}

// ── Surface ────────────────────────────────────────────────────

const wp = new WinPane();
const panel = wp.createPanel({
  width: 240, height: 140,
  x: 840, y: 450,
  draggable: true, dragHeight: 28,
});

// Background
wp.setRect(panel, "bg", {
  x: 0, y: 0, width: 240, height: 140,
  fill: "#121216ff",
  cornerRadius: 10,
  borderColor: "#ffffff17",
  borderWidth: 1,
});

// Title
wp.setText(panel, "title", {
  text: "Work",
  x: 12, y: 8,
  fontSize: 16,
  bold: true,
  color: "#e8e8edff",
});

// Close button
wp.setRect(panel, "close_btn", {
  x: 214, y: 4, width: 20, height: 20,
  fill: "#00000000",
  cornerRadius: 4,
  interactive: true,
});
wp.setText(panel, "close_x", {
  text: "×",
  x: 219, y: 4,
  fontSize: 14,
  color: "#9494a0ff",
});

// Separator
wp.setRect(panel, "sep", {
  x: 12, y: 28, width: 216, height: 1,
  fill: "#ffffff12",
});

// Timer display
wp.setText(panel, "timer", {
  text: formatTime(remainingSecs),
  x: 78, y: 38,
  fontSize: 36,
  fontFamily: "Consolas",
  bold: true,
  color: "#e8e8edff",
});

// Start/Pause button
wp.setRect(panel, "btn_start", {
  x: 12, y: 92, width: 66, height: 34,
  ...normalRect(),
  cornerRadius: 6,
  borderWidth: 1,
  interactive: true,
});
wp.setText(panel, "btn_start_text", {
  text: "Start",
  x: 24, y: 100,
  fontSize: 13,
  color: "#e8e8edff",
});

// Break button
wp.setRect(panel, "btn_break", {
  x: 86, y: 92, width: 66, height: 34,
  ...normalRect(),
  cornerRadius: 6,
  borderWidth: 1,
  interactive: true,
});
wp.setText(panel, "btn_break_text", {
  text: "Break",
  x: 98, y: 100,
  fontSize: 13,
  color: "#e8e8edff",
});

// Reset button
wp.setRect(panel, "btn_reset", {
  x: 160, y: 92, width: 66, height: 34,
  ...normalRect(),
  cornerRadius: 6,
  borderWidth: 1,
  interactive: true,
});
wp.setText(panel, "btn_reset_text", {
  text: "Reset",
  x: 172, y: 100,
  fontSize: 13,
  color: "#e8e8edff",
});

// ── Timer tick ─────────────────────────────────────────────────

function tickTimer(): void {
  if (state !== "running") return;
  const now = Date.now();
  if (lastTickMs === 0) { lastTickMs = now; return; }
  if (now - lastTickMs >= 1000) {
    remainingSecs--;
    lastTickMs = now;
    if (remainingSecs <= 0) {
      remainingSecs = 0;
      state = "idle";
      lastTickMs = 0;
    }
  }
}

// ── Display update ─────────────────────────────────────────────

function updateDisplay(): void {
  const elapsedMs = Date.now();
  const color = timerColor(remainingSecs, state, elapsedMs);
  wp.setText(panel, "timer", {
    text: formatTime(remainingSecs), x: 78, y: 38,
    fontSize: 36, fontFamily: "Consolas", bold: true,
    color,
  });
  wp.setText(panel, "title", {
    text: mode === "work" ? "Work" : "Break",
    x: 12, y: 8, fontSize: 16, bold: true,
    color: "#e8e8edff",
  });
  wp.setText(panel, "btn_start_text", {
    text: state === "running" ? "Pause" : "Start",
    x: 24, y: 100, fontSize: 13, color: "#e8e8edff",
  });
}

// ── Event loop ─────────────────────────────────────────────────

wp.show(panel);
console.log("Pomodoro timer visible at (840, 450). Press Ctrl+C to exit.");

setInterval(() => {
  let event = wp.pollEvent();
  while (event) {
    if (event.surfaceId === panel) {
      if (event.eventType === "element_clicked") {
        if (event.key === "btn_start") {
          if (state === "running") {
            state = "paused";
          } else {
            if (remainingSecs === 0) {
              remainingSecs = mode === "work" ? WORK_SECS : BREAK_SECS;
            }
            state = "running";
            lastTickMs = 0;
          }
        } else if (event.key === "btn_break") {
          mode = "break";
          remainingSecs = BREAK_SECS;
          state = "idle";
        } else if (event.key === "btn_reset") {
          remainingSecs = mode === "work" ? WORK_SECS : BREAK_SECS;
          state = "idle";
          lastTickMs = 0;
        } else if (event.key === "close_btn") {
          wp.destroy(panel);
          wp.close();
          process.exit(0);
        }
      } else if (event.eventType === "element_hovered") {
        if (event.key === "btn_start") {
          wp.setRect(panel, "btn_start", {
            x: 12, y: 92, width: 66, height: 34,
            ...hoverRect(), cornerRadius: 6, borderWidth: 1, interactive: true,
          });
        } else if (event.key === "btn_break") {
          wp.setRect(panel, "btn_break", {
            x: 86, y: 92, width: 66, height: 34,
            ...hoverRect(), cornerRadius: 6, borderWidth: 1, interactive: true,
          });
        } else if (event.key === "btn_reset") {
          wp.setRect(panel, "btn_reset", {
            x: 160, y: 92, width: 66, height: 34,
            ...hoverRect(), cornerRadius: 6, borderWidth: 1, interactive: true,
          });
        } else if (event.key === "close_btn") {
          wp.setRect(panel, "close_btn", {
            x: 214, y: 4, width: 20, height: 20,
            fill: "#ef444450", cornerRadius: 4, interactive: true,
          });
        }
      } else if (event.eventType === "element_left") {
        if (event.key === "btn_start") {
          wp.setRect(panel, "btn_start", {
            x: 12, y: 92, width: 66, height: 34,
            ...normalRect(), cornerRadius: 6, borderWidth: 1, interactive: true,
          });
        } else if (event.key === "btn_break") {
          wp.setRect(panel, "btn_break", {
            x: 86, y: 92, width: 66, height: 34,
            ...normalRect(), cornerRadius: 6, borderWidth: 1, interactive: true,
          });
        } else if (event.key === "btn_reset") {
          wp.setRect(panel, "btn_reset", {
            x: 160, y: 92, width: 66, height: 34,
            ...normalRect(), cornerRadius: 6, borderWidth: 1, interactive: true,
          });
        } else if (event.key === "close_btn") {
          wp.setRect(panel, "close_btn", {
            x: 214, y: 4, width: 20, height: 20,
            fill: "#00000000", cornerRadius: 4, interactive: true,
          });
        }
      }
    }
    event = wp.pollEvent();
  }
  tickTimer();
  updateDisplay();
}, 16);

// ── Cleanup ────────────────────────────────────────────────────

process.on("SIGINT", () => {
  wp.destroy(panel);
  wp.close();
  process.exit(0);
});
