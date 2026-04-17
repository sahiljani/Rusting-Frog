//! Hand-authored `Issue Details` metadata — the copy Screaming Frog shows
//! in the right-hand "Issues" detail pane (Description + How To Fix).
//!
//! Not auto-generated: this is editorial content curated from SF's UI
//! and Google / WCAG / W3C guidance. Coverage is the common-issue
//! long tail (~70 filters); anything not covered returns `None` and the
//! UI degrades to "no guidance available".

use super::{FilterKey, FilterSeverity};

/// Priority of an issue finding, shown in the Issues grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssuePriority {
    High,
    Medium,
    Low,
}

impl IssuePriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::High => "High",
            Self::Medium => "Medium",
            Self::Low => "Low",
        }
    }
}

/// Coarse classification surfaced to the UI as the "Issue Type" column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueType {
    Issue,
    Warning,
    Opportunity,
    Info,
}

impl IssueType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Issue => "Issue",
            Self::Warning => "Warning",
            Self::Opportunity => "Opportunity",
            Self::Info => "Info",
        }
    }
}

impl FilterKey {
    /// Recommended remediation priority. `None` for non-issue / stat filters.
    pub fn priority(&self) -> Option<IssuePriority> {
        use FilterKey::*;
        if self.severity() == FilterSeverity::Stat {
            return None;
        }
        Some(match self {
            // --- High: hard-breaks or direct crawl/index blockers ---
            ResponseCodeClientError
            | ResponseCodeServerError
            | ResponseCodeInternalClientError
            | ResponseCodeInternalServerError
            | ResponseCodeInternalRedirectLoop
            | ResponseCodeInternalNoResponse
            | ResponseCodeNoResponse
            | ResponseCodeBlocked
            | ResponseCodeInternalBlocked
            | SecurityHttp
            | SecurityMixedContent
            | SecurityFormOnHttpPage
            | SecurityFormUrlInsecure
            | SecurityBadMimeType
            | CanonicalsMissing
            | CanonicalsMultipleConflicting
            | CanonicalsNonIndexableCanonical
            | TitleMissing
            | H1Missing
            | MetaDescriptonMissing
            | ValidationMissingHead
            | ValidationMissingBody
            | JavaScriptPagesWithJsErrors
            | JavaScriptPagesWithBlockedResources => IssuePriority::High,

            // --- Low: cosmetics / edge cases ---
            UrlUnderscores
            | UrlUppercase
            | UrlNonAsciiCharacters
            | UrlOverXCharacters
            | UrlContainsSpace
            | TitleOutsideHead
            | MetaDescriptionOutsideHead
            | DirectivesOutsideHead
            | CanonicalsOutsideHead
            | HreflangOutsideHead
            | MetaKeywordsMultiple
            | UrlMultipleSlashes
            | UrlRepetitivePath
            | UrlParameters => IssuePriority::Low,

            // --- Medium: everything else flagged as an Issue ---
            _ => IssuePriority::Medium,
        })
    }

    /// Coarse issue classification for the "Type" column. Maps severity +
    /// tab heuristics to SF's Issue / Warning / Opportunity / Info buckets.
    pub fn issue_type(&self) -> IssueType {
        use FilterKey::*;
        if self.severity() == FilterSeverity::Stat {
            return IssueType::Info;
        }
        match self {
            PagespeedUnminifiedCss
            | PagespeedUnminifiedJavaScript
            | PagespeedUnusedCssRules
            | PagespeedUnusedJavaScript
            | PagespeedBootupTime
            | PagespeedMainThreadWorkBreakdown
            | PagespeedLayoutShiftCulprits
            | PagespeedDocumentLatencyInsight
            | PagespeedOptimizeDomSize
            | PagespeedFontDisplay
            | PagespeedImproveImageDelivery
            | PagespeedLegacyJavaScript
            | PagespeedRenderBlockingRequests
            | PagespeedUseEfficientCacheLifetimes
            | PagespeedLcpRequestDiscovery
            | PagespeedForcedReflow
            | PagespeedAvoidEnormousNetworkPayloads
            | PagespeedNetworkDependencyTree
            | PagespeedDuplicatedJavaScript
            | ImagesOverXKb
            | ImagesIncorrectlySizedImages
            | ImagesMissingSizeAttributes
            | ContentLowContentPages
            | ContentReadabilityDifficult
            | ContentReadabilityVeryDifficult => IssueType::Opportunity,

            TitleBelowXCharacters
            | TitleOverXCharacters
            | TitleBelowXPixels
            | TitleOverXPixels
            | MetaDescriptonBelowXCharacters
            | MetaDescriptonOverXCharacters
            | MetaDescriptonBelowXPixels
            | MetaDescriptonOverXPixels
            | H1OverXCharacters
            | H2OverXCharacters
            | LinksHighCrawlDepth
            | LinksHighInternalOutlinks
            | LinksHighExternalOutlinks
            | ContentLanguageErrorsMisspelt
            | ContentLanguageErrorsGrammar
            | ContentNearDuplicates
            | AccessibilityScoreNeedsImprovement => IssueType::Warning,

            _ => IssueType::Issue,
        }
    }

