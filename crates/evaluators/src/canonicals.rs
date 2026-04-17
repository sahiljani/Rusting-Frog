//! Canonicals tab evaluator.
//!
//! Parses `<link rel="canonical">` tags on the page and emits per-URL
//! findings about canonical hygiene. Every HTML page gets `CanonicalsAll`;
//! additional findings depend on whether / where / how the tag appears:
//!
//! - `CanonicalsContainsCanonical` — page has ≥1 canonical.
//! - `CanonicalsSelfReferencing` — canonical target == own URL (ignoring
//!   trailing slash + scheme case).
//! - `CanonicalsCanonicalised` — canonical target is some OTHER URL
//!   (implies this page is canonicalised to a different document).
//! - `CanonicalsMissing` — HTML page with zero canonical tags.
//! - `CanonicalsMultiple` — more than one canonical tag on the page.
//! - `CanonicalsMultipleConflicting` — multiple canonicals that disagree.
//! - `CanonicalsCanonicalIsRelative` — href isn't absolute (missing scheme).
//! - `CanonicalsInvalidAttributes` — tag has no `href`, or has other broken
//!   attrs (e.g. `href` is empty).
//! - `CanonicalsContainsFragmentUrl` — href contains `#fragment`.
//! - `CanonicalsOutsideHead` — canonical tag lives outside `<head>`.
//!
//! Cross-URL filters (`CanonicalsNonIndexableCanonical`,
//! `CanonicalsUnlinked`) are deferred to post-crawl analysis — they need
//! the full URL graph and final index-status table.

use scraper::Selector;
use sf_core::crawl::{ContentType, CrawlUrl};
use sf_core::filter_key::FilterKey;
use sf_core::tab::TabKey;
use url::Url;

use crate::{EvalContext, Evaluator, Finding};

pub struct CanonicalsEvaluator;

impl Evaluator for CanonicalsEvaluator {
    fn tab(&self) -> TabKey {
        TabKey::Canonicals
    }

    fn evaluate(&self, url: &CrawlUrl, ctx: &EvalContext) -> Vec<Finding> {
        if url.content_type != ContentType::Html {
            return vec![];
        }

        let mut findings = vec![Finding {
            filter_key: FilterKey::CanonicalsAll,
        }];

        let parsed = match ctx.parsed {
            Some(p) => p,
            None => return findings,
        };

        let link_sel = match Selector::parse(r#"link[rel="canonical"]"#) {
            Ok(s) => s,
            Err(_) => return findings,
        };

        let canonicals: Vec<_> = parsed.select(&link_sel).collect();

        if canonicals.is_empty() {
            findings.push(Finding {
                filter_key: FilterKey::CanonicalsMissing,
            });
            return findings;
        }

        findings.push(Finding {
            filter_key: FilterKey::CanonicalsContainsCanonical,
        });

        if canonicals.len() > 1 {
            findings.push(Finding {
                filter_key: FilterKey::CanonicalsMultiple,
            });
        }

        let mut targets: Vec<String> = Vec::new();
        let mut saw_invalid = false;
        let mut saw_relative = false;
        let mut saw_fragment = false;
        let mut saw_outside_head = false;

        for el in &canonicals {
            // Outside-<head>: ascend ancestors and check whether a <head>
            // appears before the root.
            let mut in_head = false;
            let mut node = el.parent();
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
                saw_outside_head = true;
            }

            let href = el.value().attr("href").map(|s| s.trim()).unwrap_or("");
            if href.is_empty() {
                saw_invalid = true;
                continue;
            }
            if href.contains('#') {
                saw_fragment = true;
            }
            let is_absolute =
                href.starts_with("http://") || href.starts_with("https://") || href.starts_with("//");
            if !is_absolute {
                saw_relative = true;
            }
            targets.push(href.to_string());
        }

        if saw_invalid {
            findings.push(Finding {
                filter_key: FilterKey::CanonicalsInvalidAttributes,
            });
        }
        if saw_relative {
            findings.push(Finding {
                filter_key: FilterKey::CanonicalsCanonicalIsRelative,
            });
        }
        if saw_fragment {
            findings.push(Finding {
                filter_key: FilterKey::CanonicalsContainsFragmentUrl,
            });
        }
        if saw_outside_head {
            findings.push(Finding {
                filter_key: FilterKey::CanonicalsOutsideHead,
            });
        }

        // Resolve + normalize each target against this page's URL, then
        // compare pairwise for conflicts and with self for self-reference.
        let base = Url::parse(&url.url).ok();
        let resolved: Vec<String> = targets
            .iter()
            .filter_map(|t| {
                base.as_ref()
                    .and_then(|b| b.join(t).ok())
                    .map(|u| normalize(&u))
            })
            .collect();

