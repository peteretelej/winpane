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
const { WinPane } = require('winpane');

const wp = new WinPane();
const panel = wp.createPanel({ width: 400, height: 300, x: 200, y: 200 });

wp.setBackdrop(panel, 'mica');
wp.setText(panel, 'title', {
  text: 'Mica Backdrop from Node.js',
  x: 20, y: 20, fontSize: 20, color: '#ffffff',
});
wp.show(panel);

console.log('Showing Mica backdrop. Switching to Acrylic in 3 seconds...');

setTimeout(() => {
  wp.setBackdrop(panel, 'acrylic');
  wp.setText(panel, 'title', {
    text: 'Switched to Acrylic',
    x: 20, y: 20, fontSize: 20, color: '#ffffff',
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
