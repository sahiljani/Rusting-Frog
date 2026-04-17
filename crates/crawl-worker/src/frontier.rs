use std::collections::{HashSet, VecDeque};

use url::Url;

pub struct FrontierEntry {
    pub url: Url,
    pub depth: u32,
}

pub struct Frontier {
    queue: VecDeque<FrontierEntry>,
    seen: HashSet<String>,
    max_depth: u32,
    max_urls: u64,
}

impl Frontier {
    pub fn new(max_depth: u32, max_urls: u64) -> Self {
        Self {
            queue: VecDeque::new(),
            seen: HashSet::new(),
            max_depth,
            max_urls,
        }
    }

    pub fn add(&mut self, mut url: Url, depth: u32) -> bool {
        if depth > self.max_depth {
            return false;
        }
        if self.seen.len() as u64 >= self.max_urls {
            return false;
        }
        // Fragments never reach the server — collapse them so `/` and `/#home`
        // are treated as the same URL. Matches Screaming Frog behaviour.
        url.set_fragment(None);
        let normalized = normalize_url(&url);
        if self.seen.contains(&normalized) {
            return false;
        }
        self.seen.insert(normalized);
        self.queue.push_back(FrontierEntry { url, depth });
        true
    }

    pub fn next(&mut self) -> Option<FrontierEntry> {
        self.queue.pop_front()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn discovered_count(&self) -> usize {
        self.seen.len()
    }
}

fn normalize_url(url: &Url) -> String {
    // Scheme + host are case-insensitive per RFC 3986; path/query are not.
    let scheme = url.scheme().to_ascii_lowercase();
    let host = url
        .host_str()
        .map(|h| h.to_ascii_lowercase())
        .unwrap_or_default();
    let port = url.port().map(|p| format!(":{p}")).unwrap_or_default();
    let mut path = url.path().to_string();
    if path.len() > 1 && path.ends_with('/') {
        path.pop();
    }
    let query = url.query().map(|q| format!("?{q}")).unwrap_or_default();
    format!("{scheme}://{host}{port}{path}{query}")
}
