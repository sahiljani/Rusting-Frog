//! H1 and H2 tab evaluators.
//!
//! Shares helpers between the two tabs because the logic is nearly
//! identical: Missing, Multiple, OverXCharacters, NonSequential, plus
//! H1-only AltTextInH1.
//!
//! Skipped in this pass: H1Duplicate / H2Duplicate (cross-URL,
//! post-crawl analysis).

use scraper::{ElementRef, Selector};
use sf_core::crawl::{ContentType, CrawlUrl};
use sf_core::filter_key::FilterKey;
use sf_core::tab::TabKey;

use crate::{EvalContext, Evaluator, Finding};

pub struct H1Evaluator;

impl Evaluator for H1Evaluator {
    fn tab(&self) -> TabKey {
        TabKey::H1
    }

    fn evaluate(&self, url: &CrawlUrl, ctx: &EvalContext) -> Vec<Finding> {
        if url.content_type != ContentType::Html {
            return vec![];
        }

        let mut findings = vec![Finding {
            filter_key: FilterKey::H1All,
        }];

        let parsed = match ctx.parsed {
            Some(p) => p,
            None => return findings,
        };

        let h1s = collect_headings(parsed, "h1");

        if h1s.is_empty() {
            findings.push(Finding {
                filter_key: FilterKey::H1Missing,
            });
        } else {
            if h1s.len() > 1 {
                findings.push(Finding {
                    filter_key: FilterKey::H1Multiple,
                });
            }

            let longest = h1s
                .iter()
                .map(|h| h.text.chars().count())
                .max()
                .unwrap_or(0) as u32;
            if longest > ctx.config.headings.max_h1_length {
                findings.push(Finding {
                    filter_key: FilterKey::H1OverXCharacters,
                });
            }

            if h1s.iter().any(|h| h.has_only_alt_text) {
                findings.push(Finding {
                    filter_key: FilterKey::H1AltTextInH1,
                });
            }
        }

        if !is_heading_order_sequential(parsed) {
            findings.push(Finding {
                filter_key: FilterKey::H1NonSequential,
            });
        }

        findings
    }
}

pub struct H2Evaluator;

impl Evaluator for H2Evaluator {
    fn tab(&self) -> TabKey {
        TabKey::H2
    }

    fn evaluate(&self, url: &CrawlUrl, ctx: &EvalContext) -> Vec<Finding> {
        if url.content_type != ContentType::Html {
            return vec![];
        }

        let mut findings = vec![Finding {
            filter_key: FilterKey::H2All,
        }];

        let parsed = match ctx.parsed {
            Some(p) => p,
            None => return findings,
        };

        let h2s = collect_headings(parsed, "h2");

        if h2s.is_empty() {
            findings.push(Finding {
                filter_key: FilterKey::H2Missing,
            });
        } else {
            if h2s.len() > 1 {
                findings.push(Finding {
                    filter_key: FilterKey::H2Multiple,
                });
            }

            let longest = h2s
                .iter()
                .map(|h| h.text.chars().count())
                .max()
                .unwrap_or(0) as u32;
            if longest > ctx.config.headings.max_h2_length {
                findings.push(Finding {
                    filter_key: FilterKey::H2OverXCharacters,
                });
            }
        }

        if !is_heading_order_sequential(parsed) {
            findings.push(Finding {
                filter_key: FilterKey::H2NonSequential,
            });
        }

        findings
    }
}

struct Heading {
    text: String,
    has_only_alt_text: bool,
}

fn collect_headings(html: &scraper::Html, tag: &str) -> Vec<Heading> {
    let sel = match Selector::parse(tag) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    html.select(&sel)
        .map(|el| {
            let text = el.text().collect::<String>().trim().to_string();
            Heading {
                has_only_alt_text: text.is_empty() && has_img_with_alt(&el),
                text,
            }
        })
        .filter(|h| !h.text.is_empty() || h.has_only_alt_text)
        .collect()
}

fn has_img_with_alt(el: &ElementRef<'_>) -> bool {
    let sel = match Selector::parse("img[alt]") {
        Ok(s) => s,
        Err(_) => return false,
    };
    el.select(&sel).any(|img| {
        img.value()
            .attr("alt")
            .map(|a| !a.trim().is_empty())
            .unwrap_or(false)
    })
}

