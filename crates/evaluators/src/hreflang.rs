//! Hreflang tab evaluator.
//!
//! Parses `<link rel="alternate" hreflang="...">` tags. Implements the
//! subset of SF's Hreflang filters that can be decided from a single
//! page's HTML:
//!
//! - `HreflangAll` — every HTML page.
//! - `HreflangContainsHreflang` — ≥1 hreflang tag present.
//! - `HreflangMissing` — zero hreflang tags (HTML pages only).
//! - `HreflangMultipleEntries` — the same language code appears twice.
//! - `HreflangMissingXdefault` — hreflang set without an `x-default`.
//! - `HreflangMissingSelfReference` — none of the tags point at the
//!   current page's URL.
//! - `HreflangIncorrectLanguageCodes` — code fails BCP-47-ish validation
//!   (not `x-default`, not `xx` or `xx-YY` with plausible shape).
//! - `HreflangOutsideHead` — a hreflang link lives outside `<head>`.
//! - `HreflangNotUsingCanonical` — the page has hreflang tags AND a
//!   canonical that points to a different URL (Google requires hreflang
//!   to be placed on the canonical version).
//!
//! Deferred (need cross-URL link-graph): `HreflangNon200HreflangUrls`,
//! `HreflangUnlinkedHreflangUrls`, `HreflangMissingReturnLinks`,
//! `HreflangInconsistentLanguageReturnLinks`,
//! `HreflangNonCanonicalReturnLinks`, `HreflangNoIndexReturnLinks`.

use scraper::Selector;
use sf_core::crawl::{ContentType, CrawlUrl};
use sf_core::filter_key::FilterKey;
use sf_core::tab::TabKey;
use url::Url;

use crate::{EvalContext, Evaluator, Finding};

pub struct HreflangEvaluator;

impl Evaluator for HreflangEvaluator {
    fn tab(&self) -> TabKey {
        TabKey::Hreflang
    }

