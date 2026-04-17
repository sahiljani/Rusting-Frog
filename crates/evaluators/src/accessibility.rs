//! Accessibility tab evaluator (static-HTML slice).
//!
//! The SF product integrates axe-core inside the headless browser
//! and reports every violated rule as a filter key. The full axe
//! run needs computed styles (for color-contrast), a real layout
//! (for target-size, region, landmarks), and JavaScript execution
//! (ARIA state). That pass belongs to the render worker.
//!
//! Until the render worker lands we still check the cheap
//! DOM-only rules that can be evaluated purely from the parsed
//! HTML:
//!
//! - `AccessibilityAll` — every HTML page is an accessibility
//!   candidate.
//! - `AccessibilityRuleHtmlHasLang` — `<html>` missing a `lang`
//!   attribute.
//! - `AccessibilityRuleHtmlLangValid` — `<html lang="…">` whose
//!   value is not a BCP-47-shaped tag.
//! - `AccessibilityRuleDocumentTitle` — missing or empty `<title>`.
//! - `AccessibilityRulePageHasHeadingOne` — no `<h1>` anywhere in
//!   the document.
//! - `AccessibilityRuleMetaRefresh` — page uses
//!   `<meta http-equiv="refresh">` (WCAG 2.2.1).
//! - `AccessibilityRuleImageAlt` — `<img>` with no `alt`
//!   attribute at all (empty alt is intentionally decorative and
//!   not flagged here; axe matches).
//! - `AccessibilityRuleFrameTitle` — `<iframe>` without a
//!   non-empty `title` attribute.
//! - `AccessibilityRuleButtonName` — `<button>` with no
//!   accessible name (no text content and no `aria-label` /
//!   `aria-labelledby`).
//! - `AccessibilityRuleLinkName` — `<a href>` with no accessible
//!   name (empty text and no `aria-label` / `aria-labelledby` /
//!   `title`).
//! - `AccessibilityRuleInputImageAlt` — `<input type="image">`
//!   without `alt` / `aria-label`.
//! - `AccessibilityRuleBlink` — `<blink>` element.
//! - `AccessibilityRuleMarquee` — `<marquee>` element.
//!
//! All other ~90 axe rule filters (color-contrast, target-size,
//! region, landmark-* , ARIA-state, nested-interactive, …) and
//! the three score buckets (`AccessibilityScorePoor|NeedsImprovement|Good`)
//! stay deferred to the render-worker integration.

use scraper::{ElementRef, Selector};
use sf_core::crawl::{ContentType, CrawlUrl};
use sf_core::filter_key::FilterKey;
use sf_core::tab::TabKey;

use crate::{EvalContext, Evaluator, Finding};

pub struct AccessibilityEvaluator;

impl Evaluator for AccessibilityEvaluator {
    fn tab(&self) -> TabKey {
        TabKey::Accessibility
    }

