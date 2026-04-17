//! Stub evaluators for tabs that are blocked on third-party or
//! user-driven integrations.
//!
//! Each of these tabs exists in the Screaming Frog product only
//! once a corresponding data source has been connected:
//!
//! | Tab | Blocking dependency |
//! |-----|---------------------|
//! | Analytics | Google Analytics 4 API (per-tenant OAuth) |
//! | SearchConsole | Google Search Console API (per-tenant OAuth) |
//! | LinkMetrics | Moz / Majestic / Ahrefs API (paid, per-tenant keys) |
//! | CustomSearch | User-authored regex/string slots (1..100) |
//! | CustomExtraction | User-authored XPath/CSS/regex slots (1..100) |
//! | CustomJavaScript | User-authored JS snippets (1..100), run in render worker |
//! | Ai | User-configured AI provider + prompts (1..100) |
//!
//! Until those integrations ship, the evaluators return no
//! findings. Registering them still serves two purposes: the tab
//! catalog stays aligned with the 33-tab registry, and the
//! evaluator pipeline hits zero-cost code paths for these tabs
//! instead of silently dropping URLs through an un-matched tab.
//!
//! This file will be split into `analytics.rs`, `search_console.rs`,
//! etc. once the real implementations start landing; keeping them
//! co-located here while they are all no-ops avoids cluttering the
//! module list with seven stub files.

use sf_core::crawl::CrawlUrl;
use sf_core::tab::TabKey;

use crate::{EvalContext, Evaluator, Finding};

macro_rules! integration_stub {
    ($name:ident, $tab:expr) => {
        pub struct $name;
        impl Evaluator for $name {
            fn tab(&self) -> TabKey {
                $tab
            }
            fn evaluate(&self, _url: &CrawlUrl, _ctx: &EvalContext) -> Vec<Finding> {
                Vec::new()
            }
        }
    };
}

integration_stub!(AnalyticsEvaluator, TabKey::Analytics);
integration_stub!(SearchConsoleEvaluator, TabKey::SearchConsole);
integration_stub!(LinkMetricsEvaluator, TabKey::LinkMetrics);
integration_stub!(CustomSearchEvaluator, TabKey::CustomSearch);
integration_stub!(CustomExtractionEvaluator, TabKey::CustomExtraction);
integration_stub!(CustomJavaScriptEvaluator, TabKey::CustomJavaScript);
integration_stub!(AiEvaluator, TabKey::Ai);

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sf_core::config::CrawlConfig;
    use sf_core::crawl::{ContentType, CrawlUrl};
    use sf_core::id::{CrawlId, CrawlUrlId};

    fn page() -> CrawlUrl {
        CrawlUrl {
            id: CrawlUrlId::new(),
            crawl_id: CrawlId::new(),
            url: "https://example.com/".to_string(),
            url_hash: "h".to_string(),
            content_type: ContentType::Html,
            status_code: Some(200),
            is_internal: true,
            depth: 0,
            title: None,
            title_length: None,
            title_pixel_width: None,
            meta_description: None,
            meta_description_length: None,
            meta_description_pixel_width: None,
            h1_first: None,
            h1_count: 0,
            h2_first: None,
            h2_count: 0,
            word_count: None,
            response_time_ms: None,
            content_length: None,
            redirect_url: None,
            canonical_url: None,
            meta_robots: None,
            crawled_at: Some(Utc::now()),
        }
    }

    fn empty_findings<E: Evaluator>(e: E, expected_tab: TabKey) {
        let cfg = CrawlConfig::default();
        let ctx = EvalContext {
            config: &cfg,
            html: None,
            parsed: None,
        };
        assert_eq!(e.tab(), expected_tab);
        assert!(e.evaluate(&page(), &ctx).is_empty());
    }

    #[test]
    fn analytics_emits_nothing() {
        empty_findings(AnalyticsEvaluator, TabKey::Analytics);
    }

    #[test]
    fn search_console_emits_nothing() {
        empty_findings(SearchConsoleEvaluator, TabKey::SearchConsole);
    }

    #[test]
    fn link_metrics_emits_nothing() {
        empty_findings(LinkMetricsEvaluator, TabKey::LinkMetrics);
    }

    #[test]
    fn custom_search_emits_nothing() {
        empty_findings(CustomSearchEvaluator, TabKey::CustomSearch);
    }

    #[test]
    fn custom_extraction_emits_nothing() {
        empty_findings(CustomExtractionEvaluator, TabKey::CustomExtraction);
    }

    #[test]
    fn custom_javascript_emits_nothing() {
        empty_findings(CustomJavaScriptEvaluator, TabKey::CustomJavaScript);
    }

    #[test]
    fn ai_emits_nothing() {
        empty_findings(AiEvaluator, TabKey::Ai);
    }
}
