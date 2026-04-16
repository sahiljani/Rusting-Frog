use serde::{Deserialize, Serialize};

use crate::tab::TabKey;

/// When a filter's evaluation runs.
///
/// Ported from Java: `uk.co.screamingfrog.seospider.storage.db.FilterKeyType`
/// - NORMAL: evaluated inline as each URL is fetched and parsed
/// - PostCrawlAnalysis: only after the entire crawl completes (redirect chains, orphans, etc.)
/// - SpellingAndGrammar: requires LanguageTool sidecar (Phase 2+)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterKeyType {
    Normal,
    PostCrawlAnalysis,
    SpellingAndGrammar,
}

/// Whether a filter represents an issue or an informational stat.
///
/// Ported from Java: `seo.spider.seoelements.id706500854`
/// ISSUE = red (something wrong), STAT = green (informational count)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterSeverity {
    Issue,
    Stat,
}

/// Every built-in filter in the system.
///
/// Ported from Java: `seo.spider.seoelements.SeoElementFilterKey`
///
/// Each variant carries its tab, sort order, i18n key, whether it has
/// a configurable watermark threshold, its evaluation timing, and severity.
///
/// Phase 1 implements: Internal (12), Response Codes (35), Page Titles (10).
/// Other tabs are declared but not yet evaluated — the enum is the contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterKey {
    // ── Internal tab (12 filters) ──────────────────────────────────
    // Content-type buckets for internally-discovered URLs.
    // "All" is a stat; the rest are content-type slices.
    InternalAll,
    InternalHtml,
    InternalJavaScript,
    InternalCss,
    InternalImages,
    InternalPdf,
    InternalPlugins,
    InternalMedia,
    InternalFonts,
    InternalXml,
    InternalOther,
    InternalUnknown,

    // ── Response Codes tab (35 filters) ────────────────────────────
    // Three sections: combined (all URLs), internal-only, external-only.
    // Each section has: all, blocked, blocked resource, no response,
    // success (2xx), redirection (3xx), JS redirect, meta-refresh redirect,
    // HTTP refresh redirect, client error (4xx), server error (5xx).
    // Internal section adds redirect chain + redirect loop (post-crawl).

    // Combined (all URLs)
    ResponseCodeAll,
    ResponseCodeBlocked,
    ResponseCodeBlockedResource,
    ResponseCodeNoResponse,
    ResponseCodeSuccess,
    ResponseCodeRedirection,
    ResponseCodeRedirectionJs,
    ResponseCodeRedirectionMetaRefresh,
    ResponseCodeRedirectionHttpRefresh,
    ResponseCodeClientError,
    ResponseCodeServerError,

    // Internal URLs only
    ResponseCodeInternalAll,
    ResponseCodeInternalBlocked,
    ResponseCodeInternalBlockedResource,
    ResponseCodeInternalNoResponse,
    ResponseCodeInternalSuccess,
    ResponseCodeInternalRedirection,
    ResponseCodeInternalRedirectionJs,
    ResponseCodeInternalRedirectionMetaRefresh,
    ResponseCodeInternalRedirectionHttpRefresh,
    ResponseCodeInternalRedirectChain,
    ResponseCodeInternalRedirectLoop,
    ResponseCodeInternalClientError,
    ResponseCodeInternalServerError,

    // External URLs only
    ResponseCodeExternalAll,
    ResponseCodeExternalBlocked,
    ResponseCodeExternalBlockedResource,
    ResponseCodeExternalNoResponse,
    ResponseCodeExternalSuccess,
    ResponseCodeExternalRedirection,
    ResponseCodeExternalRedirectionJs,
    ResponseCodeExternalRedirectionMetaRefresh,
    ResponseCodeExternalRedirectionHttpRefresh,
    ResponseCodeExternalClientError,
    ResponseCodeExternalServerError,

    // ── Page Titles tab (10 filters) ───────────────────────────────
    // "All" is stat; the rest are issues.
    // "Over X" / "Below X" use watermark thresholds from config.
    // Pixel-width filters compare against SERP pixel budget.
    TitleAll,
    TitleMissing,
    TitleDuplicate,
    TitleOverXCharacters,
    TitleBelowXCharacters,
    TitleOverXPixels,
    TitleBelowXPixels,
    TitleSameAsH1,
    TitleMultiple,
    TitleOutsideHead,
}

