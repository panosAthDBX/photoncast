//! Currency conversion module.
//!
//! Provides currency conversion for fiat currencies (via frankfurter.app)
//! and cryptocurrencies (via CoinGecko).

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::error::{CalculatorError, Result};

/// Supported fiat currencies (ISO 4217 codes).
pub static FIAT_CURRENCIES: std::sync::LazyLock<Vec<&'static str>> = std::sync::LazyLock::new(|| {
    vec![
        "AED", "AFN", "ALL", "AMD", "ANG", "AOA", "ARS", "AUD", "AWG", "AZN", "BAM", "BBD", "BDT",
        "BGN", "BHD", "BIF", "BMD", "BND", "BOB", "BRL", "BSD", "BTN", "BWP", "BYN", "BZD", "CAD",
        "CDF", "CHF", "CLP", "CNY", "COP", "CRC", "CUP", "CVE", "CZK", "DJF", "DKK", "DOP", "DZD",
        "EGP", "ERN", "ETB", "EUR", "FJD", "FKP", "GBP", "GEL", "GHS", "GIP", "GMD", "GNF", "GTQ",
        "GYD", "HKD", "HNL", "HRK", "HTG", "HUF", "IDR", "ILS", "INR", "IQD", "IRR", "ISK", "JMD",
        "JOD", "JPY", "KES", "KGS", "KHR", "KMF", "KPW", "KRW", "KWD", "KYD", "KZT", "LAK", "LBP",
        "LKR", "LRD", "LSL", "LYD", "MAD", "MDL", "MGA", "MKD", "MMK", "MNT", "MOP", "MRU", "MUR",
        "MVR", "MWK", "MXN", "MYR", "MZN", "NAD", "NGN", "NIO", "NOK", "NPR", "NZD", "OMR", "PAB",
        "PEN", "PGK", "PHP", "PKR", "PLN", "PYG", "QAR", "RON", "RSD", "RUB", "RWF", "SAR", "SBD",
        "SCR", "SDG", "SEK", "SGD", "SHP", "SLL", "SOS", "SRD", "SSP", "STN", "SYP", "SZL", "THB",
        "TJS", "TMT", "TND", "TOP", "TRY", "TTD", "TWD", "TZS", "UAH", "UGX", "USD", "UYU", "UZS",
        "VES", "VND", "VUV", "WST", "XAF", "XCD", "XOF", "XPF", "YER", "ZAR", "ZMW", "ZWL",
    ]
});

/// Crypto currency symbols that can be used instead of codes.
pub static CRYPTO_SYMBOLS: std::sync::LazyLock<HashMap<&'static str, &'static str>> = std::sync::LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert("₿", "BTC");
    m.insert("Ξ", "ETH");
    m
});

/// Supported cryptocurrencies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CryptoCurrency {
    /// Bitcoin
    BTC,
    /// Ethereum
    ETH,
    /// Tether USD
    USDT,
    /// Binance Coin
    BNB,
    /// Ripple
    XRP,
    /// Cardano
    ADA,
    /// Dogecoin
    DOGE,
    /// Solana
    SOL,
    /// USD Coin
    USDC,
    /// Polygon
    MATIC,
    /// Avalanche
    AVAX,
    /// Polkadot
    DOT,
    /// Chainlink
    LINK,
}

