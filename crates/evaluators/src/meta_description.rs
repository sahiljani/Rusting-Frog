//! Meta Description tab evaluator.
//!
//! Implemented filters (per-URL): MetaDescritonAll, MetaDescritonMissing,
//! MetaDescritonMultiple, MetaDescritonOverXCharacters,
//! MetaDescritonBelowXCharacters, MetaDescriptionOutsideHead.
//!
//! Skipped in this pass:
//! - MetaDescritonDuplicate: cross-URL, belongs to post-crawl analysis.
//! - MetaDescritonOverXPixels / MetaDescritonBelowXPixels: require
//!   pixel-width measurement (not yet captured on CrawlUrl).
//!
//! Note: "MetaDescriton" (missing 'i') is the literal variant name from
//! Screaming Frog's `SeoElementFilterKey` Java source; we preserve the
//! typo for parity.

use scraper::Selector;
use sf_core::crawl::{ContentType, CrawlUrl};
use sf_core::filter_key::FilterKey;
use sf_core::tab::TabKey;

use crate::{EvalContext, Evaluator, Finding};

pub struct MetaDescriptionEvaluator;

impl Evaluator for MetaDescriptionEvaluator {
    fn tab(&self) -> TabKey {
        TabKey::MetaDescription
    }

    fn evaluate(&self, url: &CrawlUrl, ctx: &EvalContext) -> Vec<Finding> {
        if url.content_type != ContentType::Html {
            return vec![];
        }

        let mut findings = vec![Finding {
            filter_key: FilterKey::MetaDescriptonAll,
        }];

        let parsed = match ctx.parsed {
            Some(p) => p,
            None => return findings,
        };

        let descriptions = extract_meta_descriptions(parsed);

        if descriptions.is_empty() {
            findings.push(Finding {
                filter_key: FilterKey::MetaDescriptonMissing,
            });
            return findings;
        }

        if descriptions.len() > 1 {
            findings.push(Finding {
                filter_key: FilterKey::MetaDescriptonMultiple,
            });
        }

        let desc = &descriptions[0];
        let len = desc.chars().count() as u32;
        let cfg = &ctx.config.meta_description;

        if len > cfg.max_length {
            findings.push(Finding {
                filter_key: FilterKey::MetaDescriptonOverXCharacters,
            });
        }
        if len < cfg.min_length {
            findings.push(Finding {
                filter_key: FilterKey::MetaDescriptonBelowXCharacters,
            });
        }

        if has_meta_description_outside_head(parsed) {
            findings.push(Finding {
                filter_key: FilterKey::MetaDescriptionOutsideHead,
            });
        }

        findings
    }
}

fn extract_meta_descriptions(html: &scraper::Html) -> Vec<String> {
    // SF treats the name as case-insensitive; emulate with an attribute-any
    // selector and filter by lowercased name.
    let sel = Selector::parse("meta[name][content]").expect("valid selector");
    html.select(&sel)
        .filter(|el| {
            el.value()
                .attr("name")
                .map(|n| n.eq_ignore_ascii_case("description"))
                .unwrap_or(false)
        })
        .filter_map(|el| el.value().attr("content"))
        .map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty())
        .collect()
}

fn has_meta_description_outside_head(html: &scraper::Html) -> bool {
    let all = Selector::parse("meta[name]").expect("valid selector");
    let in_head = Selector::parse("head meta[name]").expect("valid selector");

    let total = html.select(&all).filter(|el| is_description(el)).count();
    let in_head_count = html
        .select(&in_head)
        .filter(|el| is_description(el))
        .count();
    total > in_head_count
}

fn is_description(el: &scraper::ElementRef<'_>) -> bool {
    el.value()
        .attr("name")
        .map(|n| n.eq_ignore_ascii_case("description"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use scraper::Html;
    use sf_core::config::CrawlConfig;
    use sf_core::crawl::{ContentType, CrawlUrl};
    use sf_core::id::{CrawlId, CrawlUrlId};

    fn url_fixture() -> CrawlUrl {
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

    fn findings_for(html_str: &str) -> Vec<FilterKey> {
        let cfg = CrawlConfig::default();
        let parsed = Html::parse_document(html_str);
        let ctx = EvalContext {
            config: &cfg,
            html: Some(html_str),
            parsed: Some(&parsed),
        };
        MetaDescriptionEvaluator
            .evaluate(&url_fixture(), &ctx)
            .into_iter()
            .map(|f| f.filter_key)
            .collect()
    }

    #[test]
    fn missing_description_flags_missing() {
        let keys = findings_for("<html><head></head><body></body></html>");
        assert!(keys.contains(&FilterKey::MetaDescriptonAll));
        assert!(keys.contains(&FilterKey::MetaDescriptonMissing));
    }

    #[test]
    fn description_below_min_length_is_flagged() {
        let html = r#"<html><head><meta name="description" content="too short"></head><body></body></html>"#;
        let keys = findings_for(html);
        assert!(keys.contains(&FilterKey::MetaDescriptonBelowXCharacters));
        assert!(!keys.contains(&FilterKey::MetaDescriptonOverXCharacters));
        assert!(!keys.contains(&FilterKey::MetaDescriptonMissing));
    }

    #[test]
    fn description_over_max_length_is_flagged() {
        let long = "x".repeat(200);
        let html = format!(
            r#"<html><head><meta name="description" content="{long}"></head><body></body></html>"#
        );
        let keys = findings_for(&html);
        assert!(keys.contains(&FilterKey::MetaDescriptonOverXCharacters));
        assert!(!keys.contains(&FilterKey::MetaDescriptonBelowXCharacters));
    }

    #[test]
    fn multiple_descriptions_flagged() {
        let html = r#"<html><head>
            <meta name="description" content="First description that is long enough for SERP tests pass">
            <meta name="description" content="Second description that is also long enough to display">
        </head></html>"#;
        let keys = findings_for(html);
        assert!(keys.contains(&FilterKey::MetaDescriptonMultiple));
    }

    #[test]
    fn description_in_body_is_outside_head() {
        let html = r#"<html><head></head><body>
            <meta name="description" content="Description shoved into the body tag by a broken CMS template">
        </body></html>"#;
        let keys = findings_for(html);
        assert!(keys.contains(&FilterKey::MetaDescriptionOutsideHead));
    }

    #[test]
    fn case_insensitive_name_attribute() {
        let html = r#"<html><head><meta name="Description" content="Uppercased name attribute should still be detected as a description tag for SEO"></head></html>"#;
        let keys = findings_for(html);
        assert!(!keys.contains(&FilterKey::MetaDescriptonMissing));
    }
}
