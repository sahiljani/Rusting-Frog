//! Fetch + cache robots.txt for a crawl and decide per-URL fetchability.
//!
//! Used by the pipeline to (a) persist the raw file for the API's
//! `GET /v1/crawls/:id/robots` endpoint and (b) gate every frontier URL
//! so a `Disallow` directive skips the fetch entirely (and sets the
//! row's `blocked_by_robots` flag for SF's "Blocked by Robots.txt"
//! filter).

use robotstxt::DefaultMatcher;
use url::Url;

pub struct RobotsGate {
    raw: Option<String>,
    status: Option<i16>,
    user_agent_token: String,
}

impl RobotsGate {
    /// Fetch https://{host}/robots.txt. Network / non-2xx responses are
    /// treated as "no robots file" — fetch-everything behaviour. This
    /// mirrors what Screaming Frog does by default.
    pub async fn fetch(client: &reqwest::Client, seed: &Url, user_agent: &str) -> Self {
        let robots_url = format!("{}://{}/robots.txt", seed.scheme(), seed.host_str().unwrap_or(""));
        let (raw, status) = match client.get(&robots_url).send().await {
            Ok(resp) => {
                let code = resp.status().as_u16() as i16;
                let body = resp.text().await.unwrap_or_default();
                if (200..300).contains(&code) {
                    (Some(body), Some(code))
                } else {
                    (None, Some(code))
                }
            }
            Err(_) => (None, None),
        };

        Self {
            raw,
            status,
            user_agent_token: first_token(user_agent),
        }
    }

    pub fn raw(&self) -> Option<&str> {
        self.raw.as_deref()
    }

    pub fn status(&self) -> Option<i16> {
        self.status
    }

    /// Returns true when the configured user agent is allowed to fetch
    /// the URL. If there's no robots file, everything is allowed.
    pub fn is_allowed(&self, url: &Url) -> bool {
        let body = match &self.raw {
            Some(b) if !b.trim().is_empty() => b,
            _ => return true,
        };
        let mut matcher = DefaultMatcher::default();
        matcher.one_agent_allowed_by_robots(body, &self.user_agent_token, url.as_str())
    }
}

// robots.txt matches on the first path token of the UA, so strip any
// suffix beyond the first non [a-zA-Z_-] char — matches what the crate
// itself does internally for rule-side tokens.
fn first_token(user_agent: &str) -> String {
    let end = user_agent
        .find(|c: char| !(c.is_ascii_alphabetic() || c == '-' || c == '_'))
        .unwrap_or(user_agent.len());
    user_agent[..end].to_string()
}