impl CryptoCurrency {
    /// Returns the currency code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::BTC => "BTC",
            Self::ETH => "ETH",
            Self::USDT => "USDT",
            Self::BNB => "BNB",
            Self::XRP => "XRP",
            Self::ADA => "ADA",
            Self::DOGE => "DOGE",
            Self::SOL => "SOL",
            Self::USDC => "USDC",
            Self::MATIC => "MATIC",
            Self::AVAX => "AVAX",
            Self::DOT => "DOT",
            Self::LINK => "LINK",
        }
    }

    /// Returns the CoinGecko ID for this currency.
    #[must_use]
    pub const fn coingecko_id(&self) -> &'static str {
        match self {
            Self::BTC => "bitcoin",
            Self::ETH => "ethereum",
            Self::USDT => "tether",
            Self::BNB => "binancecoin",
            Self::XRP => "ripple",
            Self::ADA => "cardano",
            Self::DOGE => "dogecoin",
            Self::SOL => "solana",
            Self::USDC => "usd-coin",
            Self::MATIC => "matic-network",
            Self::AVAX => "avalanche-2",
            Self::DOT => "polkadot",
            Self::LINK => "chainlink",
        }
    }

    /// Creates a cryptocurrency from a code string.
    pub fn from_code(code: &str) -> Option<Self> {
        match code.to_uppercase().as_str() {
            "BTC" | "BITCOIN" => Some(Self::BTC),
            "ETH" | "ETHEREUM" | "ETHER" => Some(Self::ETH),
            "USDT" | "TETHER" => Some(Self::USDT),
            "BNB" | "BINANCE" => Some(Self::BNB),
            "XRP" | "RIPPLE" => Some(Self::XRP),
            "ADA" | "CARDANO" => Some(Self::ADA),
            "DOGE" | "DOGECOIN" => Some(Self::DOGE),
            "SOL" | "SOLANA" => Some(Self::SOL),
            "USDC" => Some(Self::USDC),
            "MATIC" | "POLYGON" => Some(Self::MATIC),
            "AVAX" | "AVALANCHE" => Some(Self::AVAX),
            "DOT" | "POLKADOT" => Some(Self::DOT),
            "LINK" | "CHAINLINK" => Some(Self::LINK),
            _ => None,
        }
    }

    /// Returns all supported cryptocurrencies.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::BTC,
            Self::ETH,
            Self::USDT,
            Self::BNB,
            Self::XRP,
            Self::ADA,
            Self::DOGE,
            Self::SOL,
            Self::USDC,
            Self::MATIC,
            Self::AVAX,
            Self::DOT,
            Self::LINK,
        ]
    }
}

/// Currency code type (fiat or crypto).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CurrencyCode {
    /// Fiat currency (ISO 4217).
    Fiat(String),
    /// Cryptocurrency.
    Crypto(CryptoCurrency),
}

impl CurrencyCode {
    /// Parses a currency code string.
    pub fn parse(code: &str) -> Option<Self> {
        let upper = code.to_uppercase();

        // Try crypto first
        if let Some(crypto) = CryptoCurrency::from_code(&upper) {
            return Some(Self::Crypto(crypto));
        }

        // Try fiat
        if FIAT_CURRENCIES.contains(&upper.as_str()) {
            return Some(Self::Fiat(upper));
        }

        None
    }

    /// Returns the code as a string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Fiat(code) => code,
            Self::Crypto(crypto) => crypto.code(),
        }
    }
}

/// Currency exchange rate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeRate {
    /// The currency code.
    pub code: String,
    /// The rate relative to USD.
    pub rate_to_usd: Decimal,
    /// The source of the rate.
    pub source: String,
    /// When the rate was last updated.
    pub updated_at: DateTime<Utc>,
}

/// Currency converter with rate caching.
#[derive(Debug)]
pub struct CurrencyConverter {
    /// Fiat rates (relative to USD).
    fiat_rates: HashMap<String, ExchangeRate>,
    /// Crypto rates (relative to USD).
    crypto_rates: HashMap<String, ExchangeRate>,
}

impl CurrencyConverter {
    /// Creates a new currency converter.
    #[must_use]
    pub fn new() -> Self {
        Self {
            fiat_rates: HashMap::new(),
            crypto_rates: HashMap::new(),
        }
    }

    /// Returns whether any rates are loaded.
    #[must_use]
    pub fn has_rates(&self) -> bool {
        !self.fiat_rates.is_empty() || !self.crypto_rates.is_empty()
    }

    /// Returns the total number of loaded rates.
    #[must_use]
    pub fn rate_count(&self) -> usize {
        self.fiat_rates.len() + self.crypto_rates.len()
    }

    /// Returns whether a specific rate exists.
    #[must_use]
    pub fn has_rate(&self, code: &str) -> bool {
        let code_upper = code.to_uppercase();
        self.fiat_rates.contains_key(&code_upper) || self.crypto_rates.contains_key(&code_upper)
    }

    /// Updates fiat rates.
    pub fn update_fiat_rates(&mut self, rates: HashMap<String, Decimal>) {
        let now = Utc::now();
        for (code, rate) in rates {
            self.fiat_rates.insert(
                code.clone(),
                ExchangeRate {
                    code,
                    rate_to_usd: rate,
                    source: "frankfurter.app".to_string(),
                    updated_at: now,
                },
            );
        }
    }

    /// Updates crypto rates.
    pub fn update_crypto_rates(&mut self, rates: HashMap<String, Decimal>) {
        let now = Utc::now();
        for (code, rate) in rates {
            self.crypto_rates.insert(
                code.clone(),
                ExchangeRate {
                    code,
                    rate_to_usd: rate,
                    source: "coingecko".to_string(),
                    updated_at: now,
                },
            );
        }
    }

