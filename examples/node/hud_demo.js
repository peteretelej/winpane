/**
 * winpane HUD demo: Creates a floating overlay with text and a colored background.
 *
 * Usage:
 *   node hud_demo.js
 *
 * Requires: npm install winpane (or link to local build)
 */
// ── winpane design tokens ──────────────────────────────────────
// Surface base: #121216  Glass: +e4  Solid: +ff  Muted: +f2
// Elevated:     #1c1c21  Interactive: #26262cff  Hover: #303038ff
// Border:       #ffffff12  Text:      #e8e8edff   Muted: #9494a0ff
// Accent:       #528bffff  Success:   #34d399ff   Warning: #fbbf24ff
// Danger:       #ef4444ff  Radius: 10/6 px
// ────────────────────────────────────────────────────────────────
const { WinPane } = require('winpane');

const wp = new WinPane();

// Create a HUD overlay
const hud = wp.createHud({ width: 400, height: 200, monitor: 0, anchor: 'top_left', margin: 40 });

// Dark semi-transparent background
wp.setRect(hud, 'bg', {
  x: 0, y: 0, width: 400, height: 200,
  fill: '#121216e4',
  cornerRadius: 10,
  borderColor: '#ffffff12',
  borderWidth: 1,
});

// Title text
wp.setText(hud, 'title', {
  text: 'Hello from Node.js!',
  x: 20, y: 20,
  fontSize: 16,
  color: '#e8e8ed',
});

// Subtitle
wp.setText(hud, 'subtitle', {
  text: 'winpane napi-rs addon demo',
  x: 20, y: 60,
  fontSize: 13,
  color: '#9494a0',
});

wp.show(hud);

// Update a counter for 5 seconds
let elapsed = 0;
const interval = setInterval(() => {
  elapsed += 100;
  wp.setText(hud, 'counter', {
    text: `Elapsed: ${(elapsed / 1000).toFixed(1)}s`,
    x: 20, y: 100,
    fontSize: 14,
    color: '#34d399',
  });

  if (elapsed >= 5000) {
    clearInterval(interval);
    wp.destroy(hud);
    wp.close();
    console.log('Done!');
  }
}, 100);
