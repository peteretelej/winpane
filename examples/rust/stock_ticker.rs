//! Demo: Stock ticker overlay
//!
//! Displays a slim draggable horizontal bar with live stock prices and colored up/down
//! indicators. Polls Yahoo Finance every 60 seconds, falls back to simulated
//! random-walk prices when offline.
//!
//! Demonstrates: Panel creation, draggable surface, dynamic width, per-element coloring, HTTP
//! polling, environment-driven configuration, design system tokens.
//!
//! Run on Windows: cargo run -p winpane --example stock_ticker
//!
//! Environment variables (optional):
//!   TICKER_SYMBOLS — comma-separated stock symbols (default: AAPL,MSFT)

// ── winpane design tokens ──────────────────────────────────────
// Surface base:   rgb(18, 18, 22)  Glass: a=228  Solid: a=255  Muted: a=242
// Elevated:       rgb(28, 28, 33)  Interactive:  rgba(38, 38, 44, 255)
// Border:         rgba(255,255,255, 18)     Hover:       rgba(48, 48, 56, 255)
// Text primary:   rgba(232, 232, 237, 255)  Secondary:   rgba(148, 148, 160, 255)
// Accent:         rgba(82, 139, 255, 255)   Accent hover:rgba(110, 160, 255, 255)
// Success:        rgba(52, 211, 153, 255)   Warning:     rgba(251, 191, 36, 255)
// Danger:         rgba(239, 68, 68, 255)    Radius: 10/6 px
// ────────────────────────────────────────────────────────────────

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::thread;
use std::time::{Duration, Instant};

use winpane::{Color, Context, PanelConfig, Placement, RectElement, TextElement};

struct StockQuote {
    symbol: String,
    price: f64,
    change_pct: f64,
}

/// Returns green for non-negative change, red for negative.
fn price_color(change_pct: f64) -> Color {
    if change_pct >= 0.0 {
        Color::rgba(52, 211, 153, 255)
    } else {
        Color::rgba(239, 68, 68, 255)
    }
}

/// Returns "▲" for non-negative change, "▼" for negative.
fn direction_arrow(change_pct: f64) -> &'static str {
    if change_pct >= 0.0 { "▲" } else { "▼" }
}

/// Seed price for known symbols; defaults to 100.0.
fn seed_price(symbol: &str) -> f64 {
    match symbol {
        "AAPL" => 187.50,
        "MSFT" => 421.30,
        "GOOG" => 176.40,
        "AMZN" => 208.10,
        "TSLA" => 248.50,
        "NVDA" => 137.70,
        "META" => 585.20,
        _ => 100.00,
    }
}

/// Calculate surface width based on number of symbols, clamped to 200..=700.
fn calc_surface_width(num_symbols: usize) -> u32 {
    let per_symbol: u32 = 155;
    let base: u32 = 24;
    let raw = base + (num_symbols as u32) * per_symbol;
    raw.clamp(200, 700)
}

/// Fetches a single stock quote from Yahoo Finance v8 API.
fn fetch_single(symbol: &str) -> Option<StockQuote> {
    let url =
        format!("https://query1.finance.yahoo.com/v8/finance/chart/{symbol}?interval=1d&range=1d");
    let mut response = ureq::get(&url)
        .header("User-Agent", "winpane-example/1.0")
        .call()
        .ok()?;
    let body = response.body_mut().read_to_string().ok()?;
    let json: serde_json::Value = serde_json::from_str(&body).ok()?;

    let result = json["chart"]["result"].as_array()?.first()?;
    let meta = &result["meta"];
    let price = meta["regularMarketPrice"].as_f64()?;
    let prev_close = meta["chartPreviousClose"].as_f64()?;
    let change_pct = if prev_close > 0.0 {
        ((price - prev_close) / prev_close) * 100.0
    } else {
        0.0
    };

    Some(StockQuote {
        symbol: symbol.to_uppercase(),
        price,
        change_pct,
    })
}

/// Fetches quotes for all symbols; returns None for any that fail.
fn fetch_quotes(symbols: &[String]) -> Vec<Option<StockQuote>> {
    symbols.iter().map(|s| fetch_single(s)).collect()
}

/// Simulates quotes using a deterministic random walk seeded with time + symbol.
fn simulate_quotes(symbols: &[String], prev_prices: &[f64]) -> Vec<StockQuote> {
    symbols
        .iter()
        .zip(prev_prices.iter())
        .map(|(sym, prev)| {
            let mut hasher = DefaultHasher::new();
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
                .hash(&mut hasher);
            sym.hash(&mut hasher);
            prev.to_bits().hash(&mut hasher);
            let hash = hasher.finish();

            let pct = ((hash % 401) as f64 - 200.0) / 10000.0;
            let new_price = (prev * (1.0 + pct)).max(1.0);

            StockQuote {
                symbol: sym.clone(),
                price: (new_price * 100.0).round() / 100.0,
                change_pct: pct * 100.0,
            }
        })
        .collect()
}

