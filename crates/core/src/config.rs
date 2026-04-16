use serde::{Deserialize, Serialize};

/// Phase 1 crawl configuration.
/// Stored as JSONB in the `crawl_configs` table.
/// Matches a subset of SF's 38 config panels (from 11-config-panels-complete.md).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlConfig {
    #[serde(default)]
    pub speed: SpeedConfig,
    #[serde(default)]
    pub limits: LimitsConfig,
    #[serde(default)]
    pub user_agent: UserAgentConfig,
    #[serde(default)]
    pub robots: RobotsConfig,
    #[serde(default)]
    pub page_title: PageTitleConfig,
}

impl Default for CrawlConfig {
    fn default() -> Self {
        Self {
            speed: SpeedConfig::default(),
            limits: LimitsConfig::default(),
            user_agent: UserAgentConfig::default(),
            robots: RobotsConfig::default(),
            page_title: PageTitleConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeedConfig {
    pub max_threads: u32,
    pub max_uri_per_second: u32,
}

impl Default for SpeedConfig {
    fn default() -> Self {
        Self {
            max_threads: 5,
            max_uri_per_second: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitsConfig {
    pub max_crawl_depth: u32,
    pub max_urls: u64,
    pub max_response_size_bytes: u64,
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            max_crawl_depth: 10,
            max_urls: 10_000,
            max_response_size_bytes: 10 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAgentConfig {
    pub user_agent_string: String,
}

impl Default for UserAgentConfig {
    fn default() -> Self {
        Self {
            user_agent_string: "ScreamingFrogClone/1.0".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobotsConfig {
    pub respect_robots_txt: bool,
}

impl Default for RobotsConfig {
    fn default() -> Self {
        Self {
            respect_robots_txt: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageTitleConfig {
    pub max_title_length: u32,
    pub min_title_length: u32,
    pub max_title_pixel_width: u32,
    pub min_title_pixel_width: u32,
}

impl Default for PageTitleConfig {
    fn default() -> Self {
        Self {
            max_title_length: 60,
            min_title_length: 30,
            max_title_pixel_width: 580,
            min_title_pixel_width: 200,
        }
    }
}
