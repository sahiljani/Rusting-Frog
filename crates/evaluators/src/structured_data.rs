//! Structured Data tab evaluator (online slice).
//!
//! Implemented online (detection only — no schema validation):
//! - `StructuredDataAll` — every HTML page is a candidate.
//! - `StructuredDataContainsStructuredData` — page has at least one
//!   JSON-LD `<script type="application/ld+json">`, a Microdata
//!   `itemscope` element, or an RDFa `typeof` / `property` element.
//! - `StructuredDataMissingStructuredData` — HTML page with none of
//!   the above.
//! - `StructuredDataJsonldUrls` — at least one JSON-LD script.
//! - `StructuredDataMicrodataUrls` — at least one `itemscope`.
//! - `StructuredDataRdfaUrls` — at least one `typeof` or `property`.
//! - `StructuredDataParseErrors` — any JSON-LD script whose body
//!   fails `serde_json::from_str`.
//!
//! Deferred (need schema/feature validation):
//! `StructuredDataValidationErrors`, `StructuredDataValidationWarnings`,
//! `StructuredDataGoogleValidationErrors`,
//! `StructuredDataGoogleValidationWarnings`,
//! `StructuredDataGoogleFeatureDetected`.

use scraper::Selector;
use sf_core::crawl::{ContentType, CrawlUrl};
use sf_core::filter_key::FilterKey;
use sf_core::tab::TabKey;

use crate::{EvalContext, Evaluator, Finding};

pub struct StructuredDataEvaluator;

impl Evaluator for StructuredDataEvaluator {
    fn tab(&self) -> TabKey {
        TabKey::StructuredData
    }

    fn evaluate(&self, url: &CrawlUrl, ctx: &EvalContext) -> Vec<Finding> {
        if url.content_type != ContentType::Html {
            return vec![];
        }

        let mut findings = vec![Finding {
            filter_key: FilterKey::StructuredDataAll,
        }];

        let parsed = match ctx.parsed {
            Some(p) => p,
            None => return findings,
        };

        let jsonld_sel = Selector::parse(r#"script[type="application/ld+json"]"#).ok();
        let microdata_sel = Selector::parse("[itemscope]").ok();
        let rdfa_sel = Selector::parse("[typeof], [property]").ok();

        let mut jsonld_count = 0usize;
        let mut jsonld_parse_errors = 0usize;
        if let Some(sel) = &jsonld_sel {
            for el in parsed.select(sel) {
                jsonld_count += 1;
                let body: String = el.text().collect::<String>();
                let trimmed = body.trim();
                if trimmed.is_empty() {
                    jsonld_parse_errors += 1;
                    continue;
                }
                if serde_json::from_str::<serde_json::Value>(trimmed).is_err() {
                    jsonld_parse_errors += 1;
                }
            }
        }

        let microdata_count = microdata_sel
            .as_ref()
            .map(|sel| parsed.select(sel).count())
            .unwrap_or(0);
        let rdfa_count = rdfa_sel
            .as_ref()
            .map(|sel| parsed.select(sel).count())
            .unwrap_or(0);

        if jsonld_count > 0 {
            findings.push(Finding {
                filter_key: FilterKey::StructuredDataJsonldUrls,
            });
        }
        if microdata_count > 0 {
            findings.push(Finding {
                filter_key: FilterKey::StructuredDataMicrodataUrls,
            });
        }
        if rdfa_count > 0 {
            findings.push(Finding {
                filter_key: FilterKey::StructuredDataRdfaUrls,
            });
        }

        if jsonld_count + microdata_count + rdfa_count > 0 {
            findings.push(Finding {
                filter_key: FilterKey::StructuredDataContainsStructuredData,
            });
        } else {
            findings.push(Finding {
                filter_key: FilterKey::StructuredDataMissingStructuredData,
            });
        }

        if jsonld_parse_errors > 0 {
            findings.push(Finding {
                filter_key: FilterKey::StructuredDataParseErrors,
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

    fn eval(html: &str) -> Vec<FilterKey> {
        let cfg = CrawlConfig::default();
        let parsed = Html::parse_document(html);
        let ctx = EvalContext {
            config: &cfg,
            html: Some(html),
            parsed: Some(&parsed),
        };
        StructuredDataEvaluator
            .evaluate(&page("https://example.com/"), &ctx)
            .into_iter()
            .map(|f| f.filter_key)
            .collect()
    }

    #[test]
    fn page_with_valid_jsonld_fires_contains_and_jsonld_but_not_parse_error() {
        let keys = eval(
            r#"<html><head><script type="application/ld+json">{"@type":"WebPage"}</script></head></html>"#,
        );
        assert!(keys.contains(&FilterKey::StructuredDataContainsStructuredData));
        assert!(keys.contains(&FilterKey::StructuredDataJsonldUrls));
        assert!(!keys.contains(&FilterKey::StructuredDataParseErrors));
        assert!(!keys.contains(&FilterKey::StructuredDataMissingStructuredData));
    }

    #[test]
    fn malformed_jsonld_fires_parse_errors() {
        let keys = eval(
            r#"<html><head><script type="application/ld+json">{not json</script></head></html>"#,
        );
        assert!(keys.contains(&FilterKey::StructuredDataJsonldUrls));
        assert!(keys.contains(&FilterKey::StructuredDataParseErrors));
    }

    #[test]
    fn microdata_fires_microdata_flag() {
        let keys = eval(
            r#"<html><body><div itemscope itemtype="https://schema.org/Product"><span itemprop="name">X</span></div></body></html>"#,
        );
        assert!(keys.contains(&FilterKey::StructuredDataMicrodataUrls));
        assert!(keys.contains(&FilterKey::StructuredDataContainsStructuredData));
    }

    #[test]
    fn rdfa_fires_rdfa_flag() {
        let keys = eval(
            r#"<html><body><div typeof="Person" property="name">A</div></body></html>"#,
        );
        assert!(keys.contains(&FilterKey::StructuredDataRdfaUrls));
        assert!(keys.contains(&FilterKey::StructuredDataContainsStructuredData));
    }

    #[test]
    fn plain_page_fires_missing() {
        let keys = eval("<html><body><p>just text</p></body></html>");
        assert!(keys.contains(&FilterKey::StructuredDataMissingStructuredData));
        assert!(!keys.contains(&FilterKey::StructuredDataContainsStructuredData));
    }
}
