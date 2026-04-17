//! Parity tab evaluator (stub — emits the "all" bucket only).
//!
//! Every meaningful Parity filter compares a URL's raw-HTML
//! extraction against its rendered-DOM extraction:
//! word-count parity, title parity, H1 parity, inlink parity,
//! structured-data type parity, content-change parity, and so on.
//! All of those require the headless-Chromium render worker to
//! produce a second `CrawlUrl` snapshot so the two can be diffed.
//!
//! Until the render worker is wired into the evaluator pipeline,
//! we simply flag every HTML page with `ParityAll` so the tab's
//! total count tracks candidate URLs. The 14 diff-specific
//! filters (word-count, crawl-depth, indexability, page-titles,
//! H1, meta-description, inlinks, unique-inlinks,
//! internal-outlinks, unique-internal-outlinks, external-outlinks,
//! unique-external-outlinks, structured-data-unique-types,
//! content-change) are intentionally deferred.

use sf_core::crawl::{ContentType, CrawlUrl};
use sf_core::filter_key::FilterKey;
use sf_core::tab::TabKey;

use crate::{EvalContext, Evaluator, Finding};

pub struct ParityEvaluator;

impl Evaluator for ParityEvaluator {
    fn tab(&self) -> TabKey {
        TabKey::Parity
    }

    fn evaluate(&self, url: &CrawlUrl, _ctx: &EvalContext) -> Vec<Finding> {
        if url.content_type != ContentType::Html {
            return vec![];
        }
        vec![Finding {
            filter_key: FilterKey::ParityAll,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sf_core::config::CrawlConfig;
    use sf_core::crawl::{ContentType, CrawlUrl};
    use sf_core::id::{CrawlId, CrawlUrlId};

    fn page(ct: ContentType) -> CrawlUrl {
        CrawlUrl {
            id: CrawlUrlId::new(),
            crawl_id: CrawlId::new(),
            url: "https://example.com/".to_string(),
            url_hash: "h".to_string(),
            content_type: ct,
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

    #[test]
    fn html_page_fires_parity_all() {
        let cfg = CrawlConfig::default();
        let ctx = EvalContext {
            config: &cfg,
            html: None,
            parsed: None,
        };
        let findings = ParityEvaluator.evaluate(&page(ContentType::Html), &ctx);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].filter_key, FilterKey::ParityAll);
    }

    #[test]
    fn non_html_page_fires_nothing() {
        let cfg = CrawlConfig::default();
        let ctx = EvalContext {
            config: &cfg,
            html: None,
            parsed: None,
        };
        let findings = ParityEvaluator.evaluate(&page(ContentType::Image), &ctx);
        assert!(findings.is_empty());
    }
}