    /// SF-style "Description" copy for the right-hand Issue Details pane.
    /// Returns `None` for filters we don't have curated copy for.
    pub fn description(&self) -> Option<&'static str> {
        use FilterKey::*;
        Some(match self {
            // --- Response codes ---
            ResponseCodeClientError | ResponseCodeInternalClientError =>
                "URLs returning a 4xx client-error response. These pages are unreachable to users and search engines, wasting crawl budget and breaking inbound links.",
            ResponseCodeServerError | ResponseCodeInternalServerError =>
                "URLs returning a 5xx server-error response. A persistent 5xx signals server instability and can cause Google to reduce crawl rate.",
            ResponseCodeRedirection | ResponseCodeInternalRedirection =>
                "URLs that respond with a 3xx redirect. Redirect chains hurt crawl efficiency and PageRank flow; replace internal links with the final destination.",
            ResponseCodeNoResponse | ResponseCodeInternalNoResponse =>
                "URLs the crawler could not receive a response from (DNS failure, connection refused, timeout).",
            ResponseCodeInternalRedirectChain =>
                "URLs that are part of a redirect chain of two or more hops before reaching a final 200.",
            ResponseCodeInternalRedirectLoop =>
                "URLs whose redirect targets form a loop — the chain never resolves.",
            ResponseCodeBlocked | ResponseCodeInternalBlocked =>
                "URLs blocked by robots.txt. Googlebot will not crawl these; ensure that is intentional.",

            // --- Titles ---
            TitleMissing =>
                "Pages without a <title> element. Without a title, search engines invent one from anchor text — lowering CTR and relevance.",
            TitleDuplicate =>
                "Pages that share an identical <title> with another indexable page. Duplicate titles make ranking harder and confuse users.",
            TitleOverXCharacters =>
                "Titles longer than the configured character threshold. Long titles are truncated with an ellipsis in search results.",
            TitleBelowXCharacters =>
                "Titles shorter than the configured character threshold. Short titles miss ranking opportunity and often lack brand or keyword terms.",
            TitleOverXPixels =>
                "Titles wider than the configured pixel width (default 561px). Google truncates over-wide titles with an ellipsis.",
            TitleBelowXPixels =>
                "Titles narrower than the configured pixel width. Consider adding more descriptive text to make use of the available space.",
            TitleSameAsH1 =>
                "Pages where the <title> is identical to the H1. They serve different SERP roles — the title should generally be more keyword-oriented.",
            TitleMultiple =>
                "Pages with more than one <title> element. Search engines pick one unpredictably.",

            // --- Meta descriptions ---
            MetaDescriptonMissing =>
                "Pages without a meta description. Google may generate its own from page text, often sub-optimally.",
            MetaDescriptonDuplicate =>
                "Pages sharing an identical meta description with another page. Unique descriptions improve SERP click-through rate.",
            MetaDescriptonOverXCharacters =>
                "Meta descriptions exceeding the configured character threshold — Google will truncate them.",
            MetaDescriptonBelowXCharacters =>
                "Meta descriptions shorter than the configured character threshold — likely missing useful SERP copy.",
            MetaDescriptonMultiple =>
                "Pages with more than one <meta name=\"description\"> element. Keep exactly one per page.",

            // --- H1/H2 ---
            H1Missing =>
                "Pages without an H1. The H1 is the primary on-page heading and a strong ranking signal.",
            H1Duplicate =>
                "Pages that share an identical H1 with another indexable page. Unique H1s help search engines differentiate page topics.",
            H1Multiple =>
                "Pages with more than one H1. Keep a single H1 that describes the primary topic.",
            H1OverXCharacters =>
                "H1s longer than the configured character threshold — usually a sign the heading isn't focused.",
            H2Missing =>
                "Pages without an H2. Sub-headings help crawlers and users understand content structure.",
            H2Duplicate =>
                "Pages sharing an identical H2 with another page.",
            H2Multiple =>
                "Pages with many H2 elements — consider whether the page is scoped to a single topic.",

            // --- Images ---
            ImagesMissingAltText =>
                "Images with an empty alt attribute. Alt text is required for accessibility and is used by Google Images.",
            ImagesMissingAltAttribute =>
                "Images missing the alt attribute entirely. Decorative images should use alt=\"\" explicitly.",
            ImagesAltTextOverXCharacters =>
                "Alt text longer than the configured threshold. Keep alt text concise and descriptive.",
            ImagesOverXKb =>
                "Images larger than the configured size threshold. Oversized images hurt LCP and Core Web Vitals.",
            ImagesIncorrectlySizedImages =>
                "Images whose rendered dimensions differ significantly from their intrinsic dimensions — browsers waste bandwidth and CPU.",
            ImagesMissingSizeAttributes =>
                "Images without width/height attributes. Omitting them causes cumulative layout shift (CLS).",

            // --- Canonicals ---
            CanonicalsMissing =>
                "Indexable HTML pages with no canonical link. Without one, duplicate-content signals can split across URL variants.",
            CanonicalsSelfReferencing =>
                "Pages whose canonical URL points to themselves — generally best practice.",
            CanonicalsCanonicalised =>
                "Pages with a canonical pointing elsewhere — indicating they are considered a duplicate of another page.",
            CanonicalsNonIndexableCanonical =>
                "Pages whose canonical target is itself non-indexable (noindex, redirect, 4xx, 5xx). The canonical signal is wasted.",
            CanonicalsMultiple =>
                "Pages with more than one canonical hint (rel=canonical link + HTTP header) pointing to different URLs. Search engines ignore conflicting signals.",
            CanonicalsMultipleConflicting =>
                "Pages with multiple canonical hints pointing to different URLs.",
            CanonicalsCanonicalIsRelative =>
                "Canonical URLs specified as relative paths. Absolute URLs are recommended to avoid resolution ambiguity.",
            CanonicalsUnlinked =>
                "Canonical target URLs that are not linked from anywhere else — risk of being missed by the crawler.",
            CanonicalsContainsFragmentUrl =>
                "Canonical URLs containing a # fragment. Search engines strip fragments — treated as a broken signal.",
            CanonicalsOutsideHead =>
                "Canonical link placed outside the <head>. Google only recognises it inside the head.",

            // --- Directives ---
            DirectivesNoindex =>
                "Pages with a noindex directive. They will be removed from search results once recrawled — verify intent.",
            DirectivesNofollow =>
                "Pages with a nofollow directive — outlinks do not pass PageRank.",
            DirectivesOutsideHead =>
                "Robots meta tag placed outside <head> — ignored by Google.",

            // --- Hreflang ---
            HreflangMissingReturnLinks =>
                "Hreflang annotations that point to a URL which does not return a corresponding hreflang pointing back. Return links are required.",
            HreflangNon200HreflangUrls =>
                "Hreflang annotations referencing a URL that does not respond with 200.",
            HreflangIncorrectLanguageCodes =>
                "Hreflang values that are not valid ISO 639-1 + ISO 3166-1 alpha-2 codes.",
            HreflangMissingSelfReference =>
                "Pages that use hreflang but don't include a self-referencing entry for their own language.",
            HreflangMissingXdefault =>
                "Hreflang cluster without an x-default entry — Google recommends one.",

            // --- Security ---
            SecurityHttp =>
                "URLs served over insecure HTTP. All traffic should be HTTPS; Chrome flags HTTP pages as Not Secure.",
            SecurityMixedContent =>
                "HTTPS pages loading sub-resources over HTTP. Browsers block mixed active content and flag passive mixed content.",
            SecurityFormUrlInsecure =>
                "Forms whose action attribute submits to an HTTP URL. Credentials and data could be intercepted.",
            SecurityFormOnHttpPage =>
                "Forms rendered on an HTTP page. Browsers show Not Secure warnings on any page with form inputs.",
            SecurityMissingHstsHeader =>
                "HTTPS responses without a Strict-Transport-Security header. HSTS prevents downgrade attacks.",
            SecurityMissingContentTypeHeader =>
                "Responses without a Content-Type header. Browsers may sniff and mis-interpret the content.",
            SecurityMissingCspHeader =>
                "Responses without a Content-Security-Policy header. CSP mitigates XSS and data-injection attacks.",
            SecurityBadMimeType =>
                "Responses with a MIME type that does not match the actual content.",

            // --- Links ---
            LinksHighCrawlDepth =>
                "URLs more than 3 clicks from the homepage. Deep pages accumulate less link equity and crawl less often.",
            LinksHighExternalOutlinks =>
                "Pages with an unusually high number of external outlinks — may appear spammy.",
            LinksHighInternalOutlinks =>
                "Pages with more than ~100 internal outlinks — PageRank dilutes across too many targets.",
            LinksNoAnchorTextOutlinks =>
                "Outlinks with no anchor text (image-only or empty). Reduces ranking signal to the target.",
            LinksNonDescriptiveAnchorTextOutlinks =>
                "Outlinks using generic anchors like \"click here\" or \"read more\" — poor ranking signal.",

            // --- URLs ---
            UrlUnderscores =>
                "URLs containing underscores. Google recommends hyphens as word separators.",
            UrlUppercase =>
                "URLs containing uppercase characters. Paths are case-sensitive — different cases risk duplication.",
            UrlParameters =>
                "URLs containing query parameters. Consider parameter handling or canonicalisation to avoid duplicates.",
            UrlOverXCharacters =>
                "URLs longer than the configured threshold. Long URLs are harder to share and less memorable.",
            UrlMultipleSlashes =>
                "URLs containing consecutive slashes — usually a CMS configuration bug.",
            UrlNonAsciiCharacters =>
                "URLs containing non-ASCII characters. Percent-encode them to avoid transport issues.",

            // --- Content ---
            ContentNearDuplicates =>
                "Pages whose content is a near-duplicate of another page (cosine similarity above the threshold).",
            ContentLowContentPages =>
                "Pages whose word count is below the low-content threshold.",
            ContentDuplicates =>
                "Pages with identical HTML content body.",
            ContentSoft404Pages =>
                "Pages that respond 200 but likely display a not-found message — Google treats these as soft-404s.",

            // --- JavaScript ---
            JavaScriptPagesWithJsErrors =>
                "Pages where Chromium raised a JavaScript console error during rendering.",
            JavaScriptPagesWithBlockedResources =>
                "Pages whose render required resources blocked by robots.txt — Googlebot sees a broken page.",
            JavaScriptCanonicalMismatch =>
                "Pages where the canonical from rendered HTML differs from the raw HTML canonical.",

            // --- Validation ---
            ValidationMissingHead =>
                "Pages missing a <head> element. The browser auto-creates one; directives placed before <html> are ignored.",
            ValidationMissingBody =>
                "Pages missing a <body> element.",
            ValidationInvalidElementsInHead =>
                "<head> containing elements that are invalid before <body> — browsers close <head> early and move them.",
            ValidationMultipleHeads =>
                "Pages with more than one <head> element.",
            ValidationMultipleBodies =>
                "Pages with more than one <body> element.",

            // --- Meta Keywords ---
            MetaKeywordsMissing =>
                "Pages without a <meta name=\"keywords\"> tag. Note: Google has ignored meta keywords since 2009 — this filter is surfaced only for completeness and legacy tooling compatibility.",
            MetaKeywordsDuplicate =>
                "Pages that share an identical meta keywords value. Only relevant if you are targeting a non-Google search engine that still uses the tag.",
            MetaKeywordsMultiple =>
                "Pages with more than one <meta name=\"keywords\"> tag. Keep at most one or remove entirely.",

            // --- Images (extras) ---
            ImagesBackgroundImages =>
                "Images rendered via CSS background-image. They are invisible to assistive technology and not indexed by Google Images.",

            // --- Directives (preview limits) ---
            DirectivesMaxSnippet =>
                "Pages declaring a max-snippet directive, limiting how much text Google may show in search result snippets.",
            DirectivesMaxImagePreview =>
                "Pages declaring a max-image-preview directive. Controls how large a thumbnail Google may use on SERPs.",
            DirectivesMaxVideoPreview =>
                "Pages declaring a max-video-preview directive. Controls how long a video snippet Google may show.",
            DirectivesUnavailableAfter =>
                "Pages with an unavailable_after directive — Google will drop them from the index after the given date.",

            // --- Hreflang (extras) ---
            HreflangMissing =>
                "Pages in an internationalised site that have no hreflang annotation. Without hreflang, Google may serve the wrong language to users.",
            HreflangContainsHreflang =>
                "Pages that contain at least one hreflang annotation — informational only.",
            HreflangUnlinkedHreflangUrls =>
                "Hreflang-referenced URLs that no other page links to. Risk of the crawler not discovering them.",

            // --- Structured data ---
            StructuredDataMissingStructuredData =>
                "Pages without any structured data markup. Consider adding relevant schema.org types to qualify for rich results.",
            StructuredDataValidationErrors =>
                "Structured data with schema.org validation errors — rich results may not be eligible.",
            StructuredDataGoogleValidationErrors =>
                "Structured data with Google-specific validation errors — the page will not be eligible for the relevant rich-result feature.",
            StructuredDataParseErrors =>
                "Structured data that could not be parsed (invalid JSON-LD / Microdata / RDFa syntax).",

            // --- Canonicals (extras) ---
            CanonicalsInvalidAttributes =>
                "Canonical link tags with attributes other than rel and href — some crawlers may ignore them.",
            CanonicalsContainsCanonical =>
                "Pages that contain a rel=canonical hint — informational only.",

            // --- Validation (extras) ---
            ValidationHeadNotFirstElement =>
                "<head> is not the first element inside <html>. Browsers insert content before it into an implicit <body>, breaking early directive parsing.",
            ValidationBodyElementPrecedingHtml =>
                "<body>-level content appears before the <html> opening — browsers silently move it, producing a malformed DOM.",
            ValidationDocumentOver15Mb =>
                "HTML document larger than 15 MB. Googlebot truncates responses above this threshold.",
            ValidationResourceOver15Mb =>
                "Sub-resource larger than 15 MB. Likely too heavy for mobile users and Core Web Vitals.",

            // --- Sitemaps ---
            SitemapsNotInSitemap =>
                "Indexable URLs that are not present in any XML sitemap. Add them so Google discovers and prioritises them.",
            SitemapsOrphanUrl =>
                "URLs listed in a sitemap but not linked from anywhere else. Either add internal links or remove from the sitemap.",
            SitemapsNonIndexableUrlInSitemap =>
                "Non-indexable URLs listed in a sitemap. Remove them — sitemaps should only contain canonical, indexable pages.",
            SitemapsOver50KUrls =>
                "Sitemap contains more than 50,000 URLs — the sitemaps.org limit. Split into multiple sitemaps with a sitemap index.",
            SitemapsOver50Mb =>
                "Sitemap file larger than 50 MB uncompressed — the sitemaps.org limit.",

            // --- Mobile ---
            MobileViewportNotSet =>
                "Pages without a viewport meta tag. Mobile browsers render at desktop width, forcing users to zoom.",
            MobileIllegibleFontSize =>
                "Pages with font-size < 12px — hard to read on mobile without zooming.",
            MobileTargetSize =>
                "Tap targets smaller than 48x48 CSS pixels or too close together.",

            _ => return None,
        })
    }

    /// SF-style "How To Fix" copy for the right-hand Issue Details pane.
    /// Returns `None` for filters we don't have curated copy for.
    pub fn how_to_fix(&self) -> Option<&'static str> {
        use FilterKey::*;
        Some(match self {
            ResponseCodeClientError | ResponseCodeInternalClientError =>
                "Identify the links pointing to these URLs (Inlinks tab) and either update them to working URLs or restore the resource. For genuinely removed pages, 301 to a relevant replacement.",
            ResponseCodeServerError | ResponseCodeInternalServerError =>
                "Check server error logs for the root cause, fix the underlying bug, then re-crawl to confirm the 5xx is gone.",
            ResponseCodeRedirection | ResponseCodeInternalRedirection =>
                "Update the source anchors to point directly at the final 200 URL rather than routing through a redirect.",
            ResponseCodeInternalRedirectChain =>
                "Shorten every chain to a single hop: source → final 200. Bulk-export the chains from Reports → Redirects.",
            ResponseCodeInternalRedirectLoop =>
                "Break the loop — fix the rules so the chain terminates at a 200 URL.",
            ResponseCodeBlocked | ResponseCodeInternalBlocked =>
                "If the block is intentional, confirm the URL should not be indexed. If unintentional, remove the matching rule from /robots.txt.",

            TitleMissing =>
                "Add a unique, descriptive <title> in the <head>. Target ~50–60 chars under 561px.",
            TitleDuplicate =>
                "Rewrite duplicates so each page has a unique title that reflects its topic. Use the Duplicate Details detail tab to see clusters.",
            TitleOverXCharacters | TitleOverXPixels =>
                "Shorten the title — keep the primary keyword early and trim brand or filler trailing words.",
            TitleBelowXCharacters | TitleBelowXPixels =>
                "Expand the title with descriptive, keyword-rich text; include the brand where appropriate.",
            TitleSameAsH1 =>
                "Differentiate: the title should be tuned for SERP click-through, the H1 for on-page hierarchy.",
            TitleMultiple =>
                "Remove the extra <title> elements so exactly one remains in <head>.",

            MetaDescriptonMissing =>
                "Write a compelling, unique meta description of 120–158 characters that summarises the page.",
            MetaDescriptonDuplicate =>
                "Rewrite duplicates to accurately describe each page's unique content.",
            MetaDescriptonOverXCharacters =>
                "Trim to under 158 characters so the description fits on a typical SERP.",
            MetaDescriptonBelowXCharacters =>
                "Extend the description with useful detail and a call to action.",

            H1Missing =>
                "Add an <h1> to the main content area describing the page's primary topic.",
            H1Multiple =>
                "Demote additional H1s to H2/H3 — keep exactly one H1.",
            H1Duplicate =>
                "Rewrite so each page has a unique H1.",

            ImagesMissingAltText | ImagesMissingAltAttribute =>
                "Add a descriptive alt attribute. Use alt=\"\" for purely decorative images, and meaningful text for content images.",
            ImagesOverXKb =>
                "Compress or resize the image. Prefer modern formats (WebP / AVIF). Target under 100KB where possible.",
            ImagesIncorrectlySizedImages =>
                "Serve images at their rendered dimensions using responsive <img srcset> or the <picture> element.",
            ImagesMissingSizeAttributes =>
                "Add width and height attributes to every <img> so the browser reserves layout space and CLS stays low.",

            CanonicalsMissing =>
                "Add <link rel=\"canonical\" href=\"…\"> in <head> pointing to the preferred URL (usually self).",
            CanonicalsNonIndexableCanonical =>
                "Point the canonical at an indexable URL — fix the target's noindex / redirect / error state, or pick a different canonical.",
            CanonicalsMultiple | CanonicalsMultipleConflicting =>
                "Keep a single canonical. If the HTTP Link header and the <link> tag disagree, remove one and align the values.",
            CanonicalsCanonicalIsRelative =>
                "Use an absolute URL (scheme + host + path) in the canonical.",

            DirectivesNoindex =>
                "If the page should be indexed, remove the noindex directive. If not, verify it's blocked only via meta robots and not robots.txt (Google can't honour noindex on blocked URLs).",

            HreflangMissingReturnLinks =>
                "Add the reciprocal hreflang entry on the target URL pointing back at the source.",
            HreflangIncorrectLanguageCodes =>
                "Use ISO 639-1 language + optional ISO 3166-1 alpha-2 region (e.g. en-GB, fr-CA).",
            HreflangMissingSelfReference =>
                "Ensure every hreflang cluster includes an entry pointing at its own URL with its own language code.",
            HreflangMissingXdefault =>
                "Add an x-default hreflang entry pointing at the language/region selector or global fallback URL.",

            SecurityHttp =>
                "Redirect HTTP → HTTPS site-wide (301) and update all internal links + canonicals to HTTPS.",
            SecurityMixedContent =>
                "Replace every http:// sub-resource with https://. Enable Content-Security-Policy: upgrade-insecure-requests as a defence in depth.",
            SecurityFormUrlInsecure =>
                "Change the form's action to an HTTPS URL.",
            SecurityMissingHstsHeader =>
                "Serve Strict-Transport-Security: max-age=63072000; includeSubDomains; preload on HTTPS responses.",
            SecurityMissingCspHeader =>
                "Serve a Content-Security-Policy header — start in Report-Only mode then enforce.",
            SecurityMissingContentTypeHeader =>
                "Set an explicit Content-Type header on every response.",

            LinksHighCrawlDepth =>
                "Flatten site architecture: link to deep pages from hubs / categories, add HTML sitemaps, add related-content blocks.",
            LinksHighInternalOutlinks =>
                "Prune or consolidate internal links. Consider splitting content or using category landing pages.",
            LinksNoAnchorTextOutlinks =>
                "Add descriptive anchor text (or use a meaningful alt for image links).",
            LinksNonDescriptiveAnchorTextOutlinks =>
                "Replace \"click here\" / \"read more\" with keyword-rich phrases that describe the target.",

            UrlUnderscores =>
                "Rename paths to use hyphens, 301 the old URLs to the new ones, and update internal links.",
            UrlUppercase =>
                "Normalize to lowercase with a 301 rewrite rule, then update internal links + canonicals.",
            UrlParameters =>
                "Canonicalise parameterised variants, or configure URL parameter handling server-side.",

            ContentNearDuplicates =>
                "Consolidate near-duplicate pages with a 301 to the canonical version, or differentiate the content.",
            ContentLowContentPages =>
                "Expand the page with more useful, unique content — or noindex if it's a thin utility page.",
            ContentSoft404Pages =>
                "Return an actual 404 / 410 status on not-found states instead of 200 + \"page not found\" copy.",

            JavaScriptPagesWithJsErrors =>
                "Open the Chrome Console Log detail tab for the URL, fix the reported errors in source.",
            JavaScriptPagesWithBlockedResources =>
                "Unblock render-required resources in /robots.txt (usually CSS / JS / fonts).",

            ValidationMissingHead | ValidationMissingBody =>
                "Add the missing element to the HTML skeleton.",
            ValidationInvalidElementsInHead =>
                "Move the offending element out of <head> into <body>. Browsers close <head> early when they see body content.",

            MetaKeywordsMissing =>
                "No action required for Google. If you target Yandex or other engines that still honour the tag, add <meta name=\"keywords\" content=\"…\">.",

            ImagesBackgroundImages =>
                "If the image conveys information, move it to an <img> tag with alt text. Keep CSS background-image only for purely decorative art.",

            HreflangMissing =>
                "Add hreflang annotations pointing at every language/region variant, including a self-reference and an x-default fallback.",

            StructuredDataMissingStructuredData =>
                "Add schema.org markup in JSON-LD format appropriate to the page type (Article, Product, FAQ, Breadcrumb, etc.).",
            StructuredDataValidationErrors | StructuredDataGoogleValidationErrors =>
                "Run the affected URLs through Google's Rich Results Test and fix the reported errors. The Structured Data Details detail tab lists each violation.",

            SitemapsNotInSitemap =>
                "Add the URL to the XML sitemap and resubmit via Search Console.",
            SitemapsNonIndexableUrlInSitemap =>
                "Remove non-indexable URLs from the sitemap. Sitemaps should only contain canonical, 200-status, indexable pages.",

            MobileViewportNotSet =>
                "Add <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"> to <head>.",
            MobileIllegibleFontSize =>
                "Set base font-size to at least 16px and use relative units (rem / em) for headings.",
            MobileTargetSize =>
                "Increase tap target padding so each interactive element is at least 48×48 CSS pixels.",

            _ => return None,
        })
    }
}
