/**
 * Stock ticker overlay — slim draggable horizontal bar with live stock prices.
 *
 * Polls Yahoo Finance every 60 seconds with colored up/down indicators.
 * Falls back to simulated random-walk prices when offline.
 *
 * Setup:
 *   cd examples/typescript
 *   npm install winpane   (or: npm link ../../../bindings/node)
 *
 * Usage:
 *   npx tsx stock_ticker.ts
 *
 * Environment variables (optional):
 *   TICKER_SYMBOLS — comma-separated stock symbols (default: AAPL,MSFT)
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

interface StockQuote {
  symbol: string;
  price: number;
  changePct: number;
}

// ── Seed prices ────────────────────────────────────────────────

const SEED_PRICES: Record<string, number> = {
  AAPL: 187.50, MSFT: 421.30, GOOG: 176.40, AMZN: 208.10,
  TSLA: 248.50, NVDA: 137.70, META: 585.20,
};

function seedPrice(symbol: string): number {
  return SEED_PRICES[symbol] ?? 100;
}

// ── Helpers ────────────────────────────────────────────────────

function priceColor(changePct: number): string {
  return changePct >= 0 ? "#34d399ff" : "#ef4444ff";
}

function directionArrow(changePct: number): string {
  return changePct >= 0 ? "▲" : "▼";
}

function calcWidth(numSymbols: number): number {
  return Math.min(700, Math.max(200, 24 + 155 * numSymbols));
}

// ── Yahoo Finance fetch ────────────────────────────────────────

async function fetchQuote(symbol: string): Promise<StockQuote | null> {
  try {
    const url = `https://query1.finance.yahoo.com/v8/finance/chart/${symbol}?interval=1d&range=1d`;
    const resp = await fetch(url, {
      headers: { "User-Agent": "winpane-example/1.0" },
    });
    if (!resp.ok) return null;
    const data = await resp.json();
    const meta = data?.chart?.result?.[0]?.meta;
    if (!meta) return null;
    const price: number = meta.regularMarketPrice;
    const prevClose: number = meta.chartPreviousClose;
    const changePct = prevClose > 0 ? ((price - prevClose) / prevClose) * 100 : 0;
    return { symbol: symbol.toUpperCase(), price, changePct };
  } catch {
    return null;
  }
}

async function fetchQuotes(symbols: string[]): Promise<(StockQuote | null)[]> {
  return Promise.all(symbols.map(fetchQuote));
}

// ── Simulate ───────────────────────────────────────────────────

function simulateQuote(symbol: string, prevPrice: number): StockQuote {
  const pct = (Math.random() * 4 - 2) / 100; // -2%..+2%
  const price = Math.round(Math.max(1, prevPrice * (1 + pct)) * 100) / 100;
  return { symbol, price, changePct: pct * 100 };
}

// ── Layout ─────────────────────────────────────────────────────

function layoutTicker(wp: InstanceType<typeof WinPane>, panel: number, quotes: StockQuote[]): void {
  let x = 12; // left padding
  for (let i = 0; i < quotes.length; i++) {
    const q = quotes[i];
    const color = priceColor(q.changePct);

    wp.setText(panel, `sym_${i}`, {
      text: q.symbol,
      x, y: 32,
      fontSize: 12,
      bold: true,
      color: "#9494a0ff",
    });
    x += 45;

    wp.setText(panel, `price_${i}`, {
      text: q.price.toFixed(2),
      x, y: 30,
      fontSize: 14,
      fontFamily: "Consolas",
      bold: true,
      color,
    });
    x += 65;

    wp.setText(panel, `arrow_${i}`, {
      text: directionArrow(q.changePct),
      x, y: 30,
      fontSize: 14,
      fontFamily: "Consolas",
      color,
    });
    x += 18;

    if (i < quotes.length - 1) {
      wp.setText(panel, `sep_${i}`, {
        text: "·",
        x, y: 30,
        fontSize: 14,
        color: "#9494a080",
      });
      x += 16;
    }
  }
}

// ── Main ───────────────────────────────────────────────────────

const symbols = (process.env.TICKER_SYMBOLS ?? "AAPL,MSFT")
  .split(",")
  .map((s) => s.trim().toUpperCase())
  .filter((s) => s.length > 0 && /^[A-Z0-9.]+$/.test(s));

const width = calcWidth(symbols.length);
const wp = new WinPane();
// Assumes 1080p — adjust x for other resolutions
const panel = wp.createPanel({ width, height: 56, x: 1920 - width - 20, y: 20, draggable: true, dragHeight: 24 });
wp.setCaptureExcluded(panel, true);

wp.setRect(panel, "bg", {
  x: 0, y: 0, width, height: 56,
  fill: "#121216e4",
  cornerRadius: 6,
  borderColor: "#ffffff12",
  borderWidth: 1,
});
wp.show(panel);

// Compact grip dots in drag region
wp.setRect(panel, "title_bg", {
  x: 0, y: 0, width, height: 24,
  fill: "#1c1c21ff",
});
wp.setText(panel, "grip", {
  text: "⋮⋮",
  x: Math.floor(width / 2 - 6), y: 4,
  fontSize: 12,
  color: "#9494a080",
});

let prices = symbols.map(seedPrice);
let lastPoll = 0;
const POLL_INTERVAL = 60_000;
let simulated = false;

console.log(`winpane stock_ticker: tracking ${symbols.join(", ")}`);
console.log("Press Ctrl+C to exit.");

setInterval(async () => {
  const now = Date.now();
  if (now - lastPoll < POLL_INTERVAL) return;

  const results = await fetchQuotes(symbols);
  const allFailed = results.every((r) => r === null);

  if (allFailed && !simulated) {
    console.log("winpane stock_ticker: simulated mode (network unavailable).");
    simulated = true;
  } else if (!allFailed && simulated) {
    console.log("winpane stock_ticker: live data resumed.");
    simulated = false;
  }

  // Per-symbol fallback: use simulated only for symbols that failed
  const quotes: StockQuote[] = symbols.map((sym, i) => {
    return results[i] ?? simulateQuote(sym, prices[i]);
  });

  prices = quotes.map((q) => q.price);
  layoutTicker(wp, panel, quotes);
  lastPoll = Date.now();
}, 1000);

process.on("SIGINT", () => {
  wp.destroy(panel);
  wp.close();
  process.exit(0);
});
