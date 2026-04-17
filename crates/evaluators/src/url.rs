//! URL tab evaluator.
//!
//! Everything in this tab is derivable from the URL string itself plus
//! the page's `CrawlUrl` metadata — no HTML parsing needed. Every
//! crawled URL (HTML or not) gets a chance to fire these filters.
//!
//! - `UrlAll` — every URL.
//! - `UrlNonAsciiCharacters` — codepoint > 0x7f anywhere in the string.
//! - `UrlUnderscores` — `_` in the path.
//! - `UrlUppercase` — any ASCII uppercase in the path or query.
//! - `UrlParameters` — URL has a query string.
//! - `UrlOverXCharacters` — length above `config.url.max_length`.
//! - `UrlMultipleSlashes` — `//` anywhere in the path (post-scheme).
//! - `UrlRepetitivePath` — same segment appears twice in a row.
//! - `UrlContainsSpace` — space or `%20` in path/query.
//! - `UrlInternalSearchUrl` — matches common site-search patterns.
//! - `UrlGaTrackingParameters` — query contains `utm_*`, `gclid`, or
//!   `fbclid`.
//!
//! `UrlBrokenBookmark` requires cross-URL knowledge (fragment must resolve
//! to an element on the target page) and is deferred to post-crawl.

use sf_core::crawl::CrawlUrl;
use sf_core::filter_key::FilterKey;
use sf_core::tab::TabKey;
use url::Url;

use crate::{EvalContext, Evaluator, Finding};

pub struct UrlEvaluator;

impl Evaluator for UrlEvaluator {
    fn tab(&self) -> TabKey {
        TabKey::Url
    }

    fn evaluate(&self, url: &CrawlUrl, ctx: &EvalContext) -> Vec<Finding> {
        let mut findings = vec![Finding {
            filter_key: FilterKey::UrlAll,
        }];

        let raw = &url.url;

        if raw.chars().any(|c| c as u32 > 0x7f) {
            findings.push(Finding {
                filter_key: FilterKey::UrlNonAsciiCharacters,
            });
        }

        if raw.len() as u32 > ctx.config.url.max_length {
            findings.push(Finding {
                filter_key: FilterKey::UrlOverXCharacters,
            });
        }

        // Most path/query checks need a parsed URL.
        let parsed = match Url::parse(raw) {
            Ok(u) => u,
            Err(_) => return findings,
        };

        let path = parsed.path();
        let query = parsed.query().unwrap_or("");

        if path.contains('_') {
            findings.push(Finding {
                filter_key: FilterKey::UrlUnderscores,
            });
        }

        if path.chars().any(|c| c.is_ascii_uppercase())
            || query.chars().any(|c| c.is_ascii_uppercase())
        {
            findings.push(Finding {
                filter_key: FilterKey::UrlUppercase,
            });
        }

        if parsed.query().is_some() {
            findings.push(Finding {
                filter_key: FilterKey::UrlParameters,
            });
        }

        // Normalise leading slash so "/" by itself doesn't count as //.
        if path.trim_start_matches('/').contains("//") || path.contains("//") && path.len() > 1 {
            // Same check, clarified: any double-slash inside the path.
            if path[1..].contains("//") {
                findings.push(Finding {
                    filter_key: FilterKey::UrlMultipleSlashes,
                });
            }
        }

        let segments: Vec<&str> = path
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();
        for pair in segments.windows(2) {
            if pair[0].eq_ignore_ascii_case(pair[1]) {
                findings.push(Finding {
                    filter_key: FilterKey::UrlRepetitivePath,
                });
                break;
            }
        }

        if raw.contains(' ') || raw.to_ascii_lowercase().contains("%20") {
            findings.push(Finding {
                filter_key: FilterKey::UrlContainsSpace,
            });
        }

        let path_lower = path.to_ascii_lowercase();
        if is_internal_search(&path_lower, query) {
            findings.push(Finding {
                filter_key: FilterKey::UrlInternalSearchUrl,
            });
        }

        if has_ga_params(query) {
            findings.push(Finding {
                filter_key: FilterKey::UrlGaTrackingParameters,
            });
        }

        findings
    }
}

