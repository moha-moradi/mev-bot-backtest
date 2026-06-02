use std::collections::HashMap;
use std::path::Path;

use alloy::primitives::Address;

/// A daily USD price snapshot for a single token.
#[derive(Debug, Clone)]
pub struct PriceSnapshot {
    /// Unix timestamp (seconds since epoch)
    pub timestamp: u64,
    /// USD price per token
    pub price_usd: f64,
}

/// Loads historical USD prices from a CSV file and provides
/// timestamp-based queries with linear interpolation.
///
/// CSV format (header required):
/// ```csv
/// date,token_address,price_usd
/// 2024-01-01,0x7ceb23fd6bc0add59e62ac25578270cff1b9f619,3500.00
/// ```
///
/// Date column accepts `YYYY-MM-DD` or unix timestamp (seconds).
pub struct HistoricalPriceDB {
    /// token address -> sorted list of price snapshots
    data: HashMap<Address, Vec<PriceSnapshot>>,
}

impl HistoricalPriceDB {
    /// Load prices from a CSV file.
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let mut data: HashMap<Address, Vec<PriceSnapshot>> = HashMap::new();
        let mut reader = csv::ReaderBuilder::new()
            .trim(csv::Trim::All)
            .from_path(path)
            .map_err(|e| anyhow::anyhow!("Failed to open CSV '{}': {}", path, e))?;

        for result in reader.records() {
            let record = result
                .map_err(|e| anyhow::anyhow!("CSV parse error in '{}': {}", path, e))?;
            if record.len() < 3 {
                continue;
            }

            let date_str = record.get(0).unwrap_or("");
            let token_str = record.get(1).unwrap_or("");
            let price_str = record.get(2).unwrap_or("");

            let timestamp = parse_date_to_timestamp(date_str)?;
            let token: Address = token_str
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid token address: {}", token_str))?;
            let price_usd: f64 = price_str
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid price: {}", price_str))?;

            data.entry(token).or_default().push(PriceSnapshot {
                timestamp,
                price_usd,
            });
        }

        // Sort each token's snapshots by timestamp
        for snapshots in data.values_mut() {
            snapshots.sort_by_key(|s| s.timestamp);
        }

        Ok(HistoricalPriceDB { data })
    }

    /// Load prices from an optional path; returns empty DB if path is None or missing.
    pub fn load_optional(path: Option<&str>) -> Self {
        match path {
            Some(p) if Path::new(p).exists() => Self::load(p).unwrap_or_else(|e| {
                tracing::warn!("Failed to load historical prices from '{}': {}", p, e);
                HistoricalPriceDB {
                    data: HashMap::new(),
                }
            }),
            _ => HistoricalPriceDB {
                data: HashMap::new(),
            },
        }
    }

    /// Get the USD price of a token at a given block timestamp.
    /// Uses linear interpolation between the nearest snapshots.
    /// Returns None if no data is available for the token.
    pub fn get_price(&self, token: &Address, block_timestamp: u64) -> Option<f64> {
        let snapshots = self.data.get(token)?;

        if snapshots.is_empty() {
            return None;
        }

        // Exact match or before first snapshot
        if block_timestamp <= snapshots[0].timestamp {
            return Some(snapshots[0].price_usd);
        }

        // After last snapshot
        if block_timestamp >= snapshots[snapshots.len() - 1].timestamp {
            return Some(snapshots[snapshots.len() - 1].price_usd);
        }

        // Linear interpolation between two surrounding snapshots
        for i in 0..snapshots.len() - 1 {
            let left = &snapshots[i];
            let right = &snapshots[i + 1];

            if block_timestamp >= left.timestamp && block_timestamp <= right.timestamp {
                if right.timestamp == left.timestamp {
                    return Some(left.price_usd);
                }
                let fraction = (block_timestamp - left.timestamp) as f64
                    / (right.timestamp - left.timestamp) as f64;
                return Some(left.price_usd + fraction * (right.price_usd - left.price_usd));
            }
        }

        None
    }

    /// Returns true if the DB has any data for the given token.
    pub fn has_token(&self, token: &Address) -> bool {
        self.data.contains_key(token)
    }

    /// Returns the number of tokens tracked.
    pub fn token_count(&self) -> usize {
        self.data.len()
    }

    /// Returns the total number of price snapshots across all tokens.
    pub fn snapshot_count(&self) -> usize {
        self.data.values().map(|v| v.len()).sum()
    }
}

