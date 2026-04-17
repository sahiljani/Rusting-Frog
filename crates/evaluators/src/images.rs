//! Images tab evaluator.
//!
//! Screaming Frog's Images tab surfaces image URLs and page-level image
//! problems together. We split the per-URL evaluation accordingly:
//!
//! - On image URLs (`content_type == Image`): emit `ImagesAll` and
//!   `ImagesOverXKb` when `content_length > max_size_kb`.
//! - On HTML pages: inspect `<img>` tags + CSS background refs and emit
//!   `ImagesMissingAltText`, `ImagesMissingAltAttribute`,
//!   `ImagesAltTextOverXCharacters`, `ImagesMissingSizeAttributes`,
//!   `ImagesBackgroundImages` when the page contains an offending image.
//!
//! Skipped: `ImagesIncorrectlySizedImages` needs rendered vs intrinsic
//! dimensions from CDP; not available in the online pass.

use scraper::{ElementRef, Selector};
use sf_core::crawl::{ContentType, CrawlUrl};
use sf_core::filter_key::FilterKey;
use sf_core::tab::TabKey;

use crate::{EvalContext, Evaluator, Finding};

pub struct ImagesEvaluator;

impl Evaluator for ImagesEvaluator {
    fn tab(&self) -> TabKey {
        TabKey::Images
    }

    fn evaluate(&self, url: &CrawlUrl, ctx: &EvalContext) -> Vec<Finding> {
        match url.content_type {
            ContentType::Image => evaluate_image_url(url, ctx),
            ContentType::Html => evaluate_html_page(url, ctx),
            _ => vec![],
        }
    }
}

fn evaluate_image_url(url: &CrawlUrl, ctx: &EvalContext) -> Vec<Finding> {
    let mut findings = vec![Finding {
        filter_key: FilterKey::ImagesAll,
    }];

    let max_bytes = (ctx.config.images.max_size_kb as i64) * 1024;
    if let Some(len) = url.content_length {
        if len > max_bytes {
            findings.push(Finding {
                filter_key: FilterKey::ImagesOverXKb,
            });
        }
    }

    findings
}

fn evaluate_html_page(_url: &CrawlUrl, ctx: &EvalContext) -> Vec<Finding> {
    let parsed = match ctx.parsed {
        Some(p) => p,
        None => return vec![],
    };

    let imgs: Vec<ElementRef<'_>> = match Selector::parse("img") {
        Ok(sel) => parsed.select(&sel).collect(),
        Err(_) => vec![],
    };

    let mut findings = Vec::new();

    let max_alt = ctx.config.images.max_alt_length as usize;

    let mut saw_missing_alt_attr = false;
    let mut saw_empty_alt = false;
    let mut saw_over_length_alt = false;
    let mut saw_missing_size_attrs = false;

    for img in &imgs {
        let alt = img.value().attr("alt");
        match alt {
            None => saw_missing_alt_attr = true,
            Some(txt) if txt.trim().is_empty() => saw_empty_alt = true,
            Some(txt) if txt.chars().count() > max_alt => saw_over_length_alt = true,
            _ => {}
        }

        let has_w = img.value().attr("width").is_some();
        let has_h = img.value().attr("height").is_some();
        if !(has_w && has_h) {
            saw_missing_size_attrs = true;
        }
    }

    if saw_missing_alt_attr {
        findings.push(Finding {
            filter_key: FilterKey::ImagesMissingAltAttribute,
        });
    }
    if saw_empty_alt {
        findings.push(Finding {
            filter_key: FilterKey::ImagesMissingAltText,
        });
    }
    if saw_over_length_alt {
        findings.push(Finding {
            filter_key: FilterKey::ImagesAltTextOverXCharacters,
        });
    }
    if !imgs.is_empty() && saw_missing_size_attrs {
        findings.push(Finding {
            filter_key: FilterKey::ImagesMissingSizeAttributes,
        });
    }

    if html_has_background_image(ctx.html.unwrap_or("")) {
        findings.push(Finding {
            filter_key: FilterKey::ImagesBackgroundImages,
        });
    }

    findings
}

