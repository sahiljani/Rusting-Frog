//! AMP tab evaluator.
//!
//! Only runs on pages that self-identify as AMP via `<html amp>` or
//! `<html ⚡>`. For those pages we check the mandatory AMP structural
//! requirements: canonical, charset, viewport, the v0 runtime script,
//! and the AMP boilerplate.
//!
//! Implemented:
//! - `AmpAll` — every AMP page.
//! - `AmpNon200` — AMP page whose HTTP status is not 200.
//! - `AmpIndexable` / `AmpNonIndexable` — derived from `meta_robots`
//!   (`noindex` ⇒ non-indexable).
//! - `AmpMissingDoctype` — page body doesn't start with `<!doctype html>`.
//! - `AmpMissingHead` / `AmpMissingBody` — structural check.
//! - `AmpMissingCanonical` — no `<link rel="canonical">`.
//! - `AmpMissingCharset` — no `<meta charset>`.
//! - `AmpMissingViewport` — no `<meta name="viewport">`.
//! - `AmpMissingAmpScript` — no `<script async src="https://cdn.ampproject.org/v0.js">`.
//! - `AmpMissingAmpBoilerplate` — no `<style amp-boilerplate>` + noscript
//!   fallback.
//! - `AmpContainsDisallowedHtml` — page contains tags that AMP forbids
//!   at the top level (`<iframe>` outside amp-iframe, `<object>`,
//!   `<embed>`, `<form>` outside amp-form, inline `<script>` without
//!   `amp-` type, etc. — we spot-check for `<iframe>`, `<embed>`,
//!   `<object>` and a plain non-AMP `<script>`).
//!
//! Deferred: `MissingNonAmpReturnLink` (cross-URL),
//! `AmpMissingCanonicalToNonAmp` (cross-URL),
//! `AmpNonIndexableCanonical` (post-crawl),
//! `AmpMissingAmpTag` (fires when a page is expected to be AMP but
//! lacks the tag — needs hint from the non-AMP page's `rel=amphtml`),
//! `AmpOtherValidationErrors` (would need the full AMP validator).

use scraper::Selector;
use sf_core::crawl::{ContentType, CrawlUrl};
use sf_core::filter_key::FilterKey;
use sf_core::tab::TabKey;

use crate::{EvalContext, Evaluator, Finding};

pub struct AmpEvaluator;

impl Evaluator for AmpEvaluator {
    fn tab(&self) -> TabKey {
        TabKey::Amp
    }

