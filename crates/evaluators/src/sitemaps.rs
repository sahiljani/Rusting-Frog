//! Sitemaps tab evaluator (stub).
//!
//! Every filter in this tab requires either a parsed XML sitemap set
//! (URLs in sitemap, multi-sitemap membership, per-sitemap URL count and
//! byte size) or a cross-URL pass (orphan URLs, not-in-sitemap). None
//! of that is available in the online evaluation pass yet — sitemap
//! discovery + parsing is a separate workstream.
//!
//! For now we emit `SitemapsAll` on every HTML page so the tab is
//! visible in overview counts; everything else will be populated by the
//! post-crawl analysis job.

use sf_core::crawl::{ContentType, CrawlUrl};
use sf_core::filter_key::FilterKey;
use sf_core::tab::TabKey;

use crate::{EvalContext, Evaluator, Finding};

pub struct SitemapsEvaluator;

impl Evaluator for SitemapsEvaluator {
    fn tab(&self) -> TabKey {
        TabKey::Sitemaps
    }

    fn evaluate(&self, url: &CrawlUrl, _ctx: &EvalContext) -> Vec<Finding> {
        if url.content_type != ContentType::Html {
            return vec![];
        }
        vec![Finding {
            filter_key: FilterKey::SitemapsAll,
        }]
    }
}
