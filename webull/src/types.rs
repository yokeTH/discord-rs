//! Enums for the Webull OpenAPI, ported from the SDK's `common` modules.
//!
//! Every enum serializes to / deserializes from its exact wire string (the
//! Python SDK's `EasyEnum` member name, e.g. `US_STOCK`, `BUY`, `MARKET`), and
//! exposes [`as_str`](Category::as_str) / [`Display`] for building query
//! parameters. Money and price fields elsewhere in the crate stay `String`
//! (the API returns them as strings) to avoid float precision loss.
//!
//! These types are always compiled — they carry no dependency on the HTTP
//! client, so they are available even with `--no-default-features`.

/// Declares a fieldless enum whose wire representation is a fixed string per
/// variant, wiring up `as_str`, `Display`, `Serialize`, and `Deserialize` from
/// a single source of truth.
macro_rules! wire_enum {
    ($(#[$meta:meta])* $name:ident { $($(#[$vmeta:meta])* $variant:ident => $wire:literal $(| $alias:literal)*),* $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $name {
            $($(#[$vmeta])* $variant),*
        }

        impl $name {
            /// The exact wire value used in query params and JSON bodies.
            #[must_use]
            pub fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $wire),*
                }
            }
        }

        impl core::fmt::Display for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl serde::Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str(self.as_str())
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let s = <String as serde::Deserialize>::deserialize(d)?;
                match s.as_str() {
                    $($wire $(| $alias)* => Ok(Self::$variant),)*
                    other => Err(serde::de::Error::custom(format!(
                        concat!("unknown ", stringify!($name), ": {}"),
                        other
                    ))),
                }
            }
        }
    };
}

// ---------------------------------------------------------------------------
// Shared / trade enums
// ---------------------------------------------------------------------------

wire_enum! {
    /// Security category (market + product), used across trade and market data.
    Category {
        UsStock => "US_STOCK",
        UsOption => "US_OPTION",
        HkStock => "HK_STOCK",
        UsEtf => "US_ETF",
        HkEtf => "HK_ETF",
        CnStock => "CN_STOCK",
        UsCrypto => "US_CRYPTO",
        UsFutures => "US_FUTURES",
        HkFutures => "HK_FUTURES",
        UsEvent => "US_EVENT",
    }
}

wire_enum! {
    /// Brokerage account type.
    AccountType { Cash => "CASH", Margin => "MARGIN" }
}

wire_enum! {
    /// Trading venue / region.
    Market { Us => "US", Hk => "HK", Jp => "JP" }
}

wire_enum! {
    /// Financial instrument type used in stock order payloads.
    InstrumentType { Equity => "EQUITY" }
}

wire_enum! {
    /// Order combination type.
    ComboType {
        Master => "MASTER",
        StopLossProfit => "STOP_LOSS_PROFIT",
        StopLoss => "STOP_LOSS",
        StopProfit => "STOP_PROFIT",
        Oto => "OTO",
        Oco => "OCO",
        Otoco => "OTOCO",
        Normal => "NORMAL",
        Broken => "BROKEN",
        BrokenInt => "BROKEN_INT",
        BrokenDec => "BROKEN_DEC",
    }
}

wire_enum! {
    /// Ticker type within a combo order.
    ComboTickerType { Option => "OPTION", Crypto => "CRYPTO", Stock => "STOCK" }
}

wire_enum! {
    /// Currency code.
    Currency { Cnh => "CNH", Cad => "CAD", Hkd => "HKD", Usd => "USD" }
}

wire_enum! {
    /// Reason an instrument is not tradable.
    ForbidReason {
        Delisted => "DELISTED",
        ReverseSplit => "REVERSE_SPLIT",
        StockSplit => "STOCK_SPLIT",
        ToOtc => "TO_OTC",
        NameChange => "NAME_CHANGE",
        IpoDelay => "IPO_DELAY",
        InstrumentTypeError => "INSTRUMENT_TYPE_ERROR",
        Other => "OTHER",
    }
}

wire_enum! {
    /// Buy, sell, or short.
    OrderSide { Buy => "BUY", Sell => "SELL", Short => "SHORT" }
}

