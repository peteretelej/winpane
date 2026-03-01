/**
 * winpane backdrop demo: Mica and Acrylic backdrop effects from Node.js.
 *
 * Creates a panel with Mica backdrop, waits 3 seconds, switches to Acrylic,
 * waits 3 seconds, then fades out.
 *
 * Requires Windows 11 22H2+ for backdrop effects.
 *
 * Usage:
 *   node backdrop_demo.js
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
const panel = wp.createPanel({ width: 400, height: 300, monitor: 0, anchor: 'top_left', margin: 40 });

wp.setBackdrop(panel, 'mica');
wp.setText(panel, 'title', {
  text: 'Mica Backdrop from Node.js',
  x: 20, y: 20, fontSize: 16, color: '#e8e8ed',
});
wp.show(panel);

console.log('Showing Mica backdrop. Switching to Acrylic in 3 seconds...');

setTimeout(() => {
  wp.setBackdrop(panel, 'acrylic');
  wp.setText(panel, 'title', {
    text: 'Switched to Acrylic',
    x: 20, y: 20, fontSize: 16, color: '#e8e8ed',
  });
  console.log('Switched to Acrylic. Fading out in 3 seconds...');
}, 3000);

setTimeout(() => {
  console.log('Fading out...');
  wp.fadeOut(panel, 500);
}, 6000);

setTimeout(() => {
  wp.close();
  console.log('Done!');
}, 8000);
