//! Serde round-trip tests against JSON matching the Webull API reference docs.

use webull::{
    Account, Bar, Category, EntrustType, NewOrder, OrderRecord, OrderRequest, OrderSide, OrderType,
    Position, Quote, Snapshot, TimeInForce, TradingSession,
};

#[test]
fn deserializes_snapshot_array() {
    let json = r#"[
      {
        "instrument_id": "913255275",
        "symbol": "AAPL",
        "price": "150.25",
        "pre_close": "149.00",
        "change": "1.25",
        "change_ratio": "0.0084",
        "open": "149.50",
        "close": "150.25",
        "high": "151.00",
        "low": "149.10",
        "volume": "1000000",
        "last_trade_time": 1640688000000,
        "ask": "150.30",
        "ask_size": "100"
      }
    ]"#;
    let snaps: Vec<Snapshot> = serde_json::from_str(json).unwrap();
    assert_eq!(snaps.len(), 1);
    assert_eq!(snaps[0].symbol, "AAPL");
    assert_eq!(snaps[0].price, "150.25");
    assert_eq!(snaps[0].last_trade_time, Some(1640688000000));
    // Fields absent from the payload default cleanly.
    assert_eq!(snaps[0].ovn_price, None);
}

#[test]
fn deserializes_quote() {
    let json = r#"{
      "symbol": "F",
      "instrument_id": "913255275",
      "quote_time": "1640688000000",
      "asks": [{"price":"13.9","size":"5","order":[{"mpid":"NSDQ","size":"5"}],"broker":[{"bid":"1","name":"2"}]}],
      "bids": [{"price":"13.8","size":"3"}]
    }"#;
    let quote: Quote = serde_json::from_str(json).unwrap();
    assert_eq!(quote.symbol, "F");
    assert_eq!(quote.asks[0].price, "13.9");
    assert_eq!(quote.asks[0].order[0].mpid, "NSDQ");
    // Missing order/broker arrays default to empty.
    assert!(quote.bids[0].order.is_empty());
}

#[test]
fn quote_time_accepts_number() {
    // UAT returns quote_time as a JSON number; the docs show a string.
    let json =
        r#"{"symbol":"F","instrument_id":"1","quote_time":1784747968460,"asks":[],"bids":[]}"#;
    let quote: Quote = serde_json::from_str(json).unwrap();
    assert_eq!(quote.quote_time, "1784747968460");
}

#[test]
fn deserializes_bars_with_camelcase_ticker_id() {
    let json = r#"[
      {"tickerId":"913256135","symbol":"AAPL","time":"2021-12-28T09:00:09.945+0000",
       "open":"100","close":"101","high":"105","low":"99","volume":"100000","trading_session":"RTH"}
    ]"#;
    let bars: Vec<Bar> = serde_json::from_str(json).unwrap();
    assert_eq!(bars[0].ticker_id, "913256135");
    assert_eq!(bars[0].close, "101");
    assert_eq!(bars[0].trading_session.as_deref(), Some("RTH"));
}

#[test]
fn deserializes_account_list() {
    let json = r#"[
      {"user_id":"1010046954","account_id":"1090369621183459328","account_number":"CSG0285510",
       "account_type":"CASH","account_class":"INDIVIDUAL_CASH","account_label":"Individual Cash"}
    ]"#;
    let accounts: Vec<Account> = serde_json::from_str(json).unwrap();
    assert_eq!(accounts[0].account_id, "1090369621183459328");
    assert_eq!(accounts[0].account_type, "CASH");
}

#[test]
fn deserializes_positions() {
    let json = r#"[
      {"position_id":"N4I4SIM8TJF38KN2TAA0QVVNE9","currency":"USD","quantity":"1","symbol":"AAPL",
       "instrument_type":"EQUITY","last_price":"10.0","cost_price":"11.12","unrealized_profit_loss":"0.08"}
    ]"#;
    let positions: Vec<Position> = serde_json::from_str(json).unwrap();
    assert_eq!(positions[0].symbol, "AAPL");
    assert_eq!(positions[0].cost_price, "11.12");
}

#[test]
fn deserializes_order_history_with_enums() {
    let json = r#"[
      {
        "client_order_id": "THI82O5JB7MQ2K76LL5FSDS2CB",
        "combo_type": "NORMAL",
        "orders": [
          {
            "client_order_id": "THI82O5JB7MQ2K76LL5FSDS2CB",
            "order_id": "0352U72LQI6DT0KF41GK000000",
            "symbol": "AAPL",
            "side": "BUY",
            "status": "SUBMITTED",
            "order_type": "MARKET",
            "instrument_type": "EQUITY",
            "support_trading_session": "CORE",
            "time_in_force": "DAY",
            "total_quantity": "1",
            "filled_quantity": "1",
            "filled_price": "11.0"
          }
        ]
      }
    ]"#;
    let records: Vec<OrderRecord> = serde_json::from_str(json).unwrap();
    let leg = &records[0].orders[0];
    assert_eq!(leg.side, Some(OrderSide::Buy));
    assert_eq!(leg.status, Some(webull::OrderStatus::Submitted));
    assert_eq!(leg.order_type, Some(OrderType::Market));
    assert_eq!(leg.support_trading_session, Some(TradingSession::Core));
}

#[test]
fn serializes_place_order_request_compactly() {
    let mut order = NewOrder::new(
        "0KGOHL4PR2SLC0DKIND4TI0002",
        "AAPL",
        OrderSide::Buy,
        OrderType::Market,
        EntrustType::Qty,
        TimeInForce::Day,
    );
    order.support_trading_session = Some(TradingSession::Core);
    order.quantity = Some("1".to_string());

    let req = OrderRequest {
        account_id: "1090369621183459328".to_string(),
        new_orders: vec![order],
    };

    let json = serde_json::to_string(&req).unwrap();
    // Fixed field order (declaration order), SCREAMING_SNAKE_CASE enums, and
    // `None` pricing fields omitted.
    assert_eq!(
        json,
        r#"{"account_id":"1090369621183459328","new_orders":[{"client_order_id":"0KGOHL4PR2SLC0DKIND4TI0002","combo_type":"NORMAL","instrument_type":"EQUITY","market":"US","symbol":"AAPL","order_type":"MARKET","entrust_type":"QTY","side":"BUY","time_in_force":"DAY","support_trading_session":"CORE","quantity":"1"}]}"#
    );
}

#[test]
fn category_wire_values() {
    assert_eq!(Category::UsStock.as_str(), "US_STOCK");
    assert_eq!(Category::UsEtf.as_str(), "US_ETF");
}
