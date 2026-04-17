//! Meta Keywords tab evaluator.
//!
//! Implemented filters (per-URL): MetaKeywordsAll, MetaKeywordsMissing,
//! MetaKeywordsMultiple.
//!
//! Skipped: MetaKeywordsDuplicate is cross-URL (post-crawl).

use scraper::Selector;
use sf_core::crawl::{ContentType, CrawlUrl};
use sf_core::filter_key::FilterKey;
use sf_core::tab::TabKey;

use crate::{EvalContext, Evaluator, Finding};

pub struct MetaKeywordsEvaluator;

impl Evaluator for MetaKeywordsEvaluator {
    fn tab(&self) -> TabKey {
        TabKey::MetaKeywords
    }

    fn evaluate(&self, url: &CrawlUrl, ctx: &EvalContext) -> Vec<Finding> {
        if url.content_type != ContentType::Html {
            return vec![];
        }

        let mut findings = vec![Finding {
            filter_key: FilterKey::MetaKeywordsAll,
        }];

        let parsed = match ctx.parsed {
            Some(p) => p,
            None => return findings,
        };

        let keywords = extract_meta_keywords(parsed);

        if keywords.is_empty() {
            findings.push(Finding {
                filter_key: FilterKey::MetaKeywordsMissing,
            });
            return findings;
        }

        if keywords.len() > 1 {
            findings.push(Finding {
                filter_key: FilterKey::MetaKeywordsMultiple,
            });
        }

        findings
    }
}

fn extract_meta_keywords(html: &scraper::Html) -> Vec<String> {
    let sel = Selector::parse("meta[name][content]").expect("valid selector");
    html.select(&sel)
        .filter(|el| {
            el.value()
                .attr("name")
                .map(|n| n.eq_ignore_ascii_case("keywords"))
                .unwrap_or(false)
        })
        .filter_map(|el| el.value().attr("content"))
        .map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty())
        .collect()
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

    fn findings_for(html: &str) -> Vec<FilterKey> {
        let cfg = CrawlConfig::default();
        let parsed = Html::parse_document(html);
        let ctx = EvalContext {
            config: &cfg,
            html: Some(html),
            parsed: Some(&parsed),
        };
        MetaKeywordsEvaluator
            .evaluate(&url_fixture(), &ctx)
            .into_iter()
            .map(|f| f.filter_key)
            .collect()
    }

    #[test]
    fn always_emits_all() {
        let keys = findings_for("<html></html>");
        assert!(keys.contains(&FilterKey::MetaKeywordsAll));
    }

    #[test]
    fn no_keywords_flags_missing() {
        let keys = findings_for("<html><head></head></html>");
        assert!(keys.contains(&FilterKey::MetaKeywordsMissing));
    }

    #[test]
    fn two_keyword_tags_flags_multiple() {
        let html = r#"<html><head>
            <meta name="keywords" content="seo, crawler">
            <meta name="keywords" content="second, tag, here">
        </head></html>"#;
        let keys = findings_for(html);
        assert!(keys.contains(&FilterKey::MetaKeywordsMultiple));
        assert!(!keys.contains(&FilterKey::MetaKeywordsMissing));
    }

    #[test]
    fn empty_content_is_missing() {
        let html = r#"<html><head><meta name="keywords" content="   "></head></html>"#;
        let keys = findings_for(html);
        assert!(keys.contains(&FilterKey::MetaKeywordsMissing));
    }
}