    /// Loads rates from a list of stored rates.
    pub fn load_rates(&mut self, rates: Vec<ExchangeRate>) {
        for rate in rates {
            let code_upper = rate.code.to_uppercase();
            if CryptoCurrency::from_code(&code_upper).is_some() {
                self.crypto_rates.insert(code_upper, rate);
            } else {
                self.fiat_rates.insert(code_upper, rate);
            }
        }
    }

    /// Converts an amount from one currency to another.
    ///
    /// Returns (converted_amount, rate, source, last_updated).
    pub fn convert(
        &self,
        amount: Decimal,
        from: &str,
        to: &str,
    ) -> Result<(Decimal, Decimal, String, DateTime<Utc>)> {
        let from_upper = from.to_uppercase();
        let to_upper = to.to_uppercase();

        // Get rates
        let from_rate = self
            .get_rate(&from_upper)
            .map_err(|err| Self::map_rate_error(err, &from_upper, &to_upper))?;
        let to_rate = self
            .get_rate(&to_upper)
            .map_err(|err| Self::map_rate_error(err, &from_upper, &to_upper))?;

        // Convert via USD
        // amount_usd = amount / from_rate (if from_rate is "X per USD")
        // For fiat from frankfurter: rate is "X per EUR", so we need to adjust
        // For our storage: we store rates relative to USD

        // Calculate the conversion
        // If from_rate is how much USD 1 unit of FROM buys
        // and to_rate is how much USD 1 unit of TO buys
        // then: result = amount * (from_rate / to_rate)

        let rate = from_rate.rate_to_usd / to_rate.rate_to_usd;
        let result = amount * rate;

        // Determine which update time to show (older of the two)
        let updated = if from_rate.updated_at < to_rate.updated_at {
            from_rate.updated_at
        } else {
            to_rate.updated_at
        };

        let source = if from_rate.source == to_rate.source {
            from_rate.source.clone()
        } else {
            format!("{} + {}", from_rate.source, to_rate.source)
        };

        Ok((result, rate, source, updated))
    }

    /// Gets the rate for a currency.
    fn get_rate(&self, code: &str) -> Result<&ExchangeRate> {
        // Check fiat first
        if let Some(rate) = self.fiat_rates.get(code) {
            return Ok(rate);
        }

        // Check crypto
        if let Some(rate) = self.crypto_rates.get(code) {
            return Ok(rate);
        }

        Err(CalculatorError::UnsupportedCurrency(code.to_string()))
    }

    fn map_rate_error(err: CalculatorError, from: &str, to: &str) -> CalculatorError {
        match err {
            CalculatorError::UnsupportedCurrency(code) => {
                if CurrencyCode::parse(&code).is_some() {
                    CalculatorError::RateNotAvailable {
                        from: from.to_string(),
                        to: to.to_string(),
                    }
                } else {
                    CalculatorError::UnsupportedCurrency(code)
                }
            },
            _ => err,
        }
    }
}

impl Default for CurrencyConverter {
    fn default() -> Self {
        Self::new()
    }
}

/// Fetches fiat currency rates from frankfurter.app.
///
/// Returns rates relative to USD.
pub async fn fetch_fiat_rates() -> Result<HashMap<String, Decimal>> {
    info!("Fetching fiat rates from frankfurter.app...");

    let client = reqwest::Client::builder()
        .user_agent("photoncast/0.1 (+https://github.com/photoncast/photoncast)")
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()?;

    // frankfurter.app returns rates relative to EUR by default
    // We need to convert to USD-based rates
    let url = "https://api.frankfurter.app/latest?from=USD";

    let response = client
        .get(url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(CalculatorError::ApiError {
            service: "frankfurter.app".to_string(),
            message: format!("HTTP {}", response.status()),
        });
    }

    let data: FrankfurterResponse = response.json().await?;

    // Convert to our format
    // frankfurter returns: { "rates": { "EUR": 0.92, "GBP": 0.79, ... } }
    // These are "how much X you get for 1 USD"
    // We want to store "how much USD you get for 1 X"
    // So we invert: rate_to_usd = 1 / frankfurter_rate

    let mut rates = HashMap::new();

    // Add USD itself
    rates.insert("USD".to_string(), Decimal::ONE);

    for (code, rate) in data.rates {
        // Invert the rate: 1/rate gives "USD per unit"
        if rate > 0.0 {
            let decimal_rate = Decimal::from_f64(rate).unwrap_or(Decimal::ONE);
            let inverted = Decimal::ONE / decimal_rate;
            rates.insert(code, inverted);
        }
    }

    info!("Fetched {} fiat rates", rates.len());
    Ok(rates)
}

