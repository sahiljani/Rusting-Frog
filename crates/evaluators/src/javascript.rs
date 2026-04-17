//! JavaScript tab evaluator (thin online slice).
//!
//! Most JavaScript filters compare a page's raw HTML against its
//! rendered DOM (title only visible after JS runs, canonical mismatch
//! between pre- and post-render, etc.) and therefore require the
//! headless-Chromium render worker. Those are deferred to the render
//! phase.
//!
//! Implemented online:
//! - `JavaScriptAll` — every HTML page.
//! - `JavaScriptContainsJsContent` — page includes any `<script>` tag.
//! - `JavaScriptOldAjaxCrawlingScheme` — URL uses the deprecated
//!   `#!` hashbang fragment or the `_escaped_fragment_` query parameter.
//! - `JavaScriptOldAjaxCrawlingMetaFragmentTag` — page has
//!   `<meta name="fragment" content="!">`.
//!
//! Deferred (need render-worker output):
//! `JavaScriptTitleJsOnly`, `JavaScriptTitleJsUpdated`, `H1JsOnly`,
//! `H1JsUpdated`, `MetadescriptionJsOnly`, `MetadescriptionJsUpdated`,
//! `CanonicalJsOnly`, `CanonicalMismatch`, `NoIndexOnlyInHtml`,
//! `NoFollowOnlyInHtml`, `ContainsJsLinks`, `PagesWithBlockedResources`,
//! `PagesWithJsErrors`, `PagesWithJsWarnings`, `PagesWithChromeIssues`.

use scraper::Selector;
use sf_core::crawl::{ContentType, CrawlUrl};
use sf_core::filter_key::FilterKey;
use sf_core::tab::TabKey;

use crate::{EvalContext, Evaluator, Finding};

pub struct JavaScriptEvaluator;

impl Evaluator for JavaScriptEvaluator {
    fn tab(&self) -> TabKey {
        TabKey::JavaScript
    }

    fn evaluate(&self, url: &CrawlUrl, ctx: &EvalContext) -> Vec<Finding> {
        if url.content_type != ContentType::Html {
            return vec![];
        }

        let mut findings = vec![Finding {
            filter_key: FilterKey::JavaScriptAll,
        }];

        if url.url.contains("#!") || url.url.contains("_escaped_fragment_=") {
            findings.push(Finding {
                filter_key: FilterKey::JavaScriptOldAjaxCrawlingScheme,
            });
        }

        let parsed = match ctx.parsed {
            Some(p) => p,
            None => return findings,
        };

        if let Some(sel) = Selector::parse("script").ok() {
            if parsed.select(&sel).next().is_some() {
                findings.push(Finding {
                    filter_key: FilterKey::JavaScriptContainsJsContent,
                });
            }
        }

        if let Some(sel) = Selector::parse(r#"meta[name="fragment"]"#).ok() {
            let has_fragment_meta = parsed.select(&sel).any(|el| {
                el.value()
                    .attr("content")
                    .map(|c| c.trim() == "!")
                    .unwrap_or(false)
            });
            if has_fragment_meta {
                findings.push(Finding {
                    filter_key: FilterKey::JavaScriptOldAjaxCrawlingMetaFragmentTag,
                });
            }
        }

        findings
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

    fn page(url: &str) -> CrawlUrl {
        CrawlUrl {
            id: CrawlUrlId::new(),
            crawl_id: CrawlId::new(),
            url: url.to_string(),
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

    fn eval(u: &CrawlUrl, html: &str) -> Vec<FilterKey> {
        let cfg = CrawlConfig::default();
        let parsed = Html::parse_document(html);
        let ctx = EvalContext {
            config: &cfg,
            html: Some(html),
            parsed: Some(&parsed),
        };
        JavaScriptEvaluator
            .evaluate(u, &ctx)
            .into_iter()
            .map(|f| f.filter_key)
            .collect()
    }

    #[test]
    fn page_with_script_tag_flagged() {
        let keys = eval(
            &page("https://example.com/"),
            "<html><body><script>x()</script></body></html>",
        );
        assert!(keys.contains(&FilterKey::JavaScriptContainsJsContent));
    }

    #[test]
    fn hashbang_url_flagged() {
        let keys = eval(&page("https://example.com/#!/foo"), "<html></html>");
        assert!(keys.contains(&FilterKey::JavaScriptOldAjaxCrawlingScheme));
    }

    #[test]
    fn escaped_fragment_flagged() {
        let keys = eval(
            &page("https://example.com/?_escaped_fragment_=foo"),
            "<html></html>",
        );
        assert!(keys.contains(&FilterKey::JavaScriptOldAjaxCrawlingScheme));
    }

    #[test]
    fn meta_fragment_flagged() {
        let keys = eval(
            &page("https://example.com/"),
            r#"<html><head><meta name="fragment" content="!"></head></html>"#,
        );
        assert!(keys.contains(&FilterKey::JavaScriptOldAjaxCrawlingMetaFragmentTag));
    }
}
