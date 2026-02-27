/**
 * winpane interactive panel demo: Creates a panel with clickable buttons.
 * Logs click events to the console.
 *
 * Usage:
 *   node interactive_panel.js
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

// Create an interactive panel
const panel = wp.createPanel({
  width: 300, height: 250,
  x: 200, y: 200,
  draggable: true,
  dragHeight: 32,
});

// Title bar area
wp.setRect(panel, 'titlebar', {
  x: 0, y: 0, width: 300, height: 40,
  fill: '#1c1c21',
});

wp.setText(panel, 'title', {
  text: 'Interactive Panel',
  x: 15, y: 10,
  fontSize: 16,
  color: '#e8e8ed',
  bold: true,
});

// Background
wp.setRect(panel, 'bg', {
  x: 0, y: 40, width: 300, height: 210,
  fill: '#121216',
});

// Button 1
wp.setRect(panel, 'btn1', {
  x: 20, y: 60, width: 120, height: 40,
  fill: '#26262c',
  cornerRadius: 6,
  interactive: true,
  borderColor: '#ffffff17',
  borderWidth: 1,
});
wp.setText(panel, 'btn1_label', {
  text: 'Button 1',
  x: 45, y: 70,
  fontSize: 13,
  color: '#e8e8ed',
  interactive: true,
});

// Button 2
wp.setRect(panel, 'btn2', {
  x: 160, y: 60, width: 120, height: 40,
  fill: '#26262c',
  cornerRadius: 6,
  interactive: true,
  borderColor: '#ffffff17',
  borderWidth: 1,
});
wp.setText(panel, 'btn2_label', {
  text: 'Button 2',
  x: 185, y: 70,
  fontSize: 13,
  color: '#e8e8ed',
  interactive: true,
});

// Status text
wp.setText(panel, 'status', {
  text: 'Click a button...',
  x: 20, y: 130,
  fontSize: 11,
  color: '#60606b',
});

wp.show(panel);

let clickCount = 0;

// Poll for events
const interval = setInterval(() => {
  let event;
  while ((event = wp.pollEvent()) !== null) {
    if (event.eventType === 'element_clicked') {
      clickCount++;
      const name = event.key.replace('_label', '');
      console.log(`Clicked: ${name} (total: ${clickCount})`);
      wp.setText(panel, 'status', {
        text: `Last click: ${name} (total: ${clickCount})`,
        x: 20, y: 130,
        fontSize: 11,
        color: '#34d399',
      });
    }
  }
}, 16);

// Run for 30 seconds then exit
setTimeout(() => {
  clearInterval(interval);
  wp.destroy(panel);
  wp.close();
  console.log('Done!');
}, 30000);

console.log('Interactive panel running for 30 seconds. Click the buttons!');