    fn evaluate(&self, url: &CrawlUrl, ctx: &EvalContext) -> Vec<Finding> {
        if url.content_type != ContentType::Html {
            return vec![];
        }

        let mut findings = vec![Finding {
            filter_key: FilterKey::HreflangAll,
        }];

        let parsed = match ctx.parsed {
            Some(p) => p,
            None => return findings,
        };

        let link_sel = match Selector::parse(r#"link[rel="alternate"][hreflang]"#) {
            Ok(s) => s,
            Err(_) => return findings,
        };

        let tags: Vec<_> = parsed.select(&link_sel).collect();

        if tags.is_empty() {
            findings.push(Finding {
                filter_key: FilterKey::HreflangMissing,
            });
            return findings;
        }

        findings.push(Finding {
            filter_key: FilterKey::HreflangContainsHreflang,
        });

        let base = Url::parse(&url.url).ok();
        let self_norm = base.as_ref().map(normalize);

        let mut codes: Vec<String> = Vec::new();
        let mut resolved_targets: Vec<String> = Vec::new();
        let mut saw_invalid_code = false;
        let mut saw_outside_head = false;
        let mut saw_xdefault = false;

        for el in &tags {
            let code = el
                .value()
                .attr("hreflang")
                .map(|s| s.trim().to_ascii_lowercase())
                .unwrap_or_default();
            if code == "x-default" {
                saw_xdefault = true;
            } else if !is_valid_bcp47_ish(&code) {
                saw_invalid_code = true;
            }
            codes.push(code);

            let href = el.value().attr("href").map(|s| s.trim()).unwrap_or("");
            if let Some(b) = &base
                && let Ok(u) = b.join(href)
            {
                resolved_targets.push(normalize(&u));
            }

            if !ancestor_is_head(el) {
                saw_outside_head = true;
            }
        }

        if saw_outside_head {
            findings.push(Finding {
                filter_key: FilterKey::HreflangOutsideHead,
            });
        }

        if saw_invalid_code {
            findings.push(Finding {
                filter_key: FilterKey::HreflangIncorrectLanguageCodes,
            });
        }

        if !saw_xdefault {
            findings.push(Finding {
                filter_key: FilterKey::HreflangMissingXdefault,
            });
        }

        // Duplicate language codes — same code listed more than once.
        let mut sorted = codes.clone();
        sorted.sort();
        let mut has_dup = false;
        for pair in sorted.windows(2) {
            if pair[0] == pair[1] {
                has_dup = true;
                break;
            }
        }
        if has_dup {
            findings.push(Finding {
                filter_key: FilterKey::HreflangMultipleEntries,
            });
        }

        if let Some(self_n) = &self_norm
            && !resolved_targets.iter().any(|t| t == self_n)
        {
            findings.push(Finding {
                filter_key: FilterKey::HreflangMissingSelfReference,
            });
        }

        // NotUsingCanonical: hreflang present AND canonical tag points
        // somewhere other than this URL.
        if let Some(self_n) = &self_norm {
            let canon_sel = Selector::parse(r#"link[rel="canonical"]"#).ok();
            let canonical_targets: Vec<String> = canon_sel
                .iter()
                .flat_map(|s| parsed.select(s))
                .filter_map(|el| el.value().attr("href"))
                .filter_map(|h| base.as_ref().and_then(|b| b.join(h).ok()))
                .map(|u| normalize(&u))
                .collect();
            let any_canon_points_elsewhere =
                !canonical_targets.is_empty() && canonical_targets.iter().all(|t| t != self_n);
            if any_canon_points_elsewhere {
                findings.push(Finding {
                    filter_key: FilterKey::HreflangNotUsingCanonical,
                });
            }
        }

        findings
    }
}

fn ancestor_is_head(el: &scraper::ElementRef) -> bool {
    let mut node = el.parent();
    while let Some(n) = node {
        if let Some(e) = scraper::ElementRef::wrap(n) {
            if e.value().name() == "head" {
                return true;
            }
            node = e.parent();
        } else {
            return false;
        }
    }
    false
}

/// Permissive BCP-47 check: one of
///   - 2-3 lowercase letters  (e.g. `en`, `zho`)
///   - 2-3 letters + `-` + 2-letter region OR 4-letter script
///     (e.g. `en-us`, `zh-hant`)
fn is_valid_bcp47_ish(code: &str) -> bool {
    let parts: Vec<&str> = code.split('-').collect();
    if parts.is_empty() {
        return false;
    }
    let lang = parts[0];
    if lang.len() < 2 || lang.len() > 3 || !lang.chars().all(|c| c.is_ascii_lowercase()) {
        return false;
    }
    if parts.len() == 1 {
        return true;
    }
    if parts.len() != 2 {
        return false;
    }
    let tail = parts[1];
    let len_ok = matches!(tail.len(), 2 | 4);
    len_ok && tail.chars().all(|c| c.is_ascii_alphabetic())
}

fn normalize(u: &Url) -> String {
    let mut s = u.as_str().to_ascii_lowercase();
    if s.ends_with('/') && s.matches('/').count() > 3 {
        s.pop();
    }
    s
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
        HreflangEvaluator
            .evaluate(u, &ctx)
            .into_iter()
            .map(|f| f.filter_key)
            .collect()
    }

    #[test]
    fn every_html_page_emits_all() {
        let keys = eval(&page("https://example.com/"), "<html><head></head></html>");
        assert!(keys.contains(&FilterKey::HreflangAll));
    }

    #[test]
    fn non_html_emits_nothing() {
        let mut u = page("https://example.com/x.png");
        u.content_type = ContentType::Image;
        let keys = eval(&u, "");
        assert!(keys.is_empty());
    }

    #[test]
    fn missing_when_no_tags() {
        let keys = eval(&page("https://example.com/"), "<html><head></head></html>");
        assert!(keys.contains(&FilterKey::HreflangMissing));
        assert!(!keys.contains(&FilterKey::HreflangContainsHreflang));
    }

