//! Directives tab evaluator.
//!
//! Parses the `meta name="robots"` directive string (already extracted by
//! the parser into `CrawlUrl.meta_robots`) plus a few other head-level
//! signals:
//!
//! - Per-token membership fires one finding each:
//!   `DirectivesIndex`, `DirectivesNoindex`, `DirectivesFollow`,
//!   `DirectivesNofollow`, `DirectivesNone`, `DirectivesNoarchive`,
//!   `DirectivesNosnippet`, `DirectivesMaxSnippet`,
//!   `DirectivesMaxImagePreview`, `DirectivesMaxVideoPreview`,
//!   `DirectivesNoodp`, `DirectivesNoydir`, `DirectivesNoimageindex`,
//!   `DirectivesNotranslate`, `DirectivesUnavailableAfter`.
//!
//! - `DirectivesAll` — every HTML page.
//! - `DirectivesRefresh` — page has `<meta http-equiv="refresh">`.
//! - `DirectivesOutsideHead` — the robots `<meta>` sits outside `<head>`.

use scraper::Selector;
use sf_core::crawl::{ContentType, CrawlUrl};
use sf_core::filter_key::FilterKey;
use sf_core::tab::TabKey;

use crate::{EvalContext, Evaluator, Finding};

pub struct DirectivesEvaluator;

impl Evaluator for DirectivesEvaluator {
    fn tab(&self) -> TabKey {
        TabKey::Directives
    }

    fn evaluate(&self, url: &CrawlUrl, ctx: &EvalContext) -> Vec<Finding> {
        if url.content_type != ContentType::Html {
            return vec![];
        }

        let mut findings = vec![Finding {
            filter_key: FilterKey::DirectivesAll,
        }];

        let robots = url
            .meta_robots
            .as_deref()
            .unwrap_or("")
            .to_ascii_lowercase();

        // Token set — comma-delimited, whitespace-tolerant. Some tokens
        // carry a value (max-snippet:10) but the key is what we match on.
        let tokens: Vec<&str> = robots
            .split(',')
            .map(|t| t.trim())
            .filter(|t| !t.is_empty())
            .collect();

        let has = |needle: &str| tokens.iter().any(|t| t == &needle || t.starts_with(needle));

        if has("index") && !has("noindex") {
            findings.push(Finding {
                filter_key: FilterKey::DirectivesIndex,
            });
        }
        if has("noindex") {
            findings.push(Finding {
                filter_key: FilterKey::DirectivesNoindex,
            });
        }
        if has("follow") && !has("nofollow") {
            findings.push(Finding {
                filter_key: FilterKey::DirectivesFollow,
            });
        }
        if has("nofollow") {
            findings.push(Finding {
                filter_key: FilterKey::DirectivesNofollow,
            });
        }
        if has("none") {
            findings.push(Finding {
                filter_key: FilterKey::DirectivesNone,
            });
        }
        if has("noarchive") {
            findings.push(Finding {
                filter_key: FilterKey::DirectivesNoarchive,
            });
        }
        if has("nosnippet") {
            findings.push(Finding {
                filter_key: FilterKey::DirectivesNosnippet,
            });
        }
        if has("max-snippet") {
            findings.push(Finding {
                filter_key: FilterKey::DirectivesMaxSnippet,
            });
        }
        if has("max-image-preview") {
            findings.push(Finding {
                filter_key: FilterKey::DirectivesMaxImagePreview,
            });
        }
        if has("max-video-preview") {
            findings.push(Finding {
                filter_key: FilterKey::DirectivesMaxVideoPreview,
            });
        }
        if has("noodp") {
            findings.push(Finding {
                filter_key: FilterKey::DirectivesNoodp,
            });
        }
        if has("noydir") {
            findings.push(Finding {
                filter_key: FilterKey::DirectivesNoydir,
            });
        }
        if has("noimageindex") {
            findings.push(Finding {
                filter_key: FilterKey::DirectivesNoimageindex,
            });
        }
        if has("notranslate") {
            findings.push(Finding {
                filter_key: FilterKey::DirectivesNotranslate,
            });
        }
        if has("unavailable_after") {
            findings.push(Finding {
                filter_key: FilterKey::DirectivesUnavailableAfter,
            });
        }

        if let Some(parsed) = ctx.parsed {
            if has_meta_refresh(parsed) {
                findings.push(Finding {
                    filter_key: FilterKey::DirectivesRefresh,
                });
            }
            if robots_meta_outside_head(parsed) {
                findings.push(Finding {
                    filter_key: FilterKey::DirectivesOutsideHead,
                });
            }
        }

        findings
    }
}

