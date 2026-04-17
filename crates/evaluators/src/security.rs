//! Security tab evaluator (online HTML-only subset).
//!
//! Evaluates filters decidable from the page URL + parsed HTML. Filters
//! that require response headers (HSTS, CSP, frame-options, content-type,
//! referrer-policy, bad-MIME) are deferred until the crawler persists
//! response headers on `CrawlUrl`.
//!
//! Implemented:
//! - `SecurityAll` — every HTML page.
//! - `SecurityHttp` / `SecurityHttps` — scheme-based classification.
//! - `SecurityMixedContent` — HTTPS page that loads an `http://` subresource.
//! - `SecurityFormOnHttpPage` — any `<form>` on a page served over HTTP.
//! - `SecurityFormUrlInsecure` — any `<form action="http://...">` (regardless
//!   of page scheme).
//! - `SecurityUnsafeCrossOrigin` — `<a target="_blank">` missing `rel`
//!   containing `noopener` or `noreferrer`.
//! - `SecurityProtocolRelativeUrls` — subresource or link whose URL starts
//!   with `//` (scheme-relative).
//!
//! Deferred (need response headers persisted):
//! `SecurityMissingHstsHeader`, `SecurityBadMimeType`,
//! `SecurityMissingContentTypeHeader`, `SecurityMissingFrameHeader`,
//! `SecurityMissingCspHeader`, `SecurityMissingSecureReferrerPolicy`.

use scraper::Selector;
use sf_core::crawl::{ContentType, CrawlUrl};
use sf_core::filter_key::FilterKey;
use sf_core::tab::TabKey;

use crate::{EvalContext, Evaluator, Finding};

pub struct SecurityEvaluator;

impl Evaluator for SecurityEvaluator {
    fn tab(&self) -> TabKey {
        TabKey::Security
    }

    fn evaluate(&self, url: &CrawlUrl, ctx: &EvalContext) -> Vec<Finding> {
        if url.content_type != ContentType::Html {
            return vec![];
        }

        let mut findings = vec![Finding {
            filter_key: FilterKey::SecurityAll,
        }];

        let is_http = url.url.starts_with("http://");
        let is_https = url.url.starts_with("https://");
        if is_http {
            findings.push(Finding {
                filter_key: FilterKey::SecurityHttp,
            });
        }
        if is_https {
            findings.push(Finding {
                filter_key: FilterKey::SecurityHttps,
            });
        }

        let parsed = match ctx.parsed {
            Some(p) => p,
            None => return findings,
        };

        // Selector for anything that carries a URL we care about.
        let subres_sel = Selector::parse(
            "img[src], script[src], iframe[src], link[href], source[src], video[src], audio[src]",
        )
        .ok();

        let mut saw_mixed = false;
        let mut saw_proto_rel = false;
        if let Some(sel) = &subres_sel {
            for el in parsed.select(sel) {
                let raw = el
                    .value()
                    .attr("src")
                    .or_else(|| el.value().attr("href"))
                    .unwrap_or("")
                    .trim();
                if raw.starts_with("//") {
                    saw_proto_rel = true;
                }
                if is_https && raw.starts_with("http://") {
                    saw_mixed = true;
                }
            }
        }
        if saw_mixed {
            findings.push(Finding {
                filter_key: FilterKey::SecurityMixedContent,
            });
        }
        if saw_proto_rel {
            findings.push(Finding {
                filter_key: FilterKey::SecurityProtocolRelativeUrls,
            });
        }

        let form_sel = Selector::parse("form").ok();
        let mut has_form = false;
        let mut saw_form_insecure = false;
        if let Some(sel) = &form_sel {
            for el in parsed.select(sel) {
                has_form = true;
                if let Some(action) = el.value().attr("action")
                    && action.trim().to_ascii_lowercase().starts_with("http://")
                {
                    saw_form_insecure = true;
                }
            }
        }
        if has_form && is_http {
            findings.push(Finding {
                filter_key: FilterKey::SecurityFormOnHttpPage,
            });
        }
        if saw_form_insecure {
            findings.push(Finding {
                filter_key: FilterKey::SecurityFormUrlInsecure,
            });
        }

        let a_sel = Selector::parse(r#"a[target="_blank"]"#).ok();
        let mut saw_unsafe = false;
        if let Some(sel) = &a_sel {
            for el in parsed.select(sel) {
                let rel = el.value().attr("rel").unwrap_or("").to_ascii_lowercase();
                let safe = rel
                    .split_ascii_whitespace()
                    .any(|t| t == "noopener" || t == "noreferrer");
                if !safe {
                    saw_unsafe = true;
                    break;
                }
            }
        }
        if saw_unsafe {
            findings.push(Finding {
                filter_key: FilterKey::SecurityUnsafeCrossOrigin,
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
        SecurityEvaluator
            .evaluate(u, &ctx)
            .into_iter()
            .map(|f| f.filter_key)
            .collect()
    }

    #[test]
    fn https_page_emits_https_not_http() {
        let keys = eval(&page("https://example.com/"), "<html></html>");
        assert!(keys.contains(&FilterKey::SecurityHttps));
        assert!(!keys.contains(&FilterKey::SecurityHttp));
    }

    #[test]
    fn mixed_content_flagged() {
        let html = r#"<html><body><img src="http://cdn.example.com/x.png"></body></html>"#;
        let keys = eval(&page("https://example.com/"), html);
        assert!(keys.contains(&FilterKey::SecurityMixedContent));
    }

    #[test]
    fn protocol_relative_flagged() {
        let html = r#"<html><body><script src="//cdn.example.com/x.js"></script></body></html>"#;
        let keys = eval(&page("https://example.com/"), html);
        assert!(keys.contains(&FilterKey::SecurityProtocolRelativeUrls));
    }

    #[test]
    fn form_on_http_page_flagged() {
        let html = r#"<html><body><form action="/submit"><input name="q"></form></body></html>"#;
        let keys = eval(&page("http://example.com/"), html);
        assert!(keys.contains(&FilterKey::SecurityFormOnHttpPage));
    }

    #[test]
    fn form_action_http_flagged_even_on_https_page() {
        let html = r#"<html><body><form action="http://example.com/submit"></form></body></html>"#;
        let keys = eval(&page("https://example.com/"), html);
        assert!(keys.contains(&FilterKey::SecurityFormUrlInsecure));
        assert!(!keys.contains(&FilterKey::SecurityFormOnHttpPage));
    }

    #[test]
    fn unsafe_cross_origin_flagged() {
        let html = r#"<html><body><a href="https://ext.com" target="_blank">ext</a></body></html>"#;
        let keys = eval(&page("https://example.com/"), html);
        assert!(keys.contains(&FilterKey::SecurityUnsafeCrossOrigin));
    }

    #[test]
    fn unsafe_cross_origin_not_flagged_with_noopener() {
        let html = r#"<html><body><a href="https://ext.com" target="_blank" rel="noopener">ext</a></body></html>"#;
        let keys = eval(&page("https://example.com/"), html);
        assert!(!keys.contains(&FilterKey::SecurityUnsafeCrossOrigin));
    }

    #[test]
    fn non_html_emits_nothing() {
        let mut u = page("https://example.com/x.png");
        u.content_type = ContentType::Image;
        let keys = eval(&u, "");
        assert!(keys.is_empty());
    }
}
