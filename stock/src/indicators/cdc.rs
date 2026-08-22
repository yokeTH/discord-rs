use anyhow::{Error, bail, ensure};
use plotters::prelude::*;
use std::sync::Once;
use ta::Next;
use ta::indicators::ExponentialMovingAverage;
use tracing::{debug, info, instrument};

const FONT_NAME: &str = "JetBrainsMono";

static REGISTER_FONT: Once = Once::new();

/// Register JetBrainsMono with plotters' ab_glyph backend (once).
/// Falls back silently — plotters will use its default font.
fn ensure_font_registered() {
    REGISTER_FONT.call_once(|| {
        let dir = font_dir();
        // Try common file names for JetBrainsMono Nerd Font
        for name in &[
            "JetBrainsMonoNerdFont-Regular.ttf",
            "JetBrainsMonoNerdFontMono-Regular.ttf",
            "JetBrainsMono-Regular.ttf",
        ] {
            let path = std::path::Path::new(dir).join(name);
            if let Ok(bytes) = std::fs::read(&path) {
                let leaked: &'static [u8] = Box::leak(bytes.into_boxed_slice());
                if plotters::style::register_font(FONT_NAME, FontStyle::Normal, leaked).is_ok() {
                    info!(font = %path.display(), "registered font with plotters");
                    return;
                }
            }
        }
        debug!("JetBrainsMono not found, using plotters default font");
    });
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Signal {
    Buy,
    Sell,
    BullishZone,
    BearishZone,
    None,
}

#[instrument(name = "cdc_calculate", skip(closes), fields(n = closes.len()))]
pub fn calculate(closes: &[f64]) -> (Signal, Vec<f64>, Vec<f64>) {
    let mut ema12 = ExponentialMovingAverage::new(12).unwrap();
    let mut ema26 = ExponentialMovingAverage::new(26).unwrap();

    let mut ema12_vals = Vec::with_capacity(closes.len());
    let mut ema26_vals = Vec::with_capacity(closes.len());

    for &x in closes {
        ema12_vals.push(ema12.next(x));
        ema26_vals.push(ema26.next(x));
    }

    if closes.len() < 2 {
        debug!("not enough data for signal");
        return (Signal::None, ema12_vals, ema26_vals);
    }

    let c = closes.len() - 1;
    let p = closes.len() - 2;

    let prev_fast = ema12_vals[p];
    let prev_slow = ema26_vals[p];
    let cur_fast = ema12_vals[c];
    let cur_slow = ema26_vals[c];

    let signal = if prev_fast <= prev_slow && cur_fast > cur_slow {
        Signal::Buy
    } else if prev_fast >= prev_slow && cur_fast < cur_slow {
        Signal::Sell
    } else if cur_fast > cur_slow {
        Signal::BullishZone
    } else {
        Signal::BearishZone
    };

    info!(signal = ?signal, "signal computed");
    (signal, ema12_vals, ema26_vals)
}

/// Collect contiguous non-NaN segments from data.
fn nan_segments(data: &[f64]) -> Vec<Vec<(i32, f64)>> {
    let mut segments: Vec<Vec<(i32, f64)>> = Vec::new();
    let mut current: Vec<(i32, f64)> = Vec::new();

    for (i, &v) in data.iter().enumerate() {
        if v.is_nan() {
            if current.len() >= 2 {
                segments.push(std::mem::take(&mut current));
            } else {
                current.clear();
            }
        } else {
            current.push((i as i32, v));
        }
    }
    if current.len() >= 2 {
        segments.push(current);
    }

    segments
}

fn font_dir() -> &'static str {
    if std::path::Path::new("/fonts").exists() {
        "/fonts"
    } else {
        "fonts"
    }
}

