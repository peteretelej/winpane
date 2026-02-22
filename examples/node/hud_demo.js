/**
 * winpane HUD demo: Creates a floating overlay with text and a colored background.
 *
 * Usage:
 *   node hud_demo.js
 *
 * Requires: npm install winpane (or link to local build)
 */
const { WinPane } = require('winpane');

const wp = new WinPane();

// Create a HUD overlay
const hud = wp.createHud({ width: 400, height: 200, x: 100, y: 100 });

// Dark semi-transparent background
wp.setRect(hud, 'bg', {
  x: 0, y: 0, width: 400, height: 200,
  fill: '#1a1a2eee',
  cornerRadius: 8,
});

// Title text
wp.setText(hud, 'title', {
  text: 'Hello from Node.js!',
  x: 20, y: 20,
  fontSize: 24,
  color: '#ffffff',
});

// Subtitle
wp.setText(hud, 'subtitle', {
  text: 'winpane napi-rs addon demo',
  x: 20, y: 60,
  fontSize: 14,
  color: '#888888',
});

wp.show(hud);

// Update a counter for 5 seconds
let elapsed = 0;
const interval = setInterval(() => {
  elapsed += 100;
  wp.setText(hud, 'counter', {
    text: `Elapsed: ${(elapsed / 1000).toFixed(1)}s`,
    x: 20, y: 100,
    fontSize: 16,
    color: '#00ff88',
  });

  if (elapsed >= 5000) {
    clearInterval(interval);
    wp.destroy(hud);
    wp.close();
    console.log('Done!');
  }
}, 100);
