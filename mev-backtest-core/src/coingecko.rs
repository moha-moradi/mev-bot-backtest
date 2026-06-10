//! CoinGecko USD pricing with caching.
//!
//! Provides live USD exchange rates for native tokens of supported chains.
//! Prices are fetched once and cached in-memory with a configurable TTL.

use crate::types::ChainName;

/// Cached USD price for a chain's native token.
#[derive(Debug, Clone)]
pub struct PriceEntry {
    pub usd: f64,
    pub fetched_at: std::time::Instant,
}

/// In-memory price cache with TTL.
#[derive(Debug)]
pub struct PriceCache {
    entry: Option<PriceEntry>,
    ttl: std::time::Duration,
    api_key: Option<String>,
}

impl PriceCache {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            entry: None,
            ttl: std::time::Duration::from_secs(300), // 5 minutes
            api_key,
        }
    }

    /// Get USD price for a chain's native token.
    /// Returns cached value if fresh, otherwise fetches from API.
    pub async fn usd_price(&mut self, chain: ChainName) -> Option<f64> {
        // Stub: always returns None for now
        let _ = chain;
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_cache_returns_none_when_empty() {
        let mut cache = PriceCache::new(None);
        let price = futures::executor::block_on(cache.usd_price(ChainName::Polygon));
        assert!(price.is_none());
    }
}