/// Detects CSS `background-image: url(...)` in inline styles or `<style>`
/// blocks. We don't fetch/parse external stylesheets — that's a render-pass
/// concern.
fn html_has_background_image(html: &str) -> bool {
    let lower = html.to_ascii_lowercase();
    lower.contains("background-image") || lower.contains("background:") && lower.contains("url(")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use scraper::Html;
    use sf_core::config::CrawlConfig;
    use sf_core::crawl::{ContentType, CrawlUrl};
    use sf_core::id::{CrawlId, CrawlUrlId};

    fn html_url() -> CrawlUrl {
        CrawlUrl {
            id: CrawlUrlId::new(),
            crawl_id: CrawlId::new(),
            url: "https://example.com/page".to_string(),
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

    fn image_url(bytes: Option<i64>) -> CrawlUrl {
        let mut u = html_url();
        u.content_type = ContentType::Image;
        u.url = "https://example.com/img.png".to_string();
        u.content_length = bytes;
        u
    }

    fn eval(url: &CrawlUrl, html: &str) -> Vec<FilterKey> {
        let cfg = CrawlConfig::default();
        let parsed = Html::parse_document(html);
        let ctx = EvalContext {
            config: &cfg,
            html: Some(html),
            parsed: Some(&parsed),
        };
        ImagesEvaluator
            .evaluate(url, &ctx)
            .into_iter()
            .map(|f| f.filter_key)
            .collect()
    }

    #[test]
    fn image_url_always_emits_all() {
        let keys = eval(&image_url(Some(1000)), "");
        assert!(keys.contains(&FilterKey::ImagesAll));
    }

    #[test]
    fn small_image_does_not_trip_over_x_kb() {
        let keys = eval(&image_url(Some(50 * 1024)), "");
        assert!(!keys.contains(&FilterKey::ImagesOverXKb));
    }

    #[test]
    fn image_over_threshold_flagged() {
        let keys = eval(&image_url(Some(200 * 1024)), "");
        assert!(keys.contains(&FilterKey::ImagesOverXKb));
    }

    #[test]
    fn html_page_with_img_missing_alt_attr() {
        let keys = eval(&html_url(), r#"<html><body><img src="/a.png"></body></html>"#);
        assert!(keys.contains(&FilterKey::ImagesMissingAltAttribute));
    }

    #[test]
    fn html_page_with_empty_alt_flagged() {
        let keys = eval(
            &html_url(),
            r#"<html><body><img src="/a.png" alt=""></body></html>"#,
        );
        assert!(keys.contains(&FilterKey::ImagesMissingAltText));
    }

    #[test]
    fn long_alt_over_max_flagged() {
        let alt = "x".repeat(200);
        let html = format!(r#"<html><body><img src="/a.png" alt="{alt}" width="10" height="10"></body></html>"#);
        let keys = eval(&html_url(), &html);
        assert!(keys.contains(&FilterKey::ImagesAltTextOverXCharacters));
    }

    #[test]
    fn missing_width_or_height_flagged() {
        let html = r#"<html><body><img src="/a.png" alt="ok"></body></html>"#;
        let keys = eval(&html_url(), html);
        assert!(keys.contains(&FilterKey::ImagesMissingSizeAttributes));
    }

    #[test]
    fn sized_img_does_not_trip_missing_size() {
        let html = r#"<html><body><img src="/a.png" alt="ok" width="10" height="10"></body></html>"#;
        let keys = eval(&html_url(), html);
        assert!(!keys.contains(&FilterKey::ImagesMissingSizeAttributes));
    }

    #[test]
    fn background_image_inline_style_flagged() {
        let html = r#"<html><body><div style="background-image:url('/x.png')"></div></body></html>"#;
        let keys = eval(&html_url(), html);
        assert!(keys.contains(&FilterKey::ImagesBackgroundImages));
    }

    #[test]
    fn empty_html_produces_no_img_findings() {
        let keys = eval(&html_url(), "<html><body></body></html>");
        assert!(!keys.contains(&FilterKey::ImagesMissingAltAttribute));
        assert!(!keys.contains(&FilterKey::ImagesMissingSizeAttributes));
    }
}