/// Parse a date string into a unix timestamp.
/// Accepts `YYYY-MM-DD` or a unix timestamp as string.
fn parse_date_to_timestamp(s: &str) -> anyhow::Result<u64> {
    let s = s.trim();

    // Try numeric (unix timestamp)
    if let Ok(ts) = s.parse::<u64>() {
        return Ok(ts);
    }

    // Try YYYY-MM-DD format
    if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let dt = d
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| anyhow::anyhow!("Invalid date: {}", s))?;
        return Ok(dt.and_utc().timestamp() as u64);
    }

    Err(anyhow::anyhow!(
        "Unrecognized date format: '{}'. Expected YYYY-MM-DD or unix timestamp.",
        s
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_date_standard() {
        let ts = parse_date_to_timestamp("2024-01-01").unwrap();
        // 2024-01-01 00:00:00 UTC = 1704067200
        assert_eq!(ts, 1704067200);
    }

    #[test]
    fn test_parse_date_unix_timestamp() {
        let ts = parse_date_to_timestamp("1704067200").unwrap();
        assert_eq!(ts, 1704067200);
    }

    #[test]
    fn test_parse_date_invalid() {
        assert!(parse_date_to_timestamp("not-a-date").is_err());
    }

    #[test]
    fn test_get_price_exact() {
        let mut db = HistoricalPriceDB {
            data: HashMap::new(),
        };
        let token: Address = "0x7ceb23fd6bc0add59e62ac25578270cff1b9f619"
            .parse()
            .unwrap();
        db.data.insert(
            token,
            vec![
                PriceSnapshot {
                    timestamp: 1704067200,
                    price_usd: 3500.0,
                },
                PriceSnapshot {
                    timestamp: 1704153600,
                    price_usd: 3600.0,
                },
            ],
        );

        assert_eq!(db.get_price(&token, 1704067200), Some(3500.0));
        assert_eq!(db.get_price(&token, 1704153600), Some(3600.0));
    }

    #[test]
    fn test_get_price_before_first() {
        let mut db = HistoricalPriceDB {
            data: HashMap::new(),
        };
        let token: Address = "0x7ceb23fd6bc0add59e62ac25578270cff1b9f619"
            .parse()
            .unwrap();
        db.data.insert(
            token,
            vec![PriceSnapshot {
                timestamp: 1704067200,
                price_usd: 3500.0,
            }],
        );

        assert_eq!(db.get_price(&token, 1704060000), Some(3500.0));
    }

    #[test]
    fn test_get_price_after_last() {
        let mut db = HistoricalPriceDB {
            data: HashMap::new(),
        };
        let token: Address = "0x7ceb23fd6bc0add59e62ac25578270cff1b9f619"
            .parse()
            .unwrap();
        db.data.insert(
            token,
            vec![PriceSnapshot {
                timestamp: 1704067200,
                price_usd: 3500.0,
            }],
        );

        assert_eq!(db.get_price(&token, 1704153600), Some(3500.0));
    }

    #[test]
    fn test_get_price_interpolation() {
        let mut db = HistoricalPriceDB {
            data: HashMap::new(),
        };
        let token: Address = "0x7ceb23fd6bc0add59e62ac25578270cff1b9f619"
            .parse()
            .unwrap();
        db.data.insert(
            token,
            vec![
                PriceSnapshot {
                    timestamp: 1704067200, // Jan 1
                    price_usd: 3500.0,
                },
                PriceSnapshot {
                    timestamp: 1704153600, // Jan 2
                    price_usd: 3600.0,
                },
            ],
        );

        // Midpoint (noon on Jan 1)
        let noon = 1704067200 + 43200;
        let price = db.get_price(&token, noon).unwrap();
        // Linear interpolation: 3500 + 0.5 * (3600 - 3500) = 3550
        assert!((price - 3550.0).abs() < 0.001);
    }

    #[test]
    fn test_get_price_unknown_token() {
        let db = HistoricalPriceDB {
            data: HashMap::new(),
        };
        let token: Address = "0x0000000000000000000000000000000000000000"
            .parse()
            .unwrap();
        assert_eq!(db.get_price(&token, 1704067200), None);
    }

    #[test]
    fn test_empty_db() {
        let db = HistoricalPriceDB {
            data: HashMap::new(),
        };
        assert_eq!(db.token_count(), 0);
        assert_eq!(db.snapshot_count(), 0);
    }
}
