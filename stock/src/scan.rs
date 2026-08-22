//! Symbol analysis: fetch price history, compute the CDC signal, chart it.
//!
//! This is the shared path behind `/stock graph`, `/stock trigger` and the daily
//! job. It deals in symbols and signals only — rendering the result for Discord
//! belongs to the caller.

use std::sync::Arc;

use anyhow::{Result, bail};
use chrono::Duration;
use futures::{Stream, StreamExt, stream};
use tracing::{debug, info, info_span, instrument, warn};
use tracing_futures::Instrument;

use crate::indicators::cdc::{Signal, calculate, generate_chart};
use crate::{PriceClient, Timeframe};

/// Days of daily bars pulled per symbol.
const HISTORY_DAYS: i64 = 300;
/// Cap on bars requested from the price API.
const BAR_LIMIT: usize = 365;
/// Symbols analysed concurrently by [`scan`].
const CONCURRENCY: usize = 8;

/// A symbol's computed signal, plus the series needed to chart it.
pub struct Analysis {
    pub symbol: String,
    pub signal: Signal,
    closes: Vec<f64>,
    ema12: Vec<f64>,
    ema26: Vec<f64>,
    dates: Vec<String>,
}

impl Analysis {
    /// Whether the signal is one worth alerting on.
    pub fn is_actionable(&self) -> bool {
        matches!(self.signal, Signal::Buy | Signal::Sell)
    }

    /// Render the price/EMA chart as a PNG. Plotting is CPU-bound, so it runs on
    /// a blocking thread rather than stalling the async runtime.
    pub async fn chart(&self) -> Result<Vec<u8>> {
        let (symbol, closes, ema12, ema26, dates) = (
            self.symbol.clone(),
            self.closes.clone(),
            self.ema12.clone(),
            self.ema26.clone(),
            self.dates.clone(),
        );
        tokio::task::spawn_blocking(move || {
            generate_chart(&symbol, &closes, &ema12, &ema26, &dates)
        })
        .await?
    }
}

/// Fetch `symbol`'s history and compute its signal. Does not chart — call
/// [`Analysis::chart`] for that, so scans only pay for symbols they alert on.
#[instrument(name = "analyze", skip(client), fields(symbol = %symbol))]
pub async fn analyze(client: &PriceClient, symbol: &str) -> Result<Analysis> {
    let bars = client
        .fetch_price(
            symbol,
            Duration::days(HISTORY_DAYS),
            Timeframe::Day1,
            BAR_LIMIT,
        )
        .await?;

    if bars.is_empty() {
        bail!("no price bars returned for {symbol}");
    }
    debug!(bars = bars.len(), "fetched price bars");

    let closes: Vec<f64> = bars.iter().map(|b| b.close).collect();
    let dates: Vec<String> = bars
        .iter()
        .map(|b| b.timestamp.format("%Y-%m-%d").to_string())
        .collect();

    let (signal, ema12, ema26) = calculate(&closes);
    info!(?signal, "calculated indicators");

    Ok(Analysis {
        symbol: symbol.to_string(),
        signal,
        closes,
        ema12,
        ema26,
        dates,
    })
}

/// An actionable signal with its chart already rendered.
pub struct Hit {
    pub symbol: String,
    pub signal: Signal,
    pub chart_png: Vec<u8>,
}

/// Analyse `symbols` concurrently, yielding a [`Hit`] per actionable signal as
/// it finishes. Symbols that fail to fetch, analyse, or chart are logged and
/// skipped, so one bad ticker never sinks the scan.
///
/// Boxed so callers can poll it (and `chunks` it) without pinning by hand.
pub fn scan(client: Arc<PriceClient>, symbols: Vec<String>) -> impl Stream<Item = Hit> + Unpin {
    stream::iter(symbols)
        .map(move |symbol| {
            let client = Arc::clone(&client);
            let span = info_span!("scan_symbol", symbol = %symbol);

            async move {
                let analysis = match analyze(&client, &symbol).await {
                    Ok(a) => a,
                    Err(e) => {
                        warn!(error = ?e, "analyze failed");
                        return None;
                    }
                };

                if !analysis.is_actionable() {
                    debug!(signal = ?analysis.signal, "no actionable signal");
                    return None;
                }

                match analysis.chart().await {
                    Ok(chart_png) => {
                        info!(bytes = chart_png.len(), "chart generated");
                        Some(Hit {
                            symbol: analysis.symbol,
                            signal: analysis.signal,
                            chart_png,
                        })
                    }
                    Err(e) => {
                        warn!(error = ?e, "chart generation failed");
                        None
                    }
                }
            }
            .instrument(span)
        })
        .buffer_unordered(CONCURRENCY)
        .filter_map(|hit| async move { hit })
        .boxed()
}
