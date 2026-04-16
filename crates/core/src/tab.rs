use serde::{Deserialize, Serialize};

/// The 33 tabs in the main UI grid.
///
/// Ported from Java: `seo.spider.seoelements.id1377782850`
/// Each variant maps to an i18n key like "tab.page_titles.title"
/// and owns a set of FilterKeys (defined in filter_key.rs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TabKey {
    Internal,
    External,
    PageTitles,
    MetaDescription,
    MetaKeywords,
    H1,
    H2,
    Images,
    Canonicals,
    Pagination,
    Directives,
    Hreflang,
    JavaScript,
    Amp,
    Links,
    ResponseCode,
    Url,
    Content,
    Security,
    Sitemaps,
    StructuredData,
    Mobile,
    Validation,
    PageSpeed,
    Analytics,
    SearchConsole,
    LinkMetrics,
    Parity,
    CustomSearch,
    CustomExtraction,
    CustomJavaScript,
    Ai,
    Accessibility,
    /// "UNDEF" tab in the Java enum — used only by the UNKNOWN filter key.
    /// Not rendered in the UI; kept for parity with the source format.
    Undef,
}

impl TabKey {
    pub fn all() -> &'static [TabKey] {
        &[
            Self::Internal,
            Self::External,
            Self::PageTitles,
            Self::MetaDescription,
            Self::MetaKeywords,
            Self::H1,
            Self::H2,
            Self::Images,
            Self::Canonicals,
            Self::Pagination,
            Self::Directives,
            Self::Hreflang,
            Self::JavaScript,
            Self::Amp,
            Self::Links,
            Self::ResponseCode,
            Self::Url,
            Self::Content,
            Self::Security,
            Self::Sitemaps,
            Self::StructuredData,
            Self::Mobile,
            Self::Validation,
            Self::PageSpeed,
            Self::Analytics,
            Self::SearchConsole,
            Self::LinkMetrics,
            Self::Parity,
            Self::CustomSearch,
            Self::CustomExtraction,
            Self::CustomJavaScript,
            Self::Ai,
            Self::Accessibility,
            Self::Undef,
        ]
    }

    pub fn i18n_key(&self) -> &'static str {
        match self {
            Self::Internal => "tab.internal.title",
            Self::External => "tab.external.title",
            Self::PageTitles => "tab.page_titles.title",
            Self::MetaDescription => "tab.meta_description.title",
            Self::MetaKeywords => "tab.meta_keywords.title",
            Self::H1 => "tab.h1.title",
            Self::H2 => "tab.h2.title",
            Self::Images => "tab.images.title",
            Self::Canonicals => "tab.canonicals.title",
            Self::Pagination => "tab.pagination.title",
            Self::Directives => "tab.directives.title",
            Self::Hreflang => "tab.hreflang.title",
            Self::JavaScript => "tab.javascript.title",
            Self::Amp => "tab.amp.title",
            Self::Links => "tab.links.title",
            Self::ResponseCode => "tab.responsecode.title",
            Self::Url => "tab.url.title",
            Self::Content => "tab.content.title",
            Self::Security => "tab.security.title",
            Self::Sitemaps => "tab.sitemaps.title",
            Self::StructuredData => "tab.structured_data.title",
            Self::Mobile => "tab.mobile.title",
            Self::Validation => "tab.validation.title",
            Self::PageSpeed => "tab.page_speed.title",
            Self::Analytics => "tab.analytics.title",
            Self::SearchConsole => "tab.search_console.title",
            Self::LinkMetrics => "tab.link_metrics.title",
            Self::Parity => "tab.parity.title",
            Self::CustomSearch => "tab.custom_search.title",
            Self::CustomExtraction => "tab.custom_extraction.title",
            Self::CustomJavaScript => "tab.custom_javascript.title",
            Self::Ai => "tab.ai.title",
            Self::Accessibility => "tab.accessibility.name",
            Self::Undef => "UNDEF",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Internal => "Internal",
            Self::External => "External",
            Self::PageTitles => "Page Titles",
            Self::MetaDescription => "Meta Description",
            Self::MetaKeywords => "Meta Keywords",
            Self::H1 => "H1",
            Self::H2 => "H2",
            Self::Images => "Images",
            Self::Canonicals => "Canonicals",
            Self::Pagination => "Pagination",
            Self::Directives => "Directives",
            Self::Hreflang => "Hreflang",
            Self::JavaScript => "JavaScript",
            Self::Amp => "AMP",
            Self::Links => "Links",
            Self::ResponseCode => "Response Codes",
            Self::Url => "URL",
            Self::Content => "Content",
            Self::Security => "Security",
            Self::Sitemaps => "Sitemaps",
            Self::StructuredData => "Structured Data",
            Self::Mobile => "Mobile",
            Self::Validation => "Validation",
            Self::PageSpeed => "PageSpeed",
            Self::Analytics => "Analytics",
            Self::SearchConsole => "Search Console",
            Self::LinkMetrics => "Link Metrics",
            Self::Parity => "Parity",
            Self::CustomSearch => "Custom Search",
            Self::CustomExtraction => "Custom Extraction",
            Self::CustomJavaScript => "Custom JavaScript",
            Self::Ai => "AI",
            Self::Accessibility => "Accessibility",
            Self::Undef => "UNDEF",
        }
    }

    pub fn is_phase1(&self) -> bool {
        matches!(self, Self::Internal | Self::ResponseCode | Self::PageTitles)
    }

    pub fn has_dynamic_filters(&self) -> bool {
        matches!(
            self,
            Self::CustomSearch | Self::CustomExtraction | Self::CustomJavaScript | Self::Ai
        )
    }
}