    fn evaluate(&self, url: &CrawlUrl, ctx: &EvalContext) -> Vec<Finding> {
        if url.content_type != ContentType::Html {
            return vec![];
        }

        let mut findings = vec![Finding {
            filter_key: FilterKey::AccessibilityAll,
        }];

        let parsed = match ctx.parsed {
            Some(p) => p,
            None => return findings,
        };

        let html_sel = Selector::parse("html").ok();
        if let Some(sel) = &html_sel {
            if let Some(html_el) = parsed.select(sel).next() {
                match html_el.value().attr("lang") {
                    None => findings.push(Finding {
                        filter_key: FilterKey::AccessibilityRuleHtmlHasLang,
                    }),
                    Some(lang) if !is_bcp47_ish(lang) => findings.push(Finding {
                        filter_key: FilterKey::AccessibilityRuleHtmlLangValid,
                    }),
                    _ => {}
                }
            }
        }

        if let Some(sel) = Selector::parse("title").ok() {
            let has_title = parsed
                .select(&sel)
                .any(|el| !el.text().collect::<String>().trim().is_empty());
            if !has_title {
                findings.push(Finding {
                    filter_key: FilterKey::AccessibilityRuleDocumentTitle,
                });
            }
        }

        if let Some(sel) = Selector::parse("h1").ok() {
            if parsed.select(&sel).next().is_none() {
                findings.push(Finding {
                    filter_key: FilterKey::AccessibilityRulePageHasHeadingOne,
                });
            }
        }

        if let Some(sel) = Selector::parse(r#"meta[http-equiv="refresh" i]"#).ok() {
            if parsed.select(&sel).next().is_some() {
                findings.push(Finding {
                    filter_key: FilterKey::AccessibilityRuleMetaRefresh,
                });
            }
        }

        if let Some(sel) = Selector::parse("img").ok() {
            let any_missing_alt = parsed.select(&sel).any(|el| el.value().attr("alt").is_none());
            if any_missing_alt {
                findings.push(Finding {
                    filter_key: FilterKey::AccessibilityRuleImageAlt,
                });
            }
        }

        if let Some(sel) = Selector::parse("iframe").ok() {
            let any_bad_frame = parsed.select(&sel).any(|el| {
                el.value()
                    .attr("title")
                    .map(|t| t.trim().is_empty())
                    .unwrap_or(true)
            });
            if any_bad_frame {
                findings.push(Finding {
                    filter_key: FilterKey::AccessibilityRuleFrameTitle,
                });
            }
        }

        if let Some(sel) = Selector::parse("button").ok() {
            if parsed.select(&sel).any(|el| !has_accessible_name(&el)) {
                findings.push(Finding {
                    filter_key: FilterKey::AccessibilityRuleButtonName,
                });
            }
        }

        if let Some(sel) = Selector::parse("a[href]").ok() {
            if parsed.select(&sel).any(|el| !has_accessible_name(&el)) {
                findings.push(Finding {
                    filter_key: FilterKey::AccessibilityRuleLinkName,
                });
            }
        }

        if let Some(sel) = Selector::parse(r#"input[type="image" i]"#).ok() {
            let any_missing = parsed.select(&sel).any(|el| {
                let alt = el.value().attr("alt").map(|s| s.trim()).unwrap_or("");
                let aria = el
                    .value()
                    .attr("aria-label")
                    .map(|s| s.trim())
                    .unwrap_or("");
                alt.is_empty() && aria.is_empty()
            });
            if any_missing {
                findings.push(Finding {
                    filter_key: FilterKey::AccessibilityRuleInputImageAlt,
                });
            }
        }

        if let Some(sel) = Selector::parse("blink").ok() {
            if parsed.select(&sel).next().is_some() {
                findings.push(Finding {
                    filter_key: FilterKey::AccessibilityRuleBlink,
                });
            }
        }

        if let Some(sel) = Selector::parse("marquee").ok() {
            if parsed.select(&sel).next().is_some() {
                findings.push(Finding {
                    filter_key: FilterKey::AccessibilityRuleMarquee,
                });
            }
        }

        findings
    }
}

