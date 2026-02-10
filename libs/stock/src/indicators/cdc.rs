use anyhow::{Error, bail, ensure};
use charming::{
    Chart, ImageFormat, ImageRenderer,
    component::{Axis, Title},
    element::{AxisType, LineStyle, Symbol, TextStyle},
    series::Line,
};
use ta::Next;
use ta::indicators::ExponentialMovingAverage;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Signal {
    Buy,
    Sell,
    BullishZone,
    BearishZone,
    None,
}

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

    (signal, ema12_vals, ema26_vals)
}

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
    const WIDTH: u32 = 1200;
    const HEIGHT: u32 = 600;

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

    let chart = Chart::new()
        .background_color("#0b0c17")
        .title(
            Title::new()
                .text(format!("{} | ${:.2}", symbol.to_uppercase(), last_price))
                .left("center")
                .top("2%")
                .text_style(TextStyle::new().color("#ffffff").font_size(14)),
        )
        .x_axis(
            Axis::new()
                .type_(AxisType::Category)
                .data(display_dates.to_vec())
                .axis_label(
                    charming::element::AxisLabel::new()
                        .rotate(45)
                        .interval(9)
                        .color("#a0a0a0"),
                )
                .split_line(
                    charming::element::SplitLine::new()
                        .line_style(charming::element::LineStyle::new().color("#2d2f45")),
                ),
        )
        .y_axis(
            Axis::new()
                .type_(AxisType::Value)
                .scale(true)
                .axis_label(charming::element::AxisLabel::new().color("#a0a0a0"))
                .split_line(
                    charming::element::SplitLine::new()
                        .line_style(charming::element::LineStyle::new().color("#2d2f45")),
                ),
        )
        .series(
            Line::new()
                .name("Price (Bull)")
                .data(price_green)
                .symbol(Symbol::None)
                .line_style(LineStyle::new().width(2).color("#00d084")),
        )
        .series(
            Line::new()
                .name("Price (Bear)")
                .data(price_red)
                .symbol(Symbol::None)
                .line_style(LineStyle::new().width(2).color("#ff4d4f")),
        )
        .series(
            Line::new()
                .name("EMA12")
                .data(display_ema12.to_vec())
                .symbol(Symbol::None)
                .line_style(LineStyle::new().width(1).color("#0064FF")),
        )
        .series(
            Line::new()
                .name("EMA26")
                .data(display_ema26.to_vec())
                .symbol(Symbol::None)
                .line_style(LineStyle::new().width(1).color("#FF6400")),
        );

    let mut renderer = ImageRenderer::new(WIDTH, HEIGHT);
    let png_bytes = renderer.render_format(ImageFormat::Png, &chart)?;
    Ok(png_bytes)
}
