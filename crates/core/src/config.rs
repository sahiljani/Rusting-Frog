use serde::{Deserialize, Serialize};

/// Phase 1 crawl configuration.
/// Stored as JSONB in the `crawl_configs` table.
/// Matches a subset of SF's 38 config panels (from 11-config-panels-complete.md).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
    #[serde(default)]
    pub meta_description: MetaDescriptionConfig,
    #[serde(default)]
    pub headings: HeadingsConfig,
    #[serde(default)]
    pub images: ImagesConfig,
    #[serde(default)]
    pub links: LinksConfig,
    #[serde(default)]
    pub url: UrlConfig,
    #[serde(default)]
    pub content: ContentConfig,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaDescriptionConfig {
    pub max_length: u32,
    pub min_length: u32,
    pub max_pixel_width: u32,
    pub min_pixel_width: u32,
}

impl Default for MetaDescriptionConfig {
    fn default() -> Self {
        // SF defaults matching SERP display cutoffs.
        Self {
            max_length: 155,
            min_length: 70,
            max_pixel_width: 985,
            min_pixel_width: 400,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeadingsConfig {
    pub max_h1_length: u32,
    pub max_h2_length: u32,
}

impl Default for HeadingsConfig {
    fn default() -> Self {
        Self {
            max_h1_length: 70,
            max_h2_length: 70,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImagesConfig {
    pub max_size_kb: u32,
    pub max_alt_length: u32,
}

impl Default for ImagesConfig {
    fn default() -> Self {
        Self {
            max_size_kb: 100,
            max_alt_length: 100,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinksConfig {
    pub max_crawl_depth: u32,
    pub high_internal_outlinks: u32,
    pub high_external_outlinks: u32,
}

impl Default for LinksConfig {
    fn default() -> Self {
        Self {
            max_crawl_depth: 4,
            high_internal_outlinks: 100,
            high_external_outlinks: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlConfig {
    pub max_length: u32,
}

impl Default for UrlConfig {
    fn default() -> Self {
        Self { max_length: 115 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentConfig {
    pub min_word_count: u32,
    pub readability_difficult: f64,
    pub readability_very_difficult: f64,
}

impl Default for ContentConfig {
    fn default() -> Self {
        Self {
            min_word_count: 200,
            readability_difficult: 12.0,
            readability_very_difficult: 16.0,
        }
    }
}
