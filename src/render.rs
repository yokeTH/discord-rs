//! Discord rendering for stock signals, shared by `/stock graph`,
//! `/stock trigger` and the daily job so they can't drift apart.

use serenity::all::{CreateAttachment, CreateEmbed};
use stock::Hit;
use stock::indicators::cdc::Signal;

/// Embeds per message. Discord rejects more than 10.
pub const BATCH_SIZE: usize = 10;

/// Embed colour for a signal: green bullish, red bearish, white neutral.
fn signal_color(signal: Signal) -> u32 {
    match signal {
        Signal::Buy | Signal::BullishZone => 0x00ff00,
        Signal::Sell | Signal::BearishZone => 0xff0000,
        Signal::None => 0xffffff,
    }
}

/// Chart embed plus the attachment it references.
pub fn signal_embed(
    symbol: &str,
    signal: Signal,
    chart_png: Vec<u8>,
) -> (CreateEmbed, CreateAttachment) {
    let filename = format!("{symbol}_chart.png");
    let embed = CreateEmbed::default()
        .title(format!("{} Analysis", symbol.to_uppercase()))
        .description(format!("Current Signal: {signal:?}"))
        .color(signal_color(signal))
        .image(format!("attachment://{filename}"));

    (embed, CreateAttachment::bytes(chart_png, filename))
}

/// Split a batch of scan hits into the parallel embed/attachment lists that
/// both the command reply and the daily message builder expect.
pub fn render_batch(hits: Vec<Hit>) -> (Vec<CreateEmbed>, Vec<CreateAttachment>) {
    hits.into_iter()
        .map(|hit| signal_embed(&hit.symbol, hit.signal, hit.chart_png))
        .unzip()
}
