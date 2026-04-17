pub mod canonicals;
pub mod directives;
pub mod external;
pub mod headings;
pub mod images;
pub mod internal;
pub mod links;
pub mod meta_description;
pub mod meta_keywords;
pub mod page_titles;
pub mod pagination;
pub mod response_codes;

use sf_core::config::CrawlConfig;
use sf_core::crawl::CrawlUrl;
use sf_core::filter_key::FilterKey;

/// A finding is a (url, filter_key) pair — "this URL matches this filter."
/// The evaluator produces zero or more findings per URL.
#[derive(Debug, Clone)]
pub struct Finding {
    pub filter_key: FilterKey,
}

/// Context shared across all evaluators during a single URL evaluation.
pub struct EvalContext<'a> {
    pub config: &'a CrawlConfig,
    pub html: Option<&'a str>,
    pub parsed: Option<&'a scraper::Html>,
}

/// Every evaluator owns one tab and checks a URL against that tab's filters.
pub trait Evaluator: Send + Sync {
    fn tab(&self) -> sf_core::tab::TabKey;
    fn evaluate(&self, url: &CrawlUrl, ctx: &EvalContext) -> Vec<Finding>;
}

/// Build the set of Phase 1 evaluators.
pub fn phase1_evaluators() -> Vec<Box<dyn Evaluator>> {
    vec![
        Box::new(internal::InternalEvaluator),
        Box::new(external::ExternalEvaluator),
        Box::new(response_codes::ResponseCodeEvaluator),
        Box::new(page_titles::PageTitleEvaluator),
        Box::new(meta_description::MetaDescriptionEvaluator),
        Box::new(meta_keywords::MetaKeywordsEvaluator),
        Box::new(headings::H1Evaluator),
        Box::new(headings::H2Evaluator),
        Box::new(images::ImagesEvaluator),
        Box::new(links::LinksEvaluator),
        Box::new(canonicals::CanonicalsEvaluator),
        Box::new(pagination::PaginationEvaluator),
        Box::new(directives::DirectivesEvaluator),
    ]
}
