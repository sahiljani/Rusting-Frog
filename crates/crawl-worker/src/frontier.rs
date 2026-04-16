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

    pub fn add(&mut self, url: Url, depth: u32) -> bool {
        if depth > self.max_depth {
            return false;
        }
        if self.seen.len() as u64 >= self.max_urls {
            return false;
        }
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

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn discovered_count(&self) -> usize {
        self.seen.len()
    }
}

fn normalize_url(url: &Url) -> String {
    let mut s = url.as_str().to_string();
    if s.ends_with('/') {
        s.pop();
    }
    s.to_lowercase()
}
