//! Validation tab evaluator (online subset).
//!
//! Catches structural HTML issues — missing/duplicate head or body,
//! head not first under html, forbidden tags inside `<head>`, and
//! oversized documents.
//!
//! - `ValidationAll` — every HTML page.
//! - `ValidationMissingHead` — zero `<head>` elements.
//! - `ValidationMultipleHeads` — two or more `<head>` elements.
//! - `ValidationMissingBody` — zero `<body>` elements.
//! - `ValidationMultipleBodies` — two or more `<body>` elements.
//! - `ValidationHeadNotFirstElement` — the first element-child of
//!   `<html>` is not `<head>` (e.g. `<body>` comes first).
//! - `ValidationInvalidElementsInHead` — `<head>` contains a tag that
//!   shouldn't be there (anything other than the standard head-level
//!   elements).
//! - `ValidationDocumentOver15Mb` — response `content_length` > 15 MiB.
//!
//! Deferred: `ValidationResourceOver15Mb` (cross-URL, needs subresource
//! sizes), `ValidationHighCarbonRating` (external API),
//! `ValidationBodyElementPrecedingHtml` (browser parsing always
//! re-roots content inside `<html>`, so this is effectively dead).

use scraper::Selector;
use sf_core::crawl::{ContentType, CrawlUrl};
use sf_core::filter_key::FilterKey;
use sf_core::tab::TabKey;

use crate::{EvalContext, Evaluator, Finding};

const FIFTEEN_MIB: i64 = 15 * 1024 * 1024;

/// Tags that may legally appear directly inside `<head>`.
const HEAD_LEGAL_CHILDREN: &[&str] = &[
    "title", "base", "link", "meta", "style", "script", "noscript", "template",
];

pub struct ValidationEvaluator;

impl Evaluator for ValidationEvaluator {
    fn tab(&self) -> TabKey {
        TabKey::Validation
    }

    fn evaluate(&self, url: &CrawlUrl, ctx: &EvalContext) -> Vec<Finding> {
        if url.content_type != ContentType::Html {
            return vec![];
        }

        let mut findings = vec![Finding {
            filter_key: FilterKey::ValidationAll,
        }];

        if let Some(len) = url.content_length {
            if len > FIFTEEN_MIB {
                findings.push(Finding {
                    filter_key: FilterKey::ValidationDocumentOver15Mb,
                });
            }
        }

        let parsed = match ctx.parsed {
            Some(p) => p,
            None => return findings,
        };

        let head_sel = Selector::parse("head").ok();
        let body_sel = Selector::parse("body").ok();
        let html_sel = Selector::parse("html").ok();

        let head_count = head_sel
            .as_ref()
            .map(|s| parsed.select(s).count())
            .unwrap_or(0);
        let body_count = body_sel
            .as_ref()
            .map(|s| parsed.select(s).count())
            .unwrap_or(0);

        if head_count == 0 {
            findings.push(Finding {
                filter_key: FilterKey::ValidationMissingHead,
            });
        } else if head_count > 1 {
            findings.push(Finding {
                filter_key: FilterKey::ValidationMultipleHeads,
            });
        }

        if body_count == 0 {
            findings.push(Finding {
                filter_key: FilterKey::ValidationMissingBody,
            });
        } else if body_count > 1 {
            findings.push(Finding {
                filter_key: FilterKey::ValidationMultipleBodies,
            });
        }

        // head-not-first: inspect first element child of <html>.
        if let Some(sel) = &html_sel {
            if let Some(html_el) = parsed.select(sel).next() {
                let first_child = html_el
                    .children()
                    .filter_map(scraper::ElementRef::wrap)
                    .next();
                if let Some(child) = first_child {
                    if child.value().name() != "head" && head_count > 0 {
                        findings.push(Finding {
                            filter_key: FilterKey::ValidationHeadNotFirstElement,
                        });
                    }
                }
            }
        }

        // Invalid tags inside <head>. html5ever relocates illegal tags
        // out of <head> into <body>, so inspect the raw HTML substring
        // between the first `<head` open and its matching `</head>`.
        if let Some(raw) = ctx.html {
            if has_invalid_head_children(raw) {
                findings.push(Finding {
                    filter_key: FilterKey::ValidationInvalidElementsInHead,
                });
            }
        }

        findings
    }
}

fn has_invalid_head_children(html: &str) -> bool {
    let lower = html.to_ascii_lowercase();
    let Some(head_start) = lower.find("<head") else {
        return false;
    };
    let Some(head_open_end) = lower[head_start..].find('>') else {
        return false;
    };
    let inner_start = head_start + head_open_end + 1;
    let Some(head_close_rel) = lower[inner_start..].find("</head") else {
        return false;
    };
    let inner = &lower[inner_start..inner_start + head_close_rel];

    let mut i = 0;
    let bytes = inner.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'<' {
            let rest = &inner[i + 1..];
            let is_closing = rest.starts_with('/');
            let is_comment = rest.starts_with("!--");
            if is_comment {
                if let Some(end) = inner[i..].find("-->") {
                    i += end + 3;
                    continue;
                } else {
                    break;
                }
            }
            if is_closing {
                i += 1;
                continue;
            }
            let name_start = i + 1;
            let mut name_end = name_start;
            while name_end < bytes.len() {
                let c = bytes[name_end];
                if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == b'>' || c == b'/' {
                    break;
                }
                name_end += 1;
            }
            let name = &inner[name_start..name_end];
            if !name.is_empty() && !HEAD_LEGAL_CHILDREN.contains(&name) {
                return true;
            }
            i = name_end;
        } else {
            i += 1;
        }
    }
    false
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
        ValidationEvaluator
            .evaluate(&page(), &ctx)
            .into_iter()
            .map(|f| f.filter_key)
            .collect()
    }

    #[test]
    fn well_formed_page_only_emits_all() {
        let keys = eval("<html><head><title>t</title></head><body></body></html>");
        assert_eq!(keys, vec![FilterKey::ValidationAll]);
    }

    #[test]
    fn invalid_head_child_flagged() {
        let html = "<html><head><div>bad</div></head><body></body></html>";
        let keys = eval(html);
        assert!(keys.contains(&FilterKey::ValidationInvalidElementsInHead));
    }

    #[test]
    fn oversized_document_flagged() {
        let cfg = CrawlConfig::default();
        let mut u = page();
        u.content_length = Some(16 * 1024 * 1024);
        let html = "<html><head></head><body></body></html>";
        let parsed = Html::parse_document(html);
        let ctx = EvalContext {
            config: &cfg,
            html: Some(html),
            parsed: Some(&parsed),
        };
        let keys: Vec<_> = ValidationEvaluator
            .evaluate(&u, &ctx)
            .into_iter()
            .map(|f| f.filter_key)
            .collect();
        assert!(keys.contains(&FilterKey::ValidationDocumentOver15Mb));
    }

    #[test]
    fn non_html_emits_nothing() {
        let cfg = CrawlConfig::default();
        let mut u = page();
        u.content_type = ContentType::Image;
        let ctx = EvalContext {
            config: &cfg,
            html: None,
            parsed: None,
        };
        let keys = ValidationEvaluator.evaluate(&u, &ctx);
        assert!(keys.is_empty());
    }
}