#[instrument(
    name = "cdc_generate_chart",
    skip(prices, ema12, ema26, dates),
    fields(
        symbol = %symbol,
        prices = prices.len(),
        ema12 = ema12.len(),
        ema26 = ema26.len(),
        dates = dates.len()
    )
)]
pub fn generate_chart(
    symbol: &str,
    prices: &[f64],
    ema12: &[f64],
    ema26: &[f64],
    dates: &[String],
) -> Result<Vec<u8>, Error> {
    ensure!(!prices.is_empty(), "prices is empty");
    ensure!(
        prices.len() == ema12.len() && prices.len() == ema26.len() && prices.len() == dates.len(),
        "length mismatch: prices={}, ema12={}, ema26={}, dates={}",
        prices.len(),
        ema12.len(),
        ema26.len(),
        dates.len()
    );

    const LOOKBACK: usize = 90;
    const WIDTH: u32 = 1280;
    const HEIGHT: u32 = 720;

    let lookback = LOOKBACK.min(prices.len());
    let start_idx = prices.len().saturating_sub(lookback);

    let display_prices = &prices[start_idx..];
    let display_ema12 = &ema12[start_idx..];
    let display_ema26 = &ema26[start_idx..];
    let display_dates = &dates[start_idx..];

    let n = display_prices.len();
    if n == 0 {
        bail!("no data to display after slicing");
    }

    debug!(lookback = n, start_idx, "prepared display window");

    // Build bull/bear price arrays with NaN gaps
    let mut price_green = vec![f64::NAN; n];
    let mut price_red = vec![f64::NAN; n];

    let mut prev_bull = display_ema12[0] > display_ema26[0];
    if prev_bull {
        price_green[0] = display_prices[0];
    } else {
        price_red[0] = display_prices[0];
    }

    for i in 1..n {
        let bull = display_ema12[i] > display_ema26[i];

        if bull {
            price_green[i] = display_prices[i];
            if bull != prev_bull {
                price_green[i - 1] = display_prices[i - 1];
            }
        } else {
            price_red[i] = display_prices[i];
            if bull != prev_bull {
                price_red[i - 1] = display_prices[i - 1];
            }
        }

        prev_bull = bull;
    }

    let last_price = *display_prices.last().unwrap_or(&0.0);

    ensure_font_registered();

    // Compute y-axis range from all visible data
    let all_vals = display_prices
        .iter()
        .chain(display_ema12.iter())
        .chain(display_ema26.iter())
        .copied()
        .filter(|v| !v.is_nan());
    let y_min = all_vals.clone().fold(f64::INFINITY, f64::min);
    let y_max = all_vals.fold(f64::NEG_INFINITY, f64::max);
    let y_padding = (y_max - y_min) * 0.05;

    // Render SVG
    let mut svg_string = String::new();
    {
        let backend = SVGBackend::with_string(&mut svg_string, (WIDTH, HEIGHT));
        let root = backend.into_drawing_area();
        root.fill(&RGBColor(11, 12, 23))?;

        let title = format!("{} | ${:.2}", symbol.to_uppercase(), last_price);
        let font = (FONT_NAME, 14).into_font().color(&WHITE);
        root.draw_text(
            &title,
            &font,
            (WIDTH as i32 / 2 - (title.len() as i32 * 4), 10),
        )?;

        let label_style = (FONT_NAME, 12).into_font().color(&RGBColor(160, 160, 160));

        let mut chart = ChartBuilder::on(&root)
            .margin(15)
            .x_label_area_size(50)
            .y_label_area_size(70)
            .build_cartesian_2d(
                0i32..(n as i32 - 1),
                (y_min - y_padding)..(y_max + y_padding),
            )?;

        chart
            .configure_mesh()
            .disable_mesh()
            .x_labels(10)
            .y_labels(8)
            .x_label_formatter(&|idx| {
                let i = *idx as usize;
                if i < display_dates.len() {
                    display_dates[i].clone()
                } else {
                    String::new()
                }
            })
            .x_label_style(label_style.clone())
            .y_label_style(label_style)
            .axis_style(RGBColor(45, 47, 69))
            .draw()?;

        // Draw grid lines manually
        let grid_color = RGBColor(45, 47, 69).to_rgba();
        let grid_style = ShapeStyle {
            color: grid_color,
            filled: false,
            stroke_width: 1,
        };
        let y_step = ((y_max - y_min) / 8.0).max(0.01);
        let mut y_tick = y_min - y_padding;
        while y_tick <= y_max + y_padding {
            chart.draw_series(LineSeries::new(
                vec![(0i32, y_tick), (n as i32 - 1, y_tick)],
                grid_style,
            ))?;
            y_tick += y_step;
        }

        // Bull price (green)
        let green_style = ShapeStyle {
            color: RGBColor(0, 208, 132).to_rgba(),
            filled: false,
            stroke_width: 2,
        };
        for seg in nan_segments(&price_green) {
            chart.draw_series(LineSeries::new(seg, green_style))?;
        }
        // Bear price (red)
        let red_style = ShapeStyle {
            color: RGBColor(255, 77, 79).to_rgba(),
            filled: false,
            stroke_width: 2,
        };
        for seg in nan_segments(&price_red) {
            chart.draw_series(LineSeries::new(seg, red_style))?;
        }
        // EMA12 (blue)
        chart.draw_series(LineSeries::new(
            display_ema12
                .iter()
                .enumerate()
                .map(|(i, &v)| (i as i32, v)),
            ShapeStyle {
                color: RGBColor(0, 100, 255).to_rgba(),
                filled: false,
                stroke_width: 1,
            },
        ))?;
        // EMA26 (orange)
        chart.draw_series(LineSeries::new(
            display_ema26
                .iter()
                .enumerate()
                .map(|(i, &v)| (i as i32, v)),
            ShapeStyle {
                color: RGBColor(255, 100, 0).to_rgba(),
                filled: false,
                stroke_width: 1,
            },
        ))?;

        root.present()?;
    }

    // SVG → PNG via resvg
    let mut fontdb = resvg::usvg::fontdb::Database::new();
    fontdb.load_fonts_dir(font_dir());
    fontdb.load_system_fonts();
    fontdb.set_serif_family("JetBrainsMono Nerd Font");
    fontdb.set_sans_serif_family("JetBrainsMono Nerd Font");
    fontdb.set_monospace_family("JetBrainsMono Nerd Font");

    let options = resvg::usvg::Options {
        font_family: "JetBrainsMono Nerd Font".to_string(),
        fontdb: std::sync::Arc::new(fontdb),
        ..Default::default()
    };

    let tree = resvg::usvg::Tree::from_str(&svg_string, &options)?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(WIDTH, HEIGHT)
        .ok_or_else(|| anyhow::anyhow!("failed to create pixmap"))?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::default(),
        &mut pixmap.as_mut(),
    );
    let png_bytes = pixmap.encode_png()?;

    info!(bytes = png_bytes.len(), "chart rendered");
    Ok(png_bytes)
}
