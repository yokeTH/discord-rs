//! Verifies every enum's wire representation: `as_str`, `Serialize`, and
//! `Deserialize` all agree on the same string (the SDK member name).

use webull::{
    Category, ComboType, Direction, ExchangeCode, ExpirationCycle, OptionType, OrderSide,
    OrderType, PositionIntent, TimeInForce, Timespan,
};

fn roundtrip<T>(value: T, wire: &str)
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug + Copy,
{
    let json = serde_json::to_string(&value).unwrap();
    assert_eq!(json, format!("\"{wire}\""), "serialize mismatch");
    let back: T = serde_json::from_str(&json).unwrap();
    assert_eq!(back, value, "deserialize round-trip mismatch");
}

#[test]
fn enum_wire_values() {
    roundtrip(Category::UsStock, "US_STOCK");
    roundtrip(Category::UsEvent, "US_EVENT");
    roundtrip(Category::HkFutures, "HK_FUTURES");
    roundtrip(OrderSide::Short, "SHORT");
    roundtrip(OrderType::StopLossLimit, "STOP_LOSS_LIMIT");
    roundtrip(OrderType::MarketOnClose, "MARKET_ON_CLOSE");
    roundtrip(TimeInForce::Ioc, "IOC");
    roundtrip(Timespan::Day, "D");
    roundtrip(Timespan::M60, "M60");
    roundtrip(Direction::B, "B");
    roundtrip(ExpirationCycle::Quarterly, "QUATERLY");
    roundtrip(ExchangeCode::Nyse, "NYSE");
    roundtrip(ComboType::Otoco, "OTOCO");
    roundtrip(PositionIntent::BuyToOpen, "BUY_TO_OPEN");
    roundtrip(OptionType::Call, "CALL");
}

#[test]
fn as_str_matches_serialization() {
    assert_eq!(Category::UsEtf.as_str(), "US_ETF");
    assert_eq!(Timespan::Week.as_str(), "W");
    assert_eq!(OrderType::TrailingStopLoss.as_str(), "TRAILING_STOP_LOSS");
    // Display uses the same wire value.
    assert_eq!(format!("{}", Category::UsStock), "US_STOCK");
}

#[test]
fn unknown_value_is_rejected() {
    assert!(serde_json::from_str::<OrderSide>("\"NOPE\"").is_err());
    assert!(serde_json::from_str::<Timespan>("\"X9\"").is_err());
}