    #[test]
    fn well_formed_set_with_xdefault_and_self() {
        let html = r#"<html><head>
            <link rel="alternate" hreflang="x-default" href="https://example.com/en/">
            <link rel="alternate" hreflang="en" href="https://example.com/en/">
            <link rel="alternate" hreflang="fr" href="https://example.com/fr/">
        </head></html>"#;
        let keys = eval(&page("https://example.com/en/"), html);
        assert!(keys.contains(&FilterKey::HreflangContainsHreflang));
        assert!(!keys.contains(&FilterKey::HreflangMissingXdefault));
        assert!(!keys.contains(&FilterKey::HreflangMissingSelfReference));
        assert!(!keys.contains(&FilterKey::HreflangIncorrectLanguageCodes));
        assert!(!keys.contains(&FilterKey::HreflangMultipleEntries));
    }

    #[test]
    fn missing_xdefault_flagged() {
        let html = r#"<html><head>
            <link rel="alternate" hreflang="en" href="https://example.com/en/">
            <link rel="alternate" hreflang="fr" href="https://example.com/fr/">
        </head></html>"#;
        let keys = eval(&page("https://example.com/en/"), html);
        assert!(keys.contains(&FilterKey::HreflangMissingXdefault));
    }

    #[test]
    fn missing_self_reference_flagged() {
        let html = r#"<html><head>
            <link rel="alternate" hreflang="x-default" href="https://example.com/en/">
            <link rel="alternate" hreflang="fr" href="https://example.com/fr/">
        </head></html>"#;
        let keys = eval(&page("https://example.com/de/"), html);
        assert!(keys.contains(&FilterKey::HreflangMissingSelfReference));
    }

    #[test]
    fn multiple_entries_flagged() {
        let html = r#"<html><head>
            <link rel="alternate" hreflang="en" href="https://example.com/en-a/">
            <link rel="alternate" hreflang="en" href="https://example.com/en-b/">
        </head></html>"#;
        let keys = eval(&page("https://example.com/en-a/"), html);
        assert!(keys.contains(&FilterKey::HreflangMultipleEntries));
    }

    #[test]
    fn invalid_code_flagged() {
        let html = r#"<html><head>
            <link rel="alternate" hreflang="english" href="https://example.com/en/">
        </head></html>"#;
        let keys = eval(&page("https://example.com/en/"), html);
        assert!(keys.contains(&FilterKey::HreflangIncorrectLanguageCodes));
    }

    #[test]
    fn outside_head_flagged() {
        let html = r#"<html><head></head><body>
            <link rel="alternate" hreflang="en" href="https://example.com/en/">
        </body></html>"#;
        let keys = eval(&page("https://example.com/en/"), html);
        assert!(keys.contains(&FilterKey::HreflangOutsideHead));
    }

    #[test]
    fn not_using_canonical_flagged() {
        // hreflang set but canonical points elsewhere.
        let html = r#"<html><head>
            <link rel="canonical" href="https://example.com/canonical-page/">
            <link rel="alternate" hreflang="x-default" href="https://example.com/en/">
            <link rel="alternate" hreflang="en" href="https://example.com/en/">
        </head></html>"#;
        let keys = eval(&page("https://example.com/en/"), html);
        assert!(keys.contains(&FilterKey::HreflangNotUsingCanonical));
    }

    #[test]
    fn bcp47_accepts_common_tags() {
        assert!(is_valid_bcp47_ish("en"));
        assert!(is_valid_bcp47_ish("en-us"));
        assert!(is_valid_bcp47_ish("zh-hant"));
        assert!(!is_valid_bcp47_ish("english"));
        assert!(!is_valid_bcp47_ish("e"));
        assert!(!is_valid_bcp47_ish("en-u"));
    }
}