    fn evaluate(&self, url: &CrawlUrl, ctx: &EvalContext) -> Vec<Finding> {
        if url.content_type != ContentType::Html {
            return vec![];
        }

        let parsed = match ctx.parsed {
            Some(p) => p,
            None => return vec![],
        };

        let html_sel = match Selector::parse("html") {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        let html_el = match parsed.select(&html_sel).next() {
            Some(h) => h,
            None => return vec![],
        };

        let is_amp = html_el.value().attr("amp").is_some()
            || html_el.value().attr("\u{26A1}").is_some()
            || html_el
                .value()
                .attrs()
                .any(|(k, _)| k == "\u{26A1}" || k == "amp");

        if !is_amp {
            return vec![];
        }

        let mut findings = vec![Finding {
            filter_key: FilterKey::AmpAll,
        }];

        if let Some(sc) = url.status_code {
            if sc != 200 {
                findings.push(Finding {
                    filter_key: FilterKey::AmpNon200,
                });
            }
        }

        let is_noindex = url
            .meta_robots
            .as_deref()
            .map(|r| r.to_ascii_lowercase().contains("noindex"))
            .unwrap_or(false);
        findings.push(Finding {
            filter_key: if is_noindex {
                FilterKey::AmpNonIndexable
            } else {
                FilterKey::AmpIndexable
            },
        });

        if let Some(raw) = ctx.html {
            if !raw
                .trim_start()
                .to_ascii_lowercase()
                .starts_with("<!doctype html")
            {
                findings.push(Finding {
                    filter_key: FilterKey::AmpMissingDoctype,
                });
            }
        }

        let first_or_empty = |sel: &str| {
            Selector::parse(sel)
                .ok()
                .and_then(|s| parsed.select(&s).next())
        };

        if first_or_empty("head").is_none() {
            findings.push(Finding {
                filter_key: FilterKey::AmpMissingHead,
            });
        }
        if first_or_empty("body").is_none() {
            findings.push(Finding {
                filter_key: FilterKey::AmpMissingBody,
            });
        }
        if first_or_empty(r#"link[rel="canonical"]"#).is_none() {
            findings.push(Finding {
                filter_key: FilterKey::AmpMissingCanonical,
            });
        }
        if first_or_empty("meta[charset]").is_none() {
            findings.push(Finding {
                filter_key: FilterKey::AmpMissingCharset,
            });
        }
        if first_or_empty(r#"meta[name="viewport"]"#).is_none() {
            findings.push(Finding {
                filter_key: FilterKey::AmpMissingViewport,
            });
        }

        let amp_script_present = Selector::parse("script[async]")
            .ok()
            .map(|sel| {
                parsed.select(&sel).any(|el| {
                    el.value()
                        .attr("src")
                        .map(|s| s.contains("cdn.ampproject.org") && s.contains("v0.js"))
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);
        if !amp_script_present {
            findings.push(Finding {
                filter_key: FilterKey::AmpMissingAmpScript,
            });
        }

        let boilerplate_present = Selector::parse("style[amp-boilerplate]")
            .ok()
            .map(|s| parsed.select(&s).next().is_some())
            .unwrap_or(false);
        if !boilerplate_present {
            findings.push(Finding {
                filter_key: FilterKey::AmpMissingAmpBoilerplate,
            });
        }

        let disallowed_tags = ["iframe", "object", "embed"];
        let mut has_disallowed = false;
        for tag in &disallowed_tags {
            if Selector::parse(tag)
                .ok()
                .and_then(|s| parsed.select(&s).next())
                .is_some()
            {
                has_disallowed = true;
                break;
            }
        }
        // Inline <script> without any AMP attribute is disallowed
        // (except for JSON-LD). Spot-check by finding non-async scripts
        // with no custom-element attribute and no type="application/ld+json".
        if !has_disallowed {
            if let Some(sel) = Selector::parse("script").ok() {
                for el in parsed.select(&sel) {
                    let v = el.value();
                    let is_amp_runtime = v
                        .attr("src")
                        .map(|s| s.contains("cdn.ampproject.org"))
                        .unwrap_or(false);
                    let is_ld_json = v
                        .attr("type")
                        .map(|t| t.eq_ignore_ascii_case("application/ld+json"))
                        .unwrap_or(false);
                    let is_custom_element =
                        v.attr("custom-element").is_some() || v.attr("custom-template").is_some();
                    if !is_amp_runtime && !is_ld_json && !is_custom_element {
                        has_disallowed = true;
                        break;
                    }
                }
            }
        }
        if has_disallowed {
            findings.push(Finding {
                filter_key: FilterKey::AmpContainsDisallowedHtml,
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
            url: "https://example.com/amp/".to_string(),
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
        AmpEvaluator
            .evaluate(u, &ctx)
            .into_iter()
            .map(|f| f.filter_key)
            .collect()
    }

    #[test]
    fn non_amp_page_emits_nothing() {
        let keys = eval(
            &page(),
            "<!doctype html><html><head></head><body></body></html>",
        );
        assert!(keys.is_empty());
    }

    #[test]
    fn amp_page_emits_amp_all() {
        let keys = eval(
            &page(),
            r#"<!doctype html><html amp><head></head><body></body></html>"#,
        );
        assert!(keys.contains(&FilterKey::AmpAll));
    }

    #[test]
    fn amp_indexable_by_default() {
        let keys = eval(
            &page(),
            r#"<!doctype html><html amp><head></head><body></body></html>"#,
        );
        assert!(keys.contains(&FilterKey::AmpIndexable));
        assert!(!keys.contains(&FilterKey::AmpNonIndexable));
    }

    #[test]
    fn amp_noindex_is_non_indexable() {
        let mut u = page();
        u.meta_robots = Some("noindex,follow".to_string());
        let keys = eval(
            &u,
            r#"<!doctype html><html amp><head></head><body></body></html>"#,
        );
        assert!(keys.contains(&FilterKey::AmpNonIndexable));
    }

    #[test]
    fn amp_missing_canonical_and_charset_flagged() {
        let keys = eval(
            &page(),
            r#"<!doctype html><html amp><head></head><body></body></html>"#,
        );
        assert!(keys.contains(&FilterKey::AmpMissingCanonical));
        assert!(keys.contains(&FilterKey::AmpMissingCharset));
        assert!(keys.contains(&FilterKey::AmpMissingViewport));
        assert!(keys.contains(&FilterKey::AmpMissingAmpScript));
        assert!(keys.contains(&FilterKey::AmpMissingAmpBoilerplate));
    }

    #[test]
    fn amp_with_all_essentials_passes_structural_checks() {
        let html = r#"<!doctype html><html amp>
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width,minimum-scale=1">
  <link rel="canonical" href="https://example.com/non-amp">
  <script async src="https://cdn.ampproject.org/v0.js"></script>
  <style amp-boilerplate>body{}</style>
</head>
<body></body></html>"#;
        let keys = eval(&page(), html);
        assert!(keys.contains(&FilterKey::AmpAll));
        assert!(!keys.contains(&FilterKey::AmpMissingCanonical));
        assert!(!keys.contains(&FilterKey::AmpMissingCharset));
        assert!(!keys.contains(&FilterKey::AmpMissingViewport));
        assert!(!keys.contains(&FilterKey::AmpMissingAmpScript));
        assert!(!keys.contains(&FilterKey::AmpMissingAmpBoilerplate));
    }

    #[test]
    fn amp_disallowed_iframe_flagged() {
        let html = r#"<!doctype html><html amp><head>
            <meta charset="utf-8">
            <script async src="https://cdn.ampproject.org/v0.js"></script>
            <style amp-boilerplate>body{}</style>
        </head><body><iframe src="x"></iframe></body></html>"#;
        let keys = eval(&page(), html);
        assert!(keys.contains(&FilterKey::AmpContainsDisallowedHtml));
    }

    #[test]
    fn non_200_amp_flagged() {
        let mut u = page();
        u.status_code = Some(404);
        let keys = eval(
            &u,
            r#"<!doctype html><html amp><head></head><body></body></html>"#,
        );
        assert!(keys.contains(&FilterKey::AmpNon200));
    }
}
