//! External tab evaluator.
//!
//! Mirror of [`crate::internal`]: fires on URLs where `is_internal == false`,
//! emitting `ExternalAll` plus one content-type filter.

use sf_core::crawl::{ContentType, CrawlUrl};
use sf_core::filter_key::FilterKey;
use sf_core::tab::TabKey;

use crate::{EvalContext, Evaluator, Finding};

pub struct ExternalEvaluator;

impl Evaluator for ExternalEvaluator {
    fn tab(&self) -> TabKey {
        TabKey::External
    }

    fn evaluate(&self, url: &CrawlUrl, _ctx: &EvalContext) -> Vec<Finding> {
        if url.is_internal {
            return vec![];
        }

        let content_filter = match url.content_type {
            ContentType::Html => FilterKey::ExternalHtml,
            ContentType::JavaScript => FilterKey::ExternalJavaScript,
            ContentType::Css => FilterKey::ExternalCss,
            ContentType::Image => FilterKey::ExternalImages,
            ContentType::Pdf => FilterKey::ExternalPdf,
            ContentType::Plugin => FilterKey::ExternalPlugins,
            ContentType::Media => FilterKey::ExternalMedia,
            ContentType::Font => FilterKey::ExternalFonts,
            ContentType::Xml => FilterKey::ExternalXml,
            ContentType::Other => FilterKey::ExternalOther,
            ContentType::Unknown => FilterKey::ExternalUnknown,
        };

        vec![
            Finding {
                filter_key: FilterKey::ExternalAll,
            },
            Finding {
                filter_key: content_filter,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sf_core::config::CrawlConfig;
    use sf_core::crawl::{ContentType, CrawlUrl};
    use sf_core::id::{CrawlId, CrawlUrlId};

    fn url_fixture(is_internal: bool, ct: ContentType) -> CrawlUrl {
        CrawlUrl {
            id: CrawlUrlId::new(),
            crawl_id: CrawlId::new(),
            url: "https://external.example/".to_string(),
            url_hash: "h".to_string(),
            content_type: ct,
            status_code: Some(200),
            is_internal,
            depth: 1,
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

    fn eval(u: &CrawlUrl) -> Vec<FilterKey> {
        let cfg = CrawlConfig::default();
        let ctx = EvalContext {
            config: &cfg,
            html: None,
            parsed: None,
        };
        ExternalEvaluator
            .evaluate(u, &ctx)
            .into_iter()
            .map(|f| f.filter_key)
            .collect()
    }

    #[test]
    fn internal_url_produces_no_findings() {
        let u = url_fixture(true, ContentType::Html);
        assert!(eval(&u).is_empty());
    }

    #[test]
    fn external_html_emits_all_and_html() {
        let u = url_fixture(false, ContentType::Html);
        let keys = eval(&u);
        assert!(keys.contains(&FilterKey::ExternalAll));
        assert!(keys.contains(&FilterKey::ExternalHtml));
    }

    #[test]
    fn external_image_bucketed_correctly() {
        let u = url_fixture(false, ContentType::Image);
        let keys = eval(&u);
        assert!(keys.contains(&FilterKey::ExternalImages));
    }

    #[test]
    fn external_unknown_when_content_type_empty() {
        let u = url_fixture(false, ContentType::Unknown);
        let keys = eval(&u);
        assert!(keys.contains(&FilterKey::ExternalUnknown));
    }
}