fn is_bcp47_ish(tag: &str) -> bool {
    let tag = tag.trim();
    if tag.is_empty() {
        return false;
    }
    let mut parts = tag.split('-');
    let primary = match parts.next() {
        Some(p) => p,
        None => return false,
    };
    if !(primary.len() == 2 || primary.len() == 3) {
        return false;
    }
    if !primary.chars().all(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    for sub in parts {
        if sub.is_empty() {
            return false;
        }
        let is_region = sub.len() == 2 && sub.chars().all(|c| c.is_ascii_alphabetic());
        let is_script = sub.len() == 4 && sub.chars().all(|c| c.is_ascii_alphabetic());
        let is_numeric_region = sub.len() == 3 && sub.chars().all(|c| c.is_ascii_digit());
        let is_variant = sub.len() >= 5 && sub.chars().all(|c| c.is_ascii_alphanumeric());
        if !(is_region || is_script || is_numeric_region || is_variant) {
            return false;
        }
    }
    true
}

fn has_accessible_name(el: &ElementRef) -> bool {
    let v = el.value();
    if let Some(label) = v.attr("aria-label") {
        if !label.trim().is_empty() {
            return true;
        }
    }
    if v.attr("aria-labelledby")
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
    {
        return true;
    }
    if v.attr("title")
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
    {
        return true;
    }
    let text: String = el.text().collect();
    if !text.trim().is_empty() {
        return true;
    }
    // An <img alt="…"> descendant gives the button/link an accessible name.
    if let Some(sel) = Selector::parse("img[alt]").ok() {
        if el
            .select(&sel)
            .any(|img| !img.value().attr("alt").unwrap_or("").trim().is_empty())
        {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use scraper::Html;
    use sf_core::config::CrawlConfig;
    use sf_core::crawl::{ContentType, CrawlUrl};
    use sf_core::id::{CrawlId, CrawlUrlId};

    fn page() -> CrawlUrl {
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

    fn eval(html: &str) -> Vec<FilterKey> {
        let cfg = CrawlConfig::default();
        let parsed = Html::parse_document(html);
        let ctx = EvalContext {
            config: &cfg,
            html: Some(html),
            parsed: Some(&parsed),
        };
        AccessibilityEvaluator
            .evaluate(&page(), &ctx)
            .into_iter()
            .map(|f| f.filter_key)
            .collect()
    }

    #[test]
    fn missing_lang_flagged() {
        let keys = eval("<html><head><title>t</title></head><body><h1>x</h1></body></html>");
        assert!(keys.contains(&FilterKey::AccessibilityRuleHtmlHasLang));
    }

    #[test]
    fn invalid_lang_flagged() {
        let keys = eval(
            r#"<html lang="notalangtag!"><head><title>t</title></head><body><h1>x</h1></body></html>"#,
        );
        assert!(keys.contains(&FilterKey::AccessibilityRuleHtmlLangValid));
    }

    #[test]
    fn valid_lang_not_flagged() {
        let keys = eval(
            r#"<html lang="en-US"><head><title>t</title></head><body><h1>x</h1></body></html>"#,
        );
        assert!(!keys.contains(&FilterKey::AccessibilityRuleHtmlLangValid));
        assert!(!keys.contains(&FilterKey::AccessibilityRuleHtmlHasLang));
    }

    #[test]
    fn missing_title_flagged() {
        let keys = eval(r#"<html lang="en"><head></head><body><h1>x</h1></body></html>"#);
        assert!(keys.contains(&FilterKey::AccessibilityRuleDocumentTitle));
    }

    #[test]
    fn missing_h1_flagged() {
        let keys = eval(r#"<html lang="en"><head><title>t</title></head><body><p>x</p></body></html>"#);
        assert!(keys.contains(&FilterKey::AccessibilityRulePageHasHeadingOne));
    }

    #[test]
    fn meta_refresh_flagged() {
        let keys = eval(
            r#"<html lang="en"><head><title>t</title><meta http-equiv="refresh" content="5"></head><body><h1>x</h1></body></html>"#,
        );
        assert!(keys.contains(&FilterKey::AccessibilityRuleMetaRefresh));
    }

    #[test]
    fn image_without_alt_flagged() {
        let keys = eval(
            r#"<html lang="en"><head><title>t</title></head><body><h1>x</h1><img src="/a.png"></body></html>"#,
        );
        assert!(keys.contains(&FilterKey::AccessibilityRuleImageAlt));
    }

    #[test]
    fn image_with_empty_alt_not_flagged() {
        let keys = eval(
            r#"<html lang="en"><head><title>t</title></head><body><h1>x</h1><img src="/a.png" alt=""></body></html>"#,
        );
        assert!(!keys.contains(&FilterKey::AccessibilityRuleImageAlt));
    }

    #[test]
    fn iframe_without_title_flagged() {
        let keys = eval(
            r#"<html lang="en"><head><title>t</title></head><body><h1>x</h1><iframe src="/f"></iframe></body></html>"#,
        );
        assert!(keys.contains(&FilterKey::AccessibilityRuleFrameTitle));
    }

    #[test]
    fn unnamed_button_flagged() {
        let keys = eval(
            r#"<html lang="en"><head><title>t</title></head><body><h1>x</h1><button></button></body></html>"#,
        );
        assert!(keys.contains(&FilterKey::AccessibilityRuleButtonName));
    }

    #[test]
    fn empty_link_flagged() {
        let keys = eval(
            r#"<html lang="en"><head><title>t</title></head><body><h1>x</h1><a href="/p"></a></body></html>"#,
        );
        assert!(keys.contains(&FilterKey::AccessibilityRuleLinkName));
    }

    #[test]
    fn link_with_aria_label_not_flagged() {
        let keys = eval(
            r#"<html lang="en"><head><title>t</title></head><body><h1>x</h1><a href="/p" aria-label="go"></a></body></html>"#,
        );
        assert!(!keys.contains(&FilterKey::AccessibilityRuleLinkName));
    }

    #[test]
    fn input_image_without_alt_flagged() {
        let keys = eval(
            r#"<html lang="en"><head><title>t</title></head><body><h1>x</h1><input type="image" src="/b.png"></body></html>"#,
        );
        assert!(keys.contains(&FilterKey::AccessibilityRuleInputImageAlt));
    }

    #[test]
    fn blink_flagged() {
        let keys = eval(
            r#"<html lang="en"><head><title>t</title></head><body><h1>x</h1><blink>hi</blink></body></html>"#,
        );
        assert!(keys.contains(&FilterKey::AccessibilityRuleBlink));
    }

    #[test]
    fn marquee_flagged() {
        let keys = eval(
            r#"<html lang="en"><head><title>t</title></head><body><h1>x</h1><marquee>hi</marquee></body></html>"#,
        );
        assert!(keys.contains(&FilterKey::AccessibilityRuleMarquee));
    }

    #[test]
    fn clean_page_only_fires_all() {
        let keys = eval(
            r#"<html lang="en"><head><title>t</title></head><body><h1>hi</h1><p>clean</p></body></html>"#,
        );
        assert_eq!(keys, vec![FilterKey::AccessibilityAll]);
    }
}