impl FilterKey {
    pub fn tab(&self) -> TabKey {
        match self {
            Self::InternalAll
            | Self::InternalHtml
            | Self::InternalJavaScript
            | Self::InternalCss
            | Self::InternalImages
            | Self::InternalPdf
            | Self::InternalPlugins
            | Self::InternalMedia
            | Self::InternalFonts
            | Self::InternalXml
            | Self::InternalOther
            | Self::InternalUnknown => TabKey::Internal,

            Self::ResponseCodeAll
            | Self::ResponseCodeBlocked
            | Self::ResponseCodeBlockedResource
            | Self::ResponseCodeNoResponse
            | Self::ResponseCodeSuccess
            | Self::ResponseCodeRedirection
            | Self::ResponseCodeRedirectionJs
            | Self::ResponseCodeRedirectionMetaRefresh
            | Self::ResponseCodeRedirectionHttpRefresh
            | Self::ResponseCodeClientError
            | Self::ResponseCodeServerError
            | Self::ResponseCodeInternalAll
            | Self::ResponseCodeInternalBlocked
            | Self::ResponseCodeInternalBlockedResource
            | Self::ResponseCodeInternalNoResponse
            | Self::ResponseCodeInternalSuccess
            | Self::ResponseCodeInternalRedirection
            | Self::ResponseCodeInternalRedirectionJs
            | Self::ResponseCodeInternalRedirectionMetaRefresh
            | Self::ResponseCodeInternalRedirectionHttpRefresh
            | Self::ResponseCodeInternalRedirectChain
            | Self::ResponseCodeInternalRedirectLoop
            | Self::ResponseCodeInternalClientError
            | Self::ResponseCodeInternalServerError
            | Self::ResponseCodeExternalAll
            | Self::ResponseCodeExternalBlocked
            | Self::ResponseCodeExternalBlockedResource
            | Self::ResponseCodeExternalNoResponse
            | Self::ResponseCodeExternalSuccess
            | Self::ResponseCodeExternalRedirection
            | Self::ResponseCodeExternalRedirectionJs
            | Self::ResponseCodeExternalRedirectionMetaRefresh
            | Self::ResponseCodeExternalRedirectionHttpRefresh
            | Self::ResponseCodeExternalClientError
            | Self::ResponseCodeExternalServerError => TabKey::ResponseCode,

            Self::TitleAll
            | Self::TitleMissing
            | Self::TitleDuplicate
            | Self::TitleOverXCharacters
            | Self::TitleBelowXCharacters
            | Self::TitleOverXPixels
            | Self::TitleBelowXPixels
            | Self::TitleSameAsH1
            | Self::TitleMultiple
            | Self::TitleOutsideHead => TabKey::PageTitles,
        }
    }

    pub fn filter_key_type(&self) -> FilterKeyType {
        match self {
            Self::ResponseCodeInternalRedirectChain
            | Self::ResponseCodeInternalRedirectLoop => FilterKeyType::PostCrawlAnalysis,
            _ => FilterKeyType::Normal,
        }
    }

    pub fn severity(&self) -> FilterSeverity {
        match self {
            // "All" filters are informational stats
            Self::InternalAll
            | Self::InternalHtml
            | Self::InternalJavaScript
            | Self::InternalCss
            | Self::InternalImages
            | Self::InternalPdf
            | Self::InternalPlugins
            | Self::InternalMedia
            | Self::InternalFonts
            | Self::InternalXml
            | Self::InternalOther
            | Self::InternalUnknown
            | Self::ResponseCodeAll
            | Self::ResponseCodeSuccess
            | Self::ResponseCodeInternalAll
            | Self::ResponseCodeInternalSuccess
            | Self::ResponseCodeExternalAll
            | Self::ResponseCodeExternalSuccess
            | Self::TitleAll => FilterSeverity::Stat,

            // Everything else is an issue
            _ => FilterSeverity::Issue,
        }
    }