/// Lays out the ticker elements horizontally across the Panel.
fn layout_ticker(panel: &winpane::Panel, quotes: &[StockQuote]) {
    let mut x: f32 = 12.0;
    for (i, q) in quotes.iter().enumerate() {
        let color = price_color(q.change_pct);
        let arrow = direction_arrow(q.change_pct);

        // Symbol label
        panel.set_text(
            &format!("sym_{i}"),
            TextElement {
                text: q.symbol.clone(),
                x,
                y: 32.0,
                font_size: 12.0,
                color: Color::rgba(148, 148, 160, 255),
                bold: true,
                ..Default::default()
            },
        );
        x += 45.0;

        // Price
        panel.set_text(
            &format!("price_{i}"),
            TextElement {
                text: format!("{:.2}", q.price),
                x,
                y: 30.0,
                font_size: 14.0,
                color,
                bold: true,
                font_family: Some("Consolas".to_string()),
                ..Default::default()
            },
        );
        x += 65.0;

        // Direction arrow
        panel.set_text(
            &format!("arrow_{i}"),
            TextElement {
                text: arrow.to_string(),
                x,
                y: 30.0,
                font_size: 14.0,
                color,
                font_family: Some("Consolas".to_string()),
                ..Default::default()
            },
        );
        x += 18.0;

        // Separator dot (omit after last symbol)
        if i + 1 < quotes.len() {
            panel.set_text(
                &format!("sep_{i}"),
                TextElement {
                    text: "·".to_string(),
                    x,
                    y: 30.0,
                    font_size: 14.0,
                    color: Color::rgba(148, 148, 160, 128),
                    ..Default::default()
                },
            );
            x += 16.0;
        }
    }
}

#[allow(clippy::print_stdout)]
fn main() -> Result<(), winpane::Error> {
    // Parse TICKER_SYMBOLS env (default: AAPL,MSFT)
    let symbols: Vec<String> = std::env::var("TICKER_SYMBOLS")
        .unwrap_or_else(|_| "AAPL,MSFT".to_string())
        .split(',')
        .map(|s| s.trim().to_uppercase())
        .filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '.'))
        .collect();

    let width = calc_surface_width(symbols.len());

    let ctx = Context::new()?;

    // Position top-right, 20px inset (assumes 1080p — 1920×1080)
    let panel = ctx.create_panel(PanelConfig {
        placement: Placement::Position {
            x: (1920 - width - 20) as i32,
            y: 20,
        },
        width,
        height: 56,
        draggable: true,
        drag_height: 24,
        position_key: None,
    })?;

    panel.set_capture_excluded(true);

    // Glass background with border
    panel.set_rect(
        "bg",
        RectElement {
            x: 0.0,
            y: 0.0,
            width: width as f32,
            height: 56.0,
            fill: Color::rgba(18, 18, 22, 228),
            corner_radius: 6.0,
            border_color: Some(Color::rgba(255, 255, 255, 18)),
            border_width: 1.0,
            interactive: false,
        },
    );

    // Compact grip dots in drag region
    panel.set_rect(
        "title_bg",
        RectElement {
            x: 0.0,
            y: 0.0,
            width: width as f32,
            height: 24.0,
            fill: Color::rgba(28, 28, 33, 255),
            corner_radius: 6.0,
            ..Default::default()
        },
    );
    panel.set_text(
        "grip",
        TextElement {
            x: (width / 2 - 6) as f32,
            y: 4.0,
            text: "⋮⋮".into(),
            font_size: 12.0,
            color: Color::rgba(148, 148, 160, 128),
            ..Default::default()
        },
    );

    panel.show();

    println!("winpane stock_ticker: tracking {}", symbols.join(", "));
    println!("Press Ctrl+C to exit.");

    // Initialize seed prices
    let mut prices: Vec<f64> = symbols.iter().map(|s| seed_price(s)).collect();

    let poll_interval = Duration::from_secs(60);
    let mut last_poll = Instant::now() - poll_interval; // force immediate first poll
    let mut printed_sim_msg = false;

    loop {
        if last_poll.elapsed() >= poll_interval {
            let live = fetch_quotes(&symbols);

            // Build final quotes: use live where available, simulate where not
            let mut quotes: Vec<StockQuote> = Vec::with_capacity(symbols.len());
            let mut any_live = false;
            let mut all_failed = true;

            for (i, maybe) in live.into_iter().enumerate() {
                if let Some(q) = maybe {
                    prices[i] = q.price;
                    any_live = true;
                    all_failed = false;
                    quotes.push(q);
                } else {
                    // Simulate just this symbol
                    let sim = simulate_quotes(&[symbols[i].clone()], &[prices[i]]);
                    if let Some(sq) = sim.into_iter().next() {
                        prices[i] = sq.price;
                        quotes.push(sq);
                    }
                }
            }

            if all_failed && !printed_sim_msg {
                println!(
                    "winpane stock_ticker: simulated mode (Yahoo Finance unreachable). Retrying each poll."
                );
                printed_sim_msg = true;
            } else if any_live && printed_sim_msg {
                println!("winpane stock_ticker: live data restored.");
                printed_sim_msg = false;
            }

            layout_ticker(&panel, &quotes);
            last_poll = Instant::now();
        }

        thread::sleep(Duration::from_secs(1));
    }
}
