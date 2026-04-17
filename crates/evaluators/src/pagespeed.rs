//! PageSpeed tab evaluator (stub — emits the "all" bucket only).
//!
//! Every meaningful PageSpeed filter depends on a PageSpeed Insights
//! (Lighthouse) audit for the URL. Those audits are fetched from
//! Google's PSI API in the full Screaming Frog product, which is an
//! out-of-scope third-party integration for this phase (see the
//! project roadmap for Batch 9).
//!
//! Until the PSI integration lands, the evaluator simply flags every
//! HTML page with `PagespeedAll` so the tab's total count tracks
//! candidate URLs. All 37 audit-specific filters
//! (unminified-css, render-blocking, LCP discovery, legacy-js, …)
//! are intentionally deferred.

use sf_core::crawl::{ContentType, CrawlUrl};
use sf_core::filter_key::FilterKey;
use sf_core::tab::TabKey;

use crate::{EvalContext, Evaluator, Finding};

pub struct PageSpeedEvaluator;

impl Evaluator for PageSpeedEvaluator {
    fn tab(&self) -> TabKey {
        TabKey::PageSpeed
    }

    fn evaluate(&self, url: &CrawlUrl, _ctx: &EvalContext) -> Vec<Finding> {
        if url.content_type != ContentType::Html {
            return vec![];
        }
        vec![Finding {
            filter_key: FilterKey::PagespeedAll,
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
    fn html_page_fires_pagespeed_all() {
        let cfg = CrawlConfig::default();
        let ctx = EvalContext {
            config: &cfg,
            html: None,
            parsed: None,
        };
        let findings = PageSpeedEvaluator.evaluate(&page(ContentType::Html), &ctx);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].filter_key, FilterKey::PagespeedAll);
    }

    #[test]
    fn non_html_page_fires_nothing() {
        let cfg = CrawlConfig::default();
        let ctx = EvalContext {
            config: &cfg,
            html: None,
            parsed: None,
        };
        let findings = PageSpeedEvaluator.evaluate(&page(ContentType::Image), &ctx);
        assert!(findings.is_empty());
    }
}
