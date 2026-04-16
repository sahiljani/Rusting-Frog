use sf_core::crawl::{ContentType, CrawlUrl};
use sf_core::filter_key::FilterKey;
use sf_core::tab::TabKey;

use crate::{EvalContext, Evaluator, Finding};

pub struct InternalEvaluator;

impl Evaluator for InternalEvaluator {
    fn tab(&self) -> TabKey {
        TabKey::Internal
    }

    fn evaluate(&self, url: &CrawlUrl, _ctx: &EvalContext) -> Vec<Finding> {
        if !url.is_internal {
            return vec![];
        }

        let mut findings = vec![Finding {
            filter_key: FilterKey::InternalAll,
        }];

        let content_filter = match url.content_type {
            ContentType::Html => FilterKey::InternalHtml,
            ContentType::JavaScript => FilterKey::InternalJavaScript,
            ContentType::Css => FilterKey::InternalCss,
            ContentType::Image => FilterKey::InternalImages,
            ContentType::Pdf => FilterKey::InternalPdf,
            ContentType::Plugin => FilterKey::InternalPlugins,
            ContentType::Media => FilterKey::InternalMedia,
            ContentType::Font => FilterKey::InternalFonts,
            ContentType::Xml => FilterKey::InternalXml,
            ContentType::Other => FilterKey::InternalOther,
            ContentType::Unknown => FilterKey::InternalUnknown,
        };

        findings.push(Finding {
            filter_key: content_filter,
        });

        findings
    }
}