/// A page is non-sequential when an H(n) appears before any H(n-1) has
/// been seen on the page. E.g. `<h3>` before any `<h2>`, or `<h2>`
/// before any `<h1>`. This matches SF's filter semantics (both H1 and
/// H2 tabs flag the same structural issue on the page).
fn is_heading_order_sequential(html: &scraper::Html) -> bool {
    let sel = match Selector::parse("h1,h2,h3,h4,h5,h6") {
        Ok(s) => s,
        Err(_) => return true,
    };
    let mut seen = [false; 6];
    for el in html.select(&sel) {
        let name = el.value().name();
        let level = match name {
            "h1" => 1,
            "h2" => 2,
            "h3" => 3,
            "h4" => 4,
            "h5" => 5,
            "h6" => 6,
            _ => continue,
        };
        if level > 1 && !seen[level - 2] {
            return false;
        }
        seen[level - 1] = true;
    }
    true
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

    fn h1_findings(html: &str) -> Vec<FilterKey> {
        let cfg = CrawlConfig::default();
        let parsed = Html::parse_document(html);
        let ctx = EvalContext {
            config: &cfg,
            html: Some(html),
            parsed: Some(&parsed),
        };
        H1Evaluator
            .evaluate(&url_fixture(), &ctx)
            .into_iter()
            .map(|f| f.filter_key)
            .collect()
    }

    fn h2_findings(html: &str) -> Vec<FilterKey> {
        let cfg = CrawlConfig::default();
        let parsed = Html::parse_document(html);
        let ctx = EvalContext {
            config: &cfg,
            html: Some(html),
            parsed: Some(&parsed),
        };
        H2Evaluator
            .evaluate(&url_fixture(), &ctx)
            .into_iter()
            .map(|f| f.filter_key)
            .collect()
    }

    #[test]
    fn no_h1_flags_missing() {
        let keys = h1_findings("<html><body></body></html>");
        assert!(keys.contains(&FilterKey::H1Missing));
    }

    #[test]
    fn single_short_h1_is_clean() {
        let keys = h1_findings("<html><body><h1>Short heading</h1></body></html>");
        assert!(!keys.contains(&FilterKey::H1Missing));
        assert!(!keys.contains(&FilterKey::H1Multiple));
        assert!(!keys.contains(&FilterKey::H1OverXCharacters));
    }

    #[test]
    fn multiple_h1_flagged() {
        let keys = h1_findings("<html><body><h1>one</h1><h1>two</h1></body></html>");
        assert!(keys.contains(&FilterKey::H1Multiple));
    }

    #[test]
    fn long_h1_flagged_over_chars() {
        let long = "a".repeat(120);
        let html = format!("<html><body><h1>{long}</h1></body></html>");
        let keys = h1_findings(&html);
        assert!(keys.contains(&FilterKey::H1OverXCharacters));
    }

    #[test]
    fn img_alt_only_h1_flagged() {
        let html = r#"<html><body><h1><img src="/x.png" alt="Brand"></h1></body></html>"#;
        let keys = h1_findings(html);
        assert!(keys.contains(&FilterKey::H1AltTextInH1));
    }

    #[test]
    fn h3_before_h2_is_non_sequential() {
        let keys = h1_findings("<html><body><h1>Top</h1><h3>Skipped a level</h3></body></html>");
        assert!(keys.contains(&FilterKey::H1NonSequential));
    }

    #[test]
    fn normal_h1_h2_h3_order_is_sequential() {
        let keys = h1_findings("<html><body><h1>1</h1><h2>2</h2><h3>3</h3></body></html>");
        assert!(!keys.contains(&FilterKey::H1NonSequential));
    }

    #[test]
    fn h2_missing_on_h1_only_page() {
        let keys = h2_findings("<html><body><h1>Only top</h1></body></html>");
        assert!(keys.contains(&FilterKey::H2Missing));
    }

    #[test]
    fn two_h2s_flagged_multiple() {
        let keys = h2_findings("<html><body><h1>Top</h1><h2>a</h2><h2>b</h2></body></html>");
        assert!(keys.contains(&FilterKey::H2Multiple));
    }

    #[test]
    fn long_h2_flagged() {
        let long = "a".repeat(80);
        let html = format!("<html><body><h1>Top</h1><h2>{long}</h2></body></html>");
        let keys = h2_findings(&html);
        assert!(keys.contains(&FilterKey::H2OverXCharacters));
    }
}