        let distinct: std::collections::HashSet<&String> = resolved.iter().collect();
        if distinct.len() > 1 {
            findings.push(Finding {
                filter_key: FilterKey::CanonicalsMultipleConflicting,
            });
        }

        if let Some(b) = &base {
            let self_norm = normalize(b);
            let any_self = resolved.iter().any(|r| r == &self_norm);
            let any_other = resolved.iter().any(|r| r != &self_norm);
            if any_self {
                findings.push(Finding {
                    filter_key: FilterKey::CanonicalsSelfReferencing,
                });
            }
            if any_other {
                findings.push(Finding {
                    filter_key: FilterKey::CanonicalsCanonicalised,
                });
            }
        }

        findings
    }
}

fn normalize(u: &Url) -> String {
    let mut s = u.as_str().to_ascii_lowercase();
    // Strip trailing slash for comparison purposes — "/" and "" are the
    // same document in SF's view.
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

    fn eval(url: &CrawlUrl, html: &str) -> Vec<FilterKey> {
        let cfg = CrawlConfig::default();
        let parsed = Html::parse_document(html);
        let ctx = EvalContext {
            config: &cfg,
            html: Some(html),
            parsed: Some(&parsed),
        };
        CanonicalsEvaluator
            .evaluate(url, &ctx)
            .into_iter()
            .map(|f| f.filter_key)
            .collect()
    }

    #[test]
    fn every_html_page_emits_all() {
        let keys = eval(
            &page("https://example.com/"),
            "<html><head></head><body></body></html>",
        );
        assert!(keys.contains(&FilterKey::CanonicalsAll));
    }

    #[test]
    fn non_html_emits_nothing() {
        let mut u = page("https://example.com/img.png");
        u.content_type = ContentType::Image;
        let keys = eval(&u, "");
        assert!(keys.is_empty());
    }

    #[test]
    fn page_without_canonical_is_missing() {
        let keys = eval(
            &page("https://example.com/"),
            "<html><head></head><body></body></html>",
        );
        assert!(keys.contains(&FilterKey::CanonicalsMissing));
        assert!(!keys.contains(&FilterKey::CanonicalsContainsCanonical));
    }

    #[test]
    fn self_referencing_canonical() {
        let html = r#"<html><head><link rel="canonical" href="https://example.com/"></head></html>"#;
        let keys = eval(&page("https://example.com/"), html);
        assert!(keys.contains(&FilterKey::CanonicalsSelfReferencing));
        assert!(!keys.contains(&FilterKey::CanonicalsCanonicalised));
    }

    #[test]
    fn canonicalised_to_other_url() {
        let html =
            r#"<html><head><link rel="canonical" href="https://example.com/home"></head></html>"#;
        let keys = eval(&page("https://example.com/home?utm=x"), html);
        assert!(keys.contains(&FilterKey::CanonicalsCanonicalised));
    }

    #[test]
    fn relative_canonical_flagged() {
        let html = r#"<html><head><link rel="canonical" href="/home"></head></html>"#;
        let keys = eval(&page("https://example.com/page"), html);
        assert!(keys.contains(&FilterKey::CanonicalsCanonicalIsRelative));
    }

    #[test]
    fn fragment_canonical_flagged() {
        let html = r#"<html><head><link rel="canonical" href="https://example.com/#section"></head></html>"#;
        let keys = eval(&page("https://example.com/"), html);
        assert!(keys.contains(&FilterKey::CanonicalsContainsFragmentUrl));
    }

    #[test]
    fn canonical_without_href_flagged_invalid() {
        let html = r#"<html><head><link rel="canonical"></head></html>"#;
        let keys = eval(&page("https://example.com/"), html);
        assert!(keys.contains(&FilterKey::CanonicalsInvalidAttributes));
    }

    #[test]
    fn multiple_canonicals_flagged() {
        let html = r#"<html><head>
            <link rel="canonical" href="https://example.com/a">
            <link rel="canonical" href="https://example.com/a">
        </head></html>"#;
        let keys = eval(&page("https://example.com/a"), html);
        assert!(keys.contains(&FilterKey::CanonicalsMultiple));
        assert!(!keys.contains(&FilterKey::CanonicalsMultipleConflicting));
    }

    #[test]
    fn multiple_conflicting_canonicals_flagged() {
        let html = r#"<html><head>
            <link rel="canonical" href="https://example.com/a">
            <link rel="canonical" href="https://example.com/b">
        </head></html>"#;
        let keys = eval(&page("https://example.com/"), html);
        assert!(keys.contains(&FilterKey::CanonicalsMultipleConflicting));
    }

    #[test]
    fn canonical_outside_head_flagged() {
        let html = r#"<html><head></head><body>
            <link rel="canonical" href="https://example.com/a">
        </body></html>"#;
        let keys = eval(&page("https://example.com/a"), html);
        assert!(keys.contains(&FilterKey::CanonicalsOutsideHead));
    }
}