    pub fn has_watermark(&self) -> bool {
        matches!(
            self,
            Self::TitleOverXCharacters
                | Self::TitleBelowXCharacters
                | Self::TitleOverXPixels
                | Self::TitleBelowXPixels
        )
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::InternalAll => "All",
            Self::InternalHtml => "HTML",
            Self::InternalJavaScript => "JavaScript",
            Self::InternalCss => "CSS",
            Self::InternalImages => "Images",
            Self::InternalPdf => "PDF",
            Self::InternalPlugins => "Plugins",
            Self::InternalMedia => "Media",
            Self::InternalFonts => "Fonts",
            Self::InternalXml => "XML",
            Self::InternalOther => "Other",
            Self::InternalUnknown => "Unknown",

            Self::ResponseCodeAll => "All",
            Self::ResponseCodeBlocked => "Blocked by Robots.txt",
            Self::ResponseCodeBlockedResource => "Blocked Resource",
            Self::ResponseCodeNoResponse => "No Response",
            Self::ResponseCodeSuccess => "Success (2xx)",
            Self::ResponseCodeRedirection => "Redirection (3xx)",
            Self::ResponseCodeRedirectionJs => "Redirection (JavaScript)",
            Self::ResponseCodeRedirectionMetaRefresh => "Redirection (Meta Refresh)",
            Self::ResponseCodeRedirectionHttpRefresh => "Redirection (HTTP Refresh)",
            Self::ResponseCodeClientError => "Client Error (4xx)",
            Self::ResponseCodeServerError => "Server Error (5xx)",

            Self::ResponseCodeInternalAll => "Internal - All",
            Self::ResponseCodeInternalBlocked => "Internal - Blocked by Robots.txt",
            Self::ResponseCodeInternalBlockedResource => "Internal - Blocked Resource",
            Self::ResponseCodeInternalNoResponse => "Internal - No Response",
            Self::ResponseCodeInternalSuccess => "Internal - Success (2xx)",
            Self::ResponseCodeInternalRedirection => "Internal - Redirection (3xx)",
            Self::ResponseCodeInternalRedirectionJs => "Internal - Redirection (JavaScript)",
            Self::ResponseCodeInternalRedirectionMetaRefresh => "Internal - Redirection (Meta Refresh)",
            Self::ResponseCodeInternalRedirectionHttpRefresh => "Internal - Redirection (HTTP Refresh)",
            Self::ResponseCodeInternalRedirectChain => "Internal - Redirect Chain",
            Self::ResponseCodeInternalRedirectLoop => "Internal - Redirect Loop",
            Self::ResponseCodeInternalClientError => "Internal - Client Error (4xx)",
            Self::ResponseCodeInternalServerError => "Internal - Server Error (5xx)",

            Self::ResponseCodeExternalAll => "External - All",
            Self::ResponseCodeExternalBlocked => "External - Blocked by Robots.txt",
            Self::ResponseCodeExternalBlockedResource => "External - Blocked Resource",
            Self::ResponseCodeExternalNoResponse => "External - No Response",
            Self::ResponseCodeExternalSuccess => "External - Success (2xx)",
            Self::ResponseCodeExternalRedirection => "External - Redirection (3xx)",
            Self::ResponseCodeExternalRedirectionJs => "External - Redirection (JavaScript)",
            Self::ResponseCodeExternalRedirectionMetaRefresh => "External - Redirection (Meta Refresh)",
            Self::ResponseCodeExternalRedirectionHttpRefresh => "External - Redirection (HTTP Refresh)",
            Self::ResponseCodeExternalClientError => "External - Client Error (4xx)",
            Self::ResponseCodeExternalServerError => "External - Server Error (5xx)",

            Self::TitleAll => "All",
            Self::TitleMissing => "Missing",
            Self::TitleDuplicate => "Duplicate",
            Self::TitleOverXCharacters => "Over X Characters",
            Self::TitleBelowXCharacters => "Below X Characters",
            Self::TitleOverXPixels => "Over X Pixels",
            Self::TitleBelowXPixels => "Below X Pixels",
            Self::TitleSameAsH1 => "Same as H1",
            Self::TitleMultiple => "Multiple",
            Self::TitleOutsideHead => "Outside <head>",
        }
    }

    pub fn for_tab(tab: TabKey) -> Vec<FilterKey> {
        ALL_FILTER_KEYS
            .iter()
            .filter(|k| k.tab() == tab)
            .copied()
            .collect()
    }
}

