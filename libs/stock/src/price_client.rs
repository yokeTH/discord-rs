use anyhow::{Error, Result};
use chrono::{DateTime, Duration, Utc};
use reqwest::{
    Client,
    header::{HeaderMap, HeaderValue},
};
use serde::Deserialize;

#[derive(Clone)]
pub struct PriceClient {
    client: Client,
    base_api: String,
}

impl PriceClient {
    pub async fn new(base_api: String, key_id: String, secret: String) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert("APCA-API-KEY-ID", HeaderValue::from_str(&key_id)?);
        headers.insert("APCA-API-SECRET-KEY", HeaderValue::from_str(&secret)?);

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Self { client, base_api })
    }

    pub async fn from_env() -> Result<Self> {
        let base_api = std::env::var("APCA_API_BASE_URL")?;
        let key_id = std::env::var("APCA_API_KEY_ID")?;
        let secret = std::env::var("APCA_API_SECRET_KEY")?;
        Self::new(base_api, key_id, secret).await
    }

    pub async fn fetch_price(
        &self,
        symbol: &str,
        duration: Duration,
        timeframe: Timeframe,
        limit: usize,
    ) -> Result<Vec<Bar>, Error> {
        let end = Utc::now();
        let start = end - duration;

        let url = format!(
            "{}/v2/stocks/{}/bars",
            self.base_api.trim_end_matches('/'),
            symbol
        );

        let res: BarsResponse = self
            .client
            .get(url)
            .query(&[
                ("feed", "iex"),
                ("timeframe", timeframe.as_str()),
                ("start", &start.to_rfc3339()),
                ("end", &end.to_rfc3339()),
                ("limit", &limit.to_string()),
            ])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(res.bars)
    }
}

//
// Match Alpaca API JSON
// https://docs.alpaca.markets/reference/stockbars
//
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Timeframe {
    Minute1,
    Minute5,
    Minute15,
    Minute30,
    Hour1,
    Day1,
    Week1,
    Month1,
}

impl Timeframe {
    pub fn as_str(&self) -> &'static str {
        match self {
            Timeframe::Minute1 => "1Min",
            Timeframe::Minute5 => "5Min",
            Timeframe::Minute15 => "15Min",
            Timeframe::Minute30 => "30Min",
            Timeframe::Hour1 => "1Hour",
            Timeframe::Day1 => "1Day",
            Timeframe::Week1 => "1Week",
            Timeframe::Month1 => "1Month",
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct BarsResponse {
    pub bars: Vec<Bar>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Bar {
    #[serde(rename = "t")]
    pub timestamp: DateTime<Utc>,

    #[serde(rename = "o")]
    pub open: f64,

    #[serde(rename = "h")]
    pub high: f64,

    #[serde(rename = "l")]
    pub low: f64,

    #[serde(rename = "c")]
    pub close: f64,

    #[serde(rename = "v")]
    pub volume: i64,
}
