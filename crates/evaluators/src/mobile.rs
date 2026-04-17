//! Mobile tab evaluator (online subset — no rendering yet).
//!
//! - `MobileAll` — every HTML page.
//! - `MobileViewportNotSet` — no `<meta name="viewport">` in the page.
//! - `MobileMobileAlternateLink` — page declares a mobile alternate via
//!   `<link rel="alternate" media="..." href="...">` (typical SF pattern
//!   uses `media="only screen and (max-width: 640px)"`).
//!
//! Deferred (need headless-browser layout):
//! `MobileTargetSize`, `MobileContentNotSizedCorrectly`,
//! `MobileIllegibleFontSize`, `MobileUnsupportedPlugins` (plugin detection
//! could be done online from `<object>`/`<embed>`, but SF's definition
//! leans on rendered results — deferred to render worker).

use scraper::Selector;
use sf_core::crawl::{ContentType, CrawlUrl};
use sf_core::filter_key::FilterKey;
use sf_core::tab::TabKey;

use crate::{EvalContext, Evaluator, Finding};

pub struct MobileEvaluator;

impl Evaluator for MobileEvaluator {
    fn tab(&self) -> TabKey {
        TabKey::Mobile
    }

    fn evaluate(&self, url: &CrawlUrl, ctx: &EvalContext) -> Vec<Finding> {
        if url.content_type != ContentType::Html {
            return vec![];
        }

        let mut findings = vec![Finding {
            filter_key: FilterKey::MobileAll,
        }];

        let parsed = match ctx.parsed {
            Some(p) => p,
            None => return findings,
        };

        let viewport_sel = Selector::parse(r#"meta[name="viewport"]"#).ok();
        let has_viewport = viewport_sel
            .as_ref()
            .map(|s| parsed.select(s).next().is_some())
            .unwrap_or(false);
        if !has_viewport {
            findings.push(Finding {
                filter_key: FilterKey::MobileViewportNotSet,
            });
        }

        let alt_sel = Selector::parse(r#"link[rel="alternate"][media]"#).ok();
        let has_mobile_alt = alt_sel
            .as_ref()
            .map(|s| parsed.select(s).next().is_some())
            .unwrap_or(false);
        if has_mobile_alt {
            findings.push(Finding {
                filter_key: FilterKey::MobileMobileAlternateLink,
            });
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

    fn eval(html: &str) -> Vec<FilterKey> {
        let cfg = CrawlConfig::default();
        let parsed = Html::parse_document(html);
        let ctx = EvalContext {
            config: &cfg,
            html: Some(html),
            parsed: Some(&parsed),
        };
        MobileEvaluator
            .evaluate(&page(), &ctx)
            .into_iter()
            .map(|f| f.filter_key)
            .collect()
    }

    #[test]
    fn page_without_viewport_flagged() {
        let keys = eval("<html><head></head></html>");
        assert!(keys.contains(&FilterKey::MobileViewportNotSet));
    }

    #[test]
    fn page_with_viewport_not_flagged() {
        let keys = eval(
            r#"<html><head><meta name="viewport" content="width=device-width,initial-scale=1"></head></html>"#,
        );
        assert!(!keys.contains(&FilterKey::MobileViewportNotSet));
    }

    #[test]
    fn mobile_alternate_link_flagged() {
        let keys = eval(
            r#"<html><head>
              <link rel="alternate" media="only screen and (max-width: 640px)" href="https://m.example.com/">
            </head></html>"#,
        );
        assert!(keys.contains(&FilterKey::MobileMobileAlternateLink));
    }
}