static ALL_FILTER_KEYS: &[FilterKey] = &[
    FilterKey::InternalAll,
    FilterKey::InternalHtml,
    FilterKey::InternalJavaScript,
    FilterKey::InternalCss,
    FilterKey::InternalImages,
    FilterKey::InternalPdf,
    FilterKey::InternalPlugins,
    FilterKey::InternalMedia,
    FilterKey::InternalFonts,
    FilterKey::InternalXml,
    FilterKey::InternalOther,
    FilterKey::InternalUnknown,
    FilterKey::ResponseCodeAll,
    FilterKey::ResponseCodeBlocked,
    FilterKey::ResponseCodeBlockedResource,
    FilterKey::ResponseCodeNoResponse,
    FilterKey::ResponseCodeSuccess,
    FilterKey::ResponseCodeRedirection,
    FilterKey::ResponseCodeRedirectionJs,
    FilterKey::ResponseCodeRedirectionMetaRefresh,
    FilterKey::ResponseCodeRedirectionHttpRefresh,
    FilterKey::ResponseCodeClientError,
    FilterKey::ResponseCodeServerError,
    FilterKey::ResponseCodeInternalAll,
    FilterKey::ResponseCodeInternalBlocked,
    FilterKey::ResponseCodeInternalBlockedResource,
    FilterKey::ResponseCodeInternalNoResponse,
    FilterKey::ResponseCodeInternalSuccess,
    FilterKey::ResponseCodeInternalRedirection,
    FilterKey::ResponseCodeInternalRedirectionJs,
    FilterKey::ResponseCodeInternalRedirectionMetaRefresh,
    FilterKey::ResponseCodeInternalRedirectionHttpRefresh,
    FilterKey::ResponseCodeInternalRedirectChain,
    FilterKey::ResponseCodeInternalRedirectLoop,
    FilterKey::ResponseCodeInternalClientError,
    FilterKey::ResponseCodeInternalServerError,
    FilterKey::ResponseCodeExternalAll,
    FilterKey::ResponseCodeExternalBlocked,
    FilterKey::ResponseCodeExternalBlockedResource,
    FilterKey::ResponseCodeExternalNoResponse,
    FilterKey::ResponseCodeExternalSuccess,
    FilterKey::ResponseCodeExternalRedirection,
    FilterKey::ResponseCodeExternalRedirectionJs,
    FilterKey::ResponseCodeExternalRedirectionMetaRefresh,
    FilterKey::ResponseCodeExternalRedirectionHttpRefresh,
    FilterKey::ResponseCodeExternalClientError,
    FilterKey::ResponseCodeExternalServerError,
    FilterKey::TitleAll,
    FilterKey::TitleMissing,
    FilterKey::TitleDuplicate,
    FilterKey::TitleOverXCharacters,
    FilterKey::TitleBelowXCharacters,
    FilterKey::TitleOverXPixels,
    FilterKey::TitleBelowXPixels,
    FilterKey::TitleSameAsH1,
    FilterKey::TitleMultiple,
    FilterKey::TitleOutsideHead,
];