/// Fetches cryptocurrency rates from CoinGecko.
///
/// Returns rates in USD (how much USD per 1 crypto).
pub async fn fetch_crypto_rates() -> Result<HashMap<String, Decimal>> {
    info!("Fetching crypto rates from CoinGecko...");

    let client = reqwest::Client::builder()
        .user_agent("photoncast/0.1 (+https://github.com/photoncast/photoncast)")
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()?;

    // Build the IDs list
    let ids: Vec<&str> = CryptoCurrency::all()
        .iter()
        .map(CryptoCurrency::coingecko_id)
        .collect();
    let ids_param = ids.join(",");

    let url = format!(
        "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd",
        ids_param
    );

    let response = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(CalculatorError::ApiError {
            service: "coingecko".to_string(),
            message: format!("HTTP {}", response.status()),
        });
    }

    let data: HashMap<String, CoinGeckoPrice> = response.json().await?;

    // Convert to our format
    let mut rates = HashMap::new();

    for crypto in CryptoCurrency::all() {
        if let Some(price) = data.get(crypto.coingecko_id()) {
            // CoinGecko returns price in USD, which is what we want
            let decimal_rate = Decimal::from_f64(price.usd).unwrap_or(Decimal::ZERO);
            rates.insert(crypto.code().to_string(), decimal_rate);
        }
    }

    info!("Fetched {} crypto rates", rates.len());
    Ok(rates)
}

/// Response from frankfurter.app API.
#[derive(Debug, Deserialize)]
struct FrankfurterResponse {
    rates: HashMap<String, f64>,
}

/// Price data from CoinGecko API.
#[derive(Debug, Deserialize)]
struct CoinGeckoPrice {
    usd: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crypto_currency_from_code() {
        assert_eq!(CryptoCurrency::from_code("btc"), Some(CryptoCurrency::BTC));
        assert_eq!(CryptoCurrency::from_code("ETH"), Some(CryptoCurrency::ETH));
        assert_eq!(
            CryptoCurrency::from_code("bitcoin"),
            Some(CryptoCurrency::BTC)
        );
        assert_eq!(CryptoCurrency::from_code("xyz"), None);
    }

    #[test]
    fn test_currency_code_parse() {
        assert!(matches!(
            CurrencyCode::parse("USD"),
            Some(CurrencyCode::Fiat(_))
        ));
        assert!(matches!(
            CurrencyCode::parse("btc"),
            Some(CurrencyCode::Crypto(_))
        ));
        assert!(CurrencyCode::parse("XYZ123").is_none());
    }

    #[test]
    fn test_currency_converter() {
        let mut converter = CurrencyConverter::new();

        // Add some test rates (all relative to USD)
        let mut fiat_rates = HashMap::new();
        fiat_rates.insert("USD".to_string(), Decimal::ONE);
        fiat_rates.insert("EUR".to_string(), Decimal::from_str("1.09").unwrap()); // 1 EUR = 1.09 USD
        fiat_rates.insert("GBP".to_string(), Decimal::from_str("1.27").unwrap()); // 1 GBP = 1.27 USD

        converter.update_fiat_rates(fiat_rates);

        // Test USD to EUR conversion
        // If 1 EUR = 1.09 USD, then 100 USD = 100/1.09 EUR ≈ 91.74 EUR
        let (result, rate, _, _) = converter
            .convert(Decimal::from(100), "USD", "EUR")
            .expect("conversion failed");

        // Rate should be 1/1.09 ≈ 0.917
        assert!(
            (rate - Decimal::from_str("0.917431192660550458715596330").unwrap()).abs()
                < Decimal::from_str("0.01").unwrap()
        );
        assert!(
            (result - Decimal::from_str("91.74").unwrap()).abs()
                < Decimal::from_str("0.1").unwrap()
        );
    }

    #[test]
    fn test_converter_unsupported_currency() {
        let converter = CurrencyConverter::new();
        let result = converter.convert(Decimal::from(100), "XYZ", "USD");
        assert!(matches!(
            result,
            Err(CalculatorError::UnsupportedCurrency(_))
        ));
    }
}