wire_enum! {
    /// How an order executes.
    OrderType {
        Market => "MARKET",
        Limit => "LIMIT",
        StopLoss => "STOP_LOSS",
        StopLossLimit => "STOP_LOSS_LIMIT",
        TrailingStopLoss => "TRAILING_STOP_LOSS",
        EnhancedLimit => "ENHANCED_LIMIT",
        AtAuction => "AT_AUCTION",
        AtAuctionLimit => "AT_AUCTION_LIMIT",
        OddLotLimit => "ODD_LOT_LIMIT",
        MarketOnOpen => "MARKET_ON_OPEN",
        MarketOnClose => "MARKET_ON_CLOSE",
    }
}

wire_enum! {
    /// Whether an order is sized by share quantity or by cash amount.
    EntrustType { Qty => "QTY", Amount => "AMOUNT" }
}

wire_enum! {
    /// How long an order remains active.
    TimeInForce { Day => "DAY", Gtc => "GTC", Ioc => "IOC" }
}

wire_enum! {
    /// Trading session window an order may execute in.
    TradingSession { Night => "NIGHT", All => "ALL", Core => "CORE", AllDay => "ALL_DAY" }
}

wire_enum! {
    /// Lifecycle status of an order.
    OrderStatus {
        Pending => "PENDING",
        Submitted => "SUBMITTED",
        Cancelled => "CANCELED" | "CANCELLED",
        Filled => "FILLED",
        Failed => "FAILED",
        PartialFilled => "PARTIAL_FILLED",
    }
}

wire_enum! {
    /// Position intent for option orders.
    PositionIntent {
        BuyToOpen => "BUY_TO_OPEN",
        BuyToClose => "BUY_TO_CLOSE",
        SellToOpen => "SELL_TO_OPEN",
        SellToClose => "SELL_TO_CLOSE",
    }
}

wire_enum! {
    /// Whether opening/closing positions is allowed.
    TradePolicy { All => "ALL", CloseOnly => "CLOSE_ONLY", Denied => "DENIED" }
}

wire_enum! {
    /// Full vs half trading day.
    TradingDateType { FullDay => "FULL_DAY", HalfDay => "HALF_DAY" }
}

wire_enum! {
    /// Trailing-stop basis.
    TrailingType { Percentage => "PERCENTAGE", Amount => "AMOUNT" }
}

wire_enum! {
    /// Status of an access token.
    TokenStatus {
        /// Awaiting SMS 2FA verification in the Webull app.
        Pending => "PENDING",
        /// Active and usable for API calls.
        Normal => "NORMAL",
        /// Unused for 15+ days, or does not exist.
        Invalid => "INVALID",
        /// Verification window (5 min) was missed.
        Expired => "EXPIRED",
    }
}

// ---------------------------------------------------------------------------
// Market-data enums
// ---------------------------------------------------------------------------

wire_enum! {
    /// Candlestick granularity for historical bars.
    Timespan {
        S5 => "S5",
        S15 => "S15",
        M1 => "M1",
        M5 => "M5",
        M15 => "M15",
        M30 => "M30",
        M60 => "M60",
        M120 => "M120",
        M240 => "M240",
        Day => "D",
        Week => "W",
        Month => "M",
        Year => "Y",
    }
}

wire_enum! {
    /// Futures contract type.
    ContractType { Monthly => "MONTHLY", Main => "MAIN" }
}

wire_enum! {
    /// Tick trade direction.
    Direction { B => "B", S => "S", G => "G", L => "L", N => "N" }
}

wire_enum! {
    /// Exchange code.
    ExchangeCode {
        Hkg => "HKG",
        Nas => "NAS",
        Nyse => "NYSE",
        Ase => "ASE",
        Pse => "PSE",
        Bat => "BAT",
        Nms => "NMS",
        Nsq => "NSQ",
        Otc => "OTC",
        Pk => "PK",
        Opra => "OPRA",
        Ccc => "CCC",
        Hkw => "HKW",
    }
}

wire_enum! {
    /// Option exercise style.
    ExerciseStyle { American => "AMERICAN", European => "EUROPEAN" }
}

wire_enum! {
    /// Option expiration cycle. (`Quarterly` keeps the SDK's `QUATERLY` wire spelling.)
    ExpirationCycle {
        Daily => "DAILY",
        Weekly => "WEEKLY",
        Monthly => "MONTHLY",
        Quarterly => "QUATERLY",
        EndOfMonth => "END_OF_MONTH",
        Yearly => "YEARLY",
    }
}

wire_enum! {
    /// Instrument trading status.
    InstrumentStatus { Active => "ACTIVE", Inactive => "INACTIVE", Halted => "HALTED" }
}

wire_enum! {
    /// Option right.
    OptionType { Call => "CALL", Put => "PUT" }
}
