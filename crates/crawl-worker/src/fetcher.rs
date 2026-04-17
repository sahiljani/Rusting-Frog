use std::net::IpAddr;
use std::time::{Duration, Instant};

use anyhow::Result;
use reqwest::Client;
use url::Url;

pub struct FetchResult {
    pub status_code: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub content_type: String,
    pub content_length: u64,
    pub response_time_ms: u64,
    pub final_url: String,
}

pub struct Fetcher {
    client: Client,
    max_response_size: u64,
}

impl Fetcher {
    pub fn new(user_agent: &str, max_response_size: u64) -> Result<Self> {
        let client = Client::builder()
            .user_agent(user_agent)
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()?;

        Ok(Self {
            client,
            max_response_size,
        })
    }

    pub async fn fetch(&self, url: &Url) -> Result<FetchResult> {
        if is_private_ip(url) && !allow_private_ips() {
            anyhow::bail!("SSRF blocked: private/reserved IP");
        }

        let start = Instant::now();
        let resp = self.client.get(url.as_str()).send().await?;

        let status_code = resp.status().as_u16();
        let final_url = resp.url().to_string();

        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let content_length = resp.content_length().unwrap_or(0);

        let headers: Vec<(String, String)> = resp
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        let body = if content_length > self.max_response_size {
            String::new()
        } else {
            resp.text().await.unwrap_or_default()
        };

        let response_time_ms = start.elapsed().as_millis() as u64;

        Ok(FetchResult {
            status_code,
            headers,
            body,
            content_type,
            content_length,
            response_time_ms,
            final_url,
        })
    }
}

fn allow_private_ips() -> bool {
    matches!(
        std::env::var("SF_ALLOW_PRIVATE_IPS").ok().as_deref(),
        Some("1") | Some("true")
    )
}

fn is_private_ip(url: &Url) -> bool {
    let host = match url.host_str() {
        Some(h) => h,
        None => return true,
    };

    if let Ok(ip) = host.parse::<IpAddr>() {
        return match ip {
            IpAddr::V4(v4) => {
                v4.is_loopback()
                    || v4.is_private()
                    || v4.is_link_local()
                    || v4.octets()[0] == 169 && v4.octets()[1] == 254
            }
            IpAddr::V6(v6) => v6.is_loopback(),
        };
    }

    host == "localhost"
        || host.ends_with(".local")
        || host.ends_with(".internal")
}