fn is_internal_search(path_lower: &str, query: &str) -> bool {
    if path_lower.contains("/search") || path_lower.ends_with("/find") {
        return true;
    }
    let q_lower = query.to_ascii_lowercase();
    let q_keys = ["q=", "s=", "search=", "query=", "keyword=", "keywords="];
    q_keys.iter().any(|k| {
        q_lower == k.trim_end_matches('=')
            || q_lower.starts_with(k)
            || q_lower.contains(&format!("&{k}"))
    })
}

fn has_ga_params(query: &str) -> bool {
    let q = query.to_ascii_lowercase();
    if q.contains("gclid=") || q.contains("fbclid=") || q.contains("mc_cid=") {
        return true;
    }
    // utm_source, utm_medium, utm_campaign, etc.
    q.split('&').any(|p| p.starts_with("utm_"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sf_core::config::CrawlConfig;
    use sf_core::crawl::{ContentType, CrawlUrl};
    use sf_core::id::{CrawlId, CrawlUrlId};

    fn crawl(u: &str) -> CrawlUrl {
        CrawlUrl {
            id: CrawlUrlId::new(),
            crawl_id: CrawlId::new(),
            url: u.to_string(),
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

    fn eval(u: &str) -> Vec<FilterKey> {
        let cfg = CrawlConfig::default();
        let ctx = EvalContext {
            config: &cfg,
            html: None,
            parsed: None,
        };
        UrlEvaluator
            .evaluate(&crawl(u), &ctx)
            .into_iter()
            .map(|f| f.filter_key)
            .collect()
    }

    #[test]
    fn every_url_emits_all() {
        let keys = eval("https://example.com/");
        assert!(keys.contains(&FilterKey::UrlAll));
    }

    #[test]
    fn ascii_clean_url_emits_only_all() {
        let keys = eval("https://example.com/");
        assert_eq!(keys, vec![FilterKey::UrlAll]);
    }

    #[test]
    fn underscore_flagged() {
        let keys = eval("https://example.com/foo_bar");
        assert!(keys.contains(&FilterKey::UrlUnderscores));
    }

    #[test]
    fn uppercase_flagged() {
        let keys = eval("https://example.com/AboutUs");
        assert!(keys.contains(&FilterKey::UrlUppercase));
    }

    #[test]
    fn non_ascii_flagged() {
        let keys = eval("https://example.com/café");
        assert!(keys.contains(&FilterKey::UrlNonAsciiCharacters));
    }

    #[test]
    fn parameters_flagged() {
        let keys = eval("https://example.com/p?x=1");
        assert!(keys.contains(&FilterKey::UrlParameters));
    }

    #[test]
    fn over_max_length_flagged() {
        let long = format!("https://example.com/{}", "a".repeat(200));
        let keys = eval(&long);
        assert!(keys.contains(&FilterKey::UrlOverXCharacters));
    }

    #[test]
    fn multiple_slashes_flagged() {
        let keys = eval("https://example.com/foo//bar");
        assert!(keys.contains(&FilterKey::UrlMultipleSlashes));
    }

    #[test]
    fn repetitive_path_flagged() {
        let keys = eval("https://example.com/foo/foo/bar");
        assert!(keys.contains(&FilterKey::UrlRepetitivePath));
    }

    #[test]
    fn space_flagged() {
        let keys = eval("https://example.com/foo%20bar");
        assert!(keys.contains(&FilterKey::UrlContainsSpace));
    }

    #[test]
    fn internal_search_flagged_by_query() {
        let keys = eval("https://example.com/page?q=hello");
        assert!(keys.contains(&FilterKey::UrlInternalSearchUrl));
    }

    #[test]
    fn internal_search_flagged_by_path() {
        let keys = eval("https://example.com/search/results?page=2");
        assert!(keys.contains(&FilterKey::UrlInternalSearchUrl));
    }

    #[test]
    fn ga_params_flagged() {
        let keys = eval("https://example.com/?utm_source=newsletter&utm_medium=email");
        assert!(keys.contains(&FilterKey::UrlGaTrackingParameters));
    }

    #[test]
    fn gclid_flagged() {
        let keys = eval("https://example.com/?gclid=abc123");
        assert!(keys.contains(&FilterKey::UrlGaTrackingParameters));
    }
}
