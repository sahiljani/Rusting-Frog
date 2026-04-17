use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::id::{CrawlId, CrawlUrlId, ProjectId, TenantId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrawlStatus {
    Queued,
    Running,
    Paused,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentType {
    Html,
    JavaScript,
    Css,
    Image,
    Pdf,
    Plugin,
    Media,
    Font,
    Xml,
    Other,
    Unknown,
}

impl ContentType {
    pub fn from_mime(mime: &str) -> Self {
        let lower = mime.to_lowercase();
        if lower.contains("text/html") || lower.contains("application/xhtml") {
            Self::Html
        } else if lower.contains("javascript") || lower.contains("ecmascript") {
            Self::JavaScript
        } else if lower.contains("text/css") {
            Self::Css
        } else if lower.starts_with("image/") {
            Self::Image
        } else if lower.contains("pdf") {
            Self::Pdf
        } else if lower.starts_with("video/") || lower.starts_with("audio/") {
            Self::Media
        } else if lower.contains("font") || lower.contains("woff") || lower.contains("ttf") || lower.contains("otf") {
            Self::Font
        } else if lower.contains("xml") {
            Self::Xml
        } else if lower.contains("flash") || lower.contains("shockwave") || lower.contains("silverlight") || lower.contains("java-archive") {
            Self::Plugin
        } else if mime.is_empty() {
            Self::Unknown
        } else {
            Self::Other
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub tenant_id: TenantId,
    pub name: String,
    pub seed_url: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Crawl {
    pub id: CrawlId,
    pub project_id: ProjectId,
    pub tenant_id: TenantId,
    pub status: CrawlStatus,
    pub seed_urls: Vec<Url>,
    pub urls_discovered: i64,
    pub urls_crawled: i64,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlUrl {
    pub id: CrawlUrlId,
    pub crawl_id: CrawlId,
    pub url: String,
    pub url_hash: String,
    pub content_type: ContentType,
    pub status_code: Option<i16>,
    pub is_internal: bool,
    pub depth: i32,

    // Extracted fields (populated during parse)
    pub title: Option<String>,
    pub title_length: Option<i32>,
    pub title_pixel_width: Option<i32>,
    pub meta_description: Option<String>,
    pub meta_description_length: Option<i32>,
    pub meta_description_pixel_width: Option<i32>,
    pub h1_first: Option<String>,
    pub h1_count: i32,
    pub h2_first: Option<String>,
    pub h2_count: i32,
    pub word_count: Option<i32>,
    pub response_time_ms: Option<i64>,
    pub content_length: Option<i64>,
    pub redirect_url: Option<String>,
    pub canonical_url: Option<String>,
    pub meta_robots: Option<String>,

    pub crawled_at: Option<DateTime<Utc>>,
}
