//! Pagination tab evaluator.
//!
//! Parses `<link rel="prev">` / `<link rel="next">` and the corresponding
//! `<a rel="prev|next">` anchors. Fires per-page findings based on what
//! pagination markup the page contains:
//!
//! - `PaginationAll` — every HTML page.
//! - `PaginationContainsPagination` — page has any rel=prev or rel=next.
//! - `PaginationFirstPage` — has rel=next but no rel=prev (first in a
//!   paginated series).
//! - `PaginationPaginated2Plus` — has rel=prev (this is page 2+ of a series).
//! - `PaginationPaginationNotInAnchor` — page has a `<link rel="prev|next">`
//!   in the head, but no matching `<a rel="prev|next">` anchor in the body.
//!   SF surfaces this because some crawlers (and Google post-2019) only
//!   follow anchor-based pagination.
//! - `PaginationMultiplePaginationUrls` — more than one rel=prev or more
//!   than one rel=next on a single page.
//!
//! Cross-URL filters (`PaginationNon200PaginationUrls`,
//! `PaginationUnlinkedPaginationUrls`, `PaginationNonIndexable`,
//! `PaginationPaginationLoop`, `PaginationSequenceError`) need the full
//! crawl graph and are deferred to post-crawl analysis.

use scraper::Selector;
use sf_core::crawl::{ContentType, CrawlUrl};
use sf_core::filter_key::FilterKey;
use sf_core::tab::TabKey;

use crate::{EvalContext, Evaluator, Finding};

pub struct PaginationEvaluator;

impl Evaluator for PaginationEvaluator {
    fn tab(&self) -> TabKey {
        TabKey::Pagination
    }

    fn evaluate(&self, url: &CrawlUrl, ctx: &EvalContext) -> Vec<Finding> {
        if url.content_type != ContentType::Html {
            return vec![];
        }

        let mut findings = vec![Finding {
            filter_key: FilterKey::PaginationAll,
        }];

        let parsed = match ctx.parsed {
            Some(p) => p,
            None => return findings,
        };

        let link_prev = selector_count(parsed, r#"link[rel~="prev"]"#);
        let link_next = selector_count(parsed, r#"link[rel~="next"]"#);
        let a_prev = selector_count(parsed, r#"a[rel~="prev"]"#);
        let a_next = selector_count(parsed, r#"a[rel~="next"]"#);

        let has_prev = link_prev > 0 || a_prev > 0;
        let has_next = link_next > 0 || a_next > 0;

        if has_prev || has_next {
            findings.push(Finding {
                filter_key: FilterKey::PaginationContainsPagination,
            });
        }

        if has_next && !has_prev {
            findings.push(Finding {
                filter_key: FilterKey::PaginationFirstPage,
            });
        }
        if has_prev {
            findings.push(Finding {
                filter_key: FilterKey::PaginationPaginated2Plus,
            });
        }

        if link_prev > 1 || link_next > 1 || a_prev > 1 || a_next > 1 {
            findings.push(Finding {
                filter_key: FilterKey::PaginationMultiplePaginationUrls,
            });
        }

        // "Pagination not in anchor" — <link> signals pagination but no
        // corresponding <a rel="..."> anchor exists for the same relation.
        let link_only_prev = link_prev > 0 && a_prev == 0;
        let link_only_next = link_next > 0 && a_next == 0;
        if link_only_prev || link_only_next {
            findings.push(Finding {
                filter_key: FilterKey::PaginationPaginationNotInAnchor,
            });
        }

        findings
    }
}

fn selector_count(doc: &scraper::Html, sel: &str) -> usize {
    match Selector::parse(sel) {
        Ok(s) => doc.select(&s).count(),
        Err(_) => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use scraper::Html;
    use sf_core::config::CrawlConfig;
    use sf_core::crawl::{ContentType, CrawlUrl};
    use sf_core::id::{CrawlId, CrawlUrlId};

    fn page() -> CrawlUrl {
        CrawlUrl {
            id: CrawlUrlId::new(),
            crawl_id: CrawlId::new(),
            url: "https://example.com/p?page=2".to_string(),
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

    fn eval(html: &str) -> Vec<FilterKey> {
        let cfg = CrawlConfig::default();
        let parsed = Html::parse_document(html);
        let ctx = EvalContext {
            config: &cfg,
            html: Some(html),
            parsed: Some(&parsed),
        };
        PaginationEvaluator
            .evaluate(&page(), &ctx)
            .into_iter()
            .map(|f| f.filter_key)
            .collect()
    }

    #[test]
    fn every_html_page_emits_all() {
        let keys = eval("<html><body></body></html>");
        assert!(keys.contains(&FilterKey::PaginationAll));
    }

    #[test]
    fn non_html_emits_nothing() {
        let cfg = CrawlConfig::default();
        let parsed = Html::parse_document("");
        let ctx = EvalContext {
            config: &cfg,
            html: Some(""),
            parsed: Some(&parsed),
        };
        let mut u = page();
        u.content_type = ContentType::Image;
        let keys = PaginationEvaluator.evaluate(&u, &ctx);
        assert!(keys.is_empty());
    }

    #[test]
    fn page_without_pagination_does_not_fire() {
        let keys = eval("<html><head></head><body></body></html>");
        assert!(!keys.contains(&FilterKey::PaginationContainsPagination));
        assert!(!keys.contains(&FilterKey::PaginationFirstPage));
        assert!(!keys.contains(&FilterKey::PaginationPaginated2Plus));
    }

    #[test]
    fn first_page_has_next_no_prev() {
        let html = r#"<html><head>
            <link rel="next" href="/page/2">
        </head></html>"#;
        let keys = eval(html);
        assert!(keys.contains(&FilterKey::PaginationFirstPage));
        assert!(!keys.contains(&FilterKey::PaginationPaginated2Plus));
    }

    #[test]
    fn second_page_has_prev() {
        let html = r#"<html><head>
            <link rel="prev" href="/page/1">
            <link rel="next" href="/page/3">
        </head></html>"#;
        let keys = eval(html);
        assert!(keys.contains(&FilterKey::PaginationPaginated2Plus));
        assert!(!keys.contains(&FilterKey::PaginationFirstPage));
    }

    #[test]
    fn link_only_flagged_not_in_anchor() {
        let html = r#"<html><head><link rel="next" href="/page/2"></head>
            <body><p>no anchor</p></body></html>"#;
        let keys = eval(html);
        assert!(keys.contains(&FilterKey::PaginationPaginationNotInAnchor));
    }

    #[test]
    fn anchor_pagination_does_not_trip_not_in_anchor() {
        let html = r#"<html><head></head>
            <body><a rel="next" href="/page/2">next</a></body></html>"#;
        let keys = eval(html);
        assert!(!keys.contains(&FilterKey::PaginationPaginationNotInAnchor));
        assert!(keys.contains(&FilterKey::PaginationContainsPagination));
    }

    #[test]
    fn multiple_next_links_flagged() {
        let html = r#"<html><head>
            <link rel="next" href="/a">
            <link rel="next" href="/b">
        </head></html>"#;
        let keys = eval(html);
        assert!(keys.contains(&FilterKey::PaginationMultiplePaginationUrls));
    }
}
