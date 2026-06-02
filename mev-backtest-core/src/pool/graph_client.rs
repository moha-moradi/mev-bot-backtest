use std::collections::HashSet;

use alloy::primitives::Address;
use serde::{Deserialize, Serialize};

use crate::pool::dex_type::DexType;
use crate::pool::state::PoolInfo;

const UNISWAP_V2_SUBGRAPH: &str =
    "https://gateway.thegraph.com/api/{api_key}/subgraphs/id/EXLWG1EhgRL7PkH2QhETtTBCMyYqZFGcvtbSMNfEhHkT";
const UNISWAP_V3_SUBGRAPH: &str =
    "https://gateway.thegraph.com/api/{api_key}/subgraphs/id/5zvR82dY4ibLwdYNseXNR4zQsNUT2zHuBKET7G8hMy7D";
const DEFAULT_PAGE_SIZE: i32 = 1000;

pub struct TheGraphClient {
    client: reqwest::Client,
    #[allow(dead_code)]
    api_key: String,
    v2_url: String,
    v3_url: String,
}

#[derive(Deserialize)]
struct GraphResponse<T> {
    data: Option<T>,
}

#[derive(Deserialize)]
struct PoolsDataV2 {
    pairs: Vec<GraphPoolV2>,
}

#[derive(Deserialize)]
struct GraphPoolV2 {
    id: String,
    token0: GraphToken,
    token1: GraphToken,
}

#[derive(Deserialize)]
struct PoolsDataV3 {
    pools: Vec<GraphPoolV3>,
}

#[derive(Deserialize)]
struct GraphPoolV3 {
    id: String,
    token0: GraphToken,
    token1: GraphToken,
    fee_tier: String,
    tick_spacing: Option<String>,
}

#[derive(Deserialize)]
struct GraphToken {
    id: String,
}

#[derive(Serialize)]
struct GraphRequest {
    query: String,
    variables: serde_json::Value,
}

impl TheGraphClient {
    pub fn new(api_key: String) -> Self {
        let v2_url = UNISWAP_V2_SUBGRAPH.replace("{api_key}", &api_key);
        let v3_url = UNISWAP_V3_SUBGRAPH.replace("{api_key}", &api_key);
        TheGraphClient {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("reqwest client"),
            api_key,
            v2_url,
            v3_url,
        }
    }

    pub fn with_custom_urls(mut self, v2_url: String, v3_url: String) -> Self {
        self.v2_url = v2_url;
        self.v3_url = v3_url;
        self
    }

    pub async fn fetch_v2_pools(
        &self,
        factory_filter: Option<Address>,
        existing: &HashSet<Address>,
    ) -> anyhow::Result<Vec<PoolInfo>> {
        let factory_filter_str = factory_filter.map(|a| format!("{:#x}", a));
        let mut all_pools = Vec::new();
        let mut skip = 0i32;

        loop {
            let query = r#"
                query ($skip: Int!, $first: Int!, $factory: String) {
                    pairs(skip: $skip, first: $first, where: { factory_gt: $factory, factory_lt: $factory }) {
                        id
                        token0 { id }
                        token1 { id }
                    }
                }
            "#;

            let vars = serde_json::json!({
                "skip": skip,
                "first": DEFAULT_PAGE_SIZE,
                "factory": factory_filter_str,
            });

            let resp: GraphResponse<PoolsDataV2> = self
                .query_graph(&self.v2_url, query, vars)
                .await?;

            let Some(data) = resp.data else { break };
            if data.pairs.is_empty() {
                break;
            }

            let has_more = data.pairs.len() >= DEFAULT_PAGE_SIZE as usize;
            for pair in data.pairs {
                let addr = match pair.id.parse::<Address>() {
                    Ok(a) => a,
                    Err(_) => continue,
                };
                if existing.contains(&addr) {
                    continue;
                }
                let token0 = match pair.token0.id.parse::<Address>() {
                    Ok(a) => a,
                    Err(_) => continue,
                };
                let token1 = match pair.token1.id.parse::<Address>() {
                    Ok(a) => a,
                    Err(_) => continue,
                };

                all_pools.push(PoolInfo {
                    address: addr,
                    pool_type: "UniswapV2".to_string(),
                    token0,
                    token1,
                    fee: 30,
                    name: None,
                    dex_type: DexType::UniswapV2,
                    tick_spacing: None,
                });
            }

            if !has_more {
                break;
            }
            skip += DEFAULT_PAGE_SIZE;
        }

        Ok(all_pools)
    }

    pub async fn fetch_v3_pools(
        &self,
        factory_filter: Option<Address>,
        existing: &HashSet<Address>,
    ) -> anyhow::Result<Vec<PoolInfo>> {
        let factory_filter_str = factory_filter.map(|a| format!("{:#x}", a));
        let mut all_pools = Vec::new();
        let mut skip = 0i32;

        loop {
            let query = r#"
                query ($skip: Int!, $first: Int!, $factory: String) {
                    pools(skip: $skip, first: $first, where: { factory_gt: $factory, factory_lt: $factory }) {
                        id
                        token0 { id }
                        token1 { id }
                        feeTier
                        tickSpacing
                    }
                }
            "#;

            let vars = serde_json::json!({
                "skip": skip,
                "first": DEFAULT_PAGE_SIZE,
                "factory": factory_filter_str,
            });

            let resp: GraphResponse<PoolsDataV3> = self
                .query_graph(&self.v3_url, query, vars)
                .await?;

            let Some(data) = resp.data else { break };
            if data.pools.is_empty() {
                break;
            }

            let has_more = data.pools.len() >= DEFAULT_PAGE_SIZE as usize;
            for pool in data.pools {
                let addr = match pool.id.parse::<Address>() {
                    Ok(a) => a,
                    Err(_) => continue,
                };
                if existing.contains(&addr) {
                    continue;
                }
                let token0 = match pool.token0.id.parse::<Address>() {
                    Ok(a) => a,
                    Err(_) => continue,
                };
                let token1 = match pool.token1.id.parse::<Address>() {
                    Ok(a) => a,
                    Err(_) => continue,
                };
                let fee: u32 = pool.fee_tier.parse().unwrap_or(30);
                let tick_spacing: Option<u32> = pool
                    .tick_spacing
                    .and_then(|s| s.parse().ok());

                all_pools.push(PoolInfo {
                    address: addr,
                    pool_type: "UniswapV3".to_string(),
                    token0,
                    token1,
                    fee,
                    name: None,
                    dex_type: DexType::UniswapV3,
                    tick_spacing,
                });
            }

            if !has_more {
                break;
            }
            skip += DEFAULT_PAGE_SIZE;
        }

        Ok(all_pools)
    }

    async fn query_graph<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        query: &str,
        variables: serde_json::Value,
    ) -> anyhow::Result<GraphResponse<T>> {
        let body = GraphRequest {
            query: query.to_string(),
            variables,
        };

        let resp = self
            .client
            .post(url)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("TheGraph request failed: {}", e))?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("TheGraph returned HTTP {}: {}", status, text);
        }

        let graph_resp: GraphResponse<T> = resp
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("TheGraph parse failed: {}", e))?;

        Ok(graph_resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_construction() {
        let client = TheGraphClient::new("test_key".to_string());
        assert!(client.v2_url.contains("test_key"));
        assert!(client.v3_url.contains("test_key"));
    }

    #[test]
    fn test_client_custom_urls() {
        let client = TheGraphClient::new("k".to_string())
            .with_custom_urls("http://v2".to_string(), "http://v3".to_string());
        assert_eq!(client.v2_url, "http://v2");
        assert_eq!(client.v3_url, "http://v3");
    }
}