fn has_meta_refresh(doc: &scraper::Html) -> bool {
    match Selector::parse(r#"meta[http-equiv]"#) {
        Ok(sel) => doc.select(&sel).any(|el| {
            el.value()
                .attr("http-equiv")
                .map(|v| v.eq_ignore_ascii_case("refresh"))
                .unwrap_or(false)
        }),
        Err(_) => false,
    }
}

fn robots_meta_outside_head(doc: &scraper::Html) -> bool {
    let sel = match Selector::parse(r#"meta[name]"#) {
        Ok(s) => s,
        Err(_) => return false,
    };
    for el in doc.select(&sel) {
        let name_lc = el.value().attr("name").unwrap_or("").to_ascii_lowercase();
        if name_lc != "robots" {
            continue;
        }
        let mut node = el.parent();
        let mut in_head = false;
        while let Some(n) = node {
            if let Some(e) = scraper::ElementRef::wrap(n) {
                if e.value().name() == "head" {
                    in_head = true;
                    break;
                }
                node = e.parent();
            } else {
                node = None;
            }
        }
        if !in_head {
            return true;
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

    fn page_with_robots(robots: Option<&str>) -> CrawlUrl {
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
            meta_robots: robots.map(String::from),
            crawled_at: Some(Utc::now()),
        }
    }

    fn eval(robots: Option<&str>, html: &str) -> Vec<FilterKey> {
        let cfg = CrawlConfig::default();
        let parsed = Html::parse_document(html);
        let ctx = EvalContext {
            config: &cfg,
            html: Some(html),
            parsed: Some(&parsed),
        };
        DirectivesEvaluator
            .evaluate(&page_with_robots(robots), &ctx)
            .into_iter()
            .map(|f| f.filter_key)
            .collect()
    }

    #[test]
    fn every_html_page_emits_all() {
        let keys = eval(None, "<html><head></head></html>");
        assert!(keys.contains(&FilterKey::DirectivesAll));
    }

    #[test]
    fn noindex_and_nofollow_flagged() {
        let keys = eval(Some("noindex, nofollow"), "<html><head></head></html>");
        assert!(keys.contains(&FilterKey::DirectivesNoindex));
        assert!(keys.contains(&FilterKey::DirectivesNofollow));
        assert!(!keys.contains(&FilterKey::DirectivesIndex));
        assert!(!keys.contains(&FilterKey::DirectivesFollow));
    }

    #[test]
    fn index_and_follow_flagged_when_positive() {
        let keys = eval(Some("index, follow"), "<html><head></head></html>");
        assert!(keys.contains(&FilterKey::DirectivesIndex));
        assert!(keys.contains(&FilterKey::DirectivesFollow));
        assert!(!keys.contains(&FilterKey::DirectivesNoindex));
    }

    #[test]
    fn max_snippet_with_value_flagged() {
        let keys = eval(Some("max-snippet:-1"), "<html><head></head></html>");
        assert!(keys.contains(&FilterKey::DirectivesMaxSnippet));
    }

    #[test]
    fn max_image_and_video_preview_flagged() {
        let keys = eval(
            Some("max-image-preview:large, max-video-preview:-1"),
            "<html><head></head></html>",
        );
        assert!(keys.contains(&FilterKey::DirectivesMaxImagePreview));
        assert!(keys.contains(&FilterKey::DirectivesMaxVideoPreview));
    }

    #[test]
    fn none_token_flagged() {
        let keys = eval(Some("none"), "<html><head></head></html>");
        assert!(keys.contains(&FilterKey::DirectivesNone));
    }

    #[test]
    fn meta_refresh_flagged() {
        let html = r#"<html><head><meta http-equiv="refresh" content="5;url=/x"></head></html>"#;
        let keys = eval(None, html);
        assert!(keys.contains(&FilterKey::DirectivesRefresh));
    }

    #[test]
    fn robots_outside_head_flagged() {
        let html =
            r#"<html><head></head><body><meta name="robots" content="noindex"></body></html>"#;
        let keys = eval(Some("noindex"), html);
        assert!(keys.contains(&FilterKey::DirectivesOutsideHead));
    }

    #[test]
    fn robots_inside_head_not_flagged_outside() {
        let html = r#"<html><head><meta name="robots" content="noindex"></head></html>"#;
        let keys = eval(Some("noindex"), html);
        assert!(!keys.contains(&FilterKey::DirectivesOutsideHead));
    }

    #[test]
    fn empty_robots_fires_only_all() {
        let keys = eval(None, "<html><head></head></html>");
        assert_eq!(keys, vec![FilterKey::DirectivesAll]);
    }
}
