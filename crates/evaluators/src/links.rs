//! Links tab evaluator.
//!
//! Fires on HTML pages. Parses `<a href>` anchors and emits per-page Links
//! findings based on counts + anchor-text + href-target properties:
//!
//! - `LinksAll` — every HTML page.
//! - `LinksNoInternalOutlinks` — page has zero internal outbound anchors.
//! - `LinksNofollowInternalOutlinks` — page contains at least one internal
//!   link with `rel="nofollow"`.
//! - `LinksNoAnchorTextOutlinks` — page contains an anchor whose visible
//!   text is empty (after trim, no alt on any child `<img>`).
//! - `LinksNonDescriptiveAnchorTextOutlinks` — anchor text matches a known
//!   low-value set ("click here", "read more", …).
//! - `LinksHighExternalOutlinks` / `LinksHighInternalOutlinks` — counts
//!   exceed `links.high_{internal,external}_outlinks`.
//! - `LinksHighCrawlDepth` — `url.depth > links.max_crawl_depth`.
//! - `LinksLocalHostOutlinks` — page links to `localhost` / `127.0.0.1`.
//!
//! Cross-URL filters (`LinksFollowNofollowInlinks`,
//! `LinksInternalNofollowInlinksOnly`, `LinksNonIndexablePageInlinksOnly`)
//! are deferred to the post-crawl analysis pass — they need the full
//! inlink graph.

use scraper::Selector;
use sf_core::crawl::{ContentType, CrawlUrl};
use sf_core::filter_key::FilterKey;
use sf_core::tab::TabKey;
use url::Url;

use crate::{EvalContext, Evaluator, Finding};

pub struct LinksEvaluator;

const NON_DESCRIPTIVE: &[&str] = &[
    "click here",
    "here",
    "read more",
    "learn more",
    "more",
    "this",
    "link",
    "this link",
    "click",
];

impl Evaluator for LinksEvaluator {
    fn tab(&self) -> TabKey {
        TabKey::Links
    }

    fn evaluate(&self, url: &CrawlUrl, ctx: &EvalContext) -> Vec<Finding> {
        if url.content_type != ContentType::Html {
            return vec![];
        }

        let mut findings = vec![Finding {
            filter_key: FilterKey::LinksAll,
        }];

        if url.depth > ctx.config.links.max_crawl_depth as i32 {
            findings.push(Finding {
                filter_key: FilterKey::LinksHighCrawlDepth,
            });
        }

        let parsed = match ctx.parsed {
            Some(p) => p,
            None => return findings,
        };

        let base = match Url::parse(&url.url) {
            Ok(u) => u,
            Err(_) => return findings,
        };
        let base_host = base.host_str().map(|h| h.to_ascii_lowercase());

        let a_sel = match Selector::parse("a[href]") {
            Ok(s) => s,
            Err(_) => return findings,
        };

        let mut internal = 0usize;
        let mut external = 0usize;
        let mut saw_nofollow_internal = false;
        let mut saw_empty_anchor = false;
        let mut saw_non_descriptive = false;
        let mut saw_localhost = false;

        for a in parsed.select(&a_sel) {
            let href = match a.value().attr("href") {
                Some(h) => h,
                None => continue,
            };
            let abs = match base.join(href) {
                Ok(u) => u,
                Err(_) => continue,
            };
            let scheme = abs.scheme();
            if scheme != "http" && scheme != "https" {
                continue;
            }

            let host = abs.host_str().map(|h| h.to_ascii_lowercase());
            let is_internal_link = host.is_some() && host == base_host;
            if is_internal_link {
                internal += 1;
            } else if host.is_some() {
                external += 1;
            }

            // SF semantics: localhost/127.0.0.1 outlinks are only interesting
            // when they're foreign to the page's own host (i.e. a stray dev
            // URL left in production). Skip if the page itself is on localhost.
            if !is_internal_link
                && matches!(host.as_deref(), Some("localhost") | Some("127.0.0.1"))
            {
                saw_localhost = true;
            }

            let rel = a.value().attr("rel").unwrap_or("").to_ascii_lowercase();
            let is_nofollow = rel.split_whitespace().any(|t| t == "nofollow");
            if is_internal_link && is_nofollow {
                saw_nofollow_internal = true;
            }

            let text: String = a.text().collect::<String>().trim().to_string();
            let has_img_alt = a
                .select(&Selector::parse("img[alt]").unwrap())
                .any(|i| !i.value().attr("alt").unwrap_or("").trim().is_empty());
            if text.is_empty() && !has_img_alt {
                saw_empty_anchor = true;
            } else if !text.is_empty() {
                let norm = text.to_ascii_lowercase();
                let norm = norm.trim_end_matches(|c: char| c == '.' || c == '!' || c == '?');
                if NON_DESCRIPTIVE.contains(&norm) {
                    saw_non_descriptive = true;
                }
            }
        }

        if internal == 0 {
            findings.push(Finding {
                filter_key: FilterKey::LinksNoInternalOutlinks,
            });
        }
        if saw_nofollow_internal {
            findings.push(Finding {
                filter_key: FilterKey::LinksNofollowInternalOutlinks,
            });
        }
        if saw_empty_anchor {
            findings.push(Finding {
                filter_key: FilterKey::LinksNoAnchorTextOutlinks,
            });
        }
        if saw_non_descriptive {
            findings.push(Finding {
                filter_key: FilterKey::LinksNonDescriptiveAnchorTextOutlinks,
            });
        }
        if internal > ctx.config.links.high_internal_outlinks as usize {
            findings.push(Finding {
                filter_key: FilterKey::LinksHighInternalOutlinks,
            });
        }
        if external > ctx.config.links.high_external_outlinks as usize {
            findings.push(Finding {
                filter_key: FilterKey::LinksHighExternalOutlinks,
            });
        }
        if saw_localhost {
            findings.push(Finding {
                filter_key: FilterKey::LinksLocalHostOutlinks,
            });
        }

        findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use scraper::Html;
    use sf_core::config::CrawlConfig;
    use sf_core::crawl::{ContentType, CrawlUrl};
    use sf_core::id::{CrawlId, CrawlUrlId};

    fn html_url(depth: i32) -> CrawlUrl {
        CrawlUrl {
            id: CrawlUrlId::new(),
            crawl_id: CrawlId::new(),
            url: "https://example.com/page".to_string(),
            url_hash: "h".to_string(),
            content_type: ContentType::Html,
            status_code: Some(200),
            is_internal: true,
            depth,
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

    fn eval(url: &CrawlUrl, html: &str) -> Vec<FilterKey> {
        let cfg = CrawlConfig::default();
        let parsed = Html::parse_document(html);
        let ctx = EvalContext {
            config: &cfg,
            html: Some(html),
            parsed: Some(&parsed),
        };
        LinksEvaluator
            .evaluate(url, &ctx)
            .into_iter()
            .map(|f| f.filter_key)
            .collect()
    }

    #[test]
    fn every_html_page_emits_all() {
        let keys = eval(&html_url(0), "<html><body></body></html>");
        assert!(keys.contains(&FilterKey::LinksAll));
    }

    #[test]
    fn non_html_emits_nothing() {
        let mut u = html_url(0);
        u.content_type = ContentType::Image;
        let keys = eval(&u, "");
        assert!(keys.is_empty());
    }

    #[test]
    fn page_with_no_internal_links_flagged() {
        let html = r#"<html><body><a href="https://other.test/x">x</a></body></html>"#;
        let keys = eval(&html_url(0), html);
        assert!(keys.contains(&FilterKey::LinksNoInternalOutlinks));
    }

    #[test]
    fn page_with_internal_link_not_flagged_no_internal() {
        let html = r#"<html><body><a href="/next">next</a></body></html>"#;
        let keys = eval(&html_url(0), html);
        assert!(!keys.contains(&FilterKey::LinksNoInternalOutlinks));
    }

    #[test]
    fn internal_nofollow_flagged() {
        let html = r#"<html><body><a href="/a" rel="nofollow">a</a></body></html>"#;
        let keys = eval(&html_url(0), html);
        assert!(keys.contains(&FilterKey::LinksNofollowInternalOutlinks));
    }

    #[test]
    fn empty_anchor_text_flagged() {
        let html = r#"<html><body><a href="/a"></a></body></html>"#;
        let keys = eval(&html_url(0), html);
        assert!(keys.contains(&FilterKey::LinksNoAnchorTextOutlinks));
    }

    #[test]
    fn anchor_with_img_alt_is_not_empty() {
        let html = r#"<html><body><a href="/a"><img src="x.png" alt="ok"></a></body></html>"#;
        let keys = eval(&html_url(0), html);
        assert!(!keys.contains(&FilterKey::LinksNoAnchorTextOutlinks));
    }

    #[test]
    fn click_here_flagged_non_descriptive() {
        let html = r#"<html><body><a href="/a">click here</a></body></html>"#;
        let keys = eval(&html_url(0), html);
        assert!(keys.contains(&FilterKey::LinksNonDescriptiveAnchorTextOutlinks));
    }

    #[test]
    fn descriptive_anchor_not_flagged() {
        let html = r#"<html><body><a href="/a">Read the full postmortem</a></body></html>"#;
        let keys = eval(&html_url(0), html);
        assert!(!keys.contains(&FilterKey::LinksNonDescriptiveAnchorTextOutlinks));
    }

    #[test]
    fn high_crawl_depth_flagged() {
        let keys = eval(&html_url(5), "<html><body></body></html>");
        assert!(keys.contains(&FilterKey::LinksHighCrawlDepth));
    }

    #[test]
    fn localhost_outlink_flagged() {
        let html = r#"<html><body><a href="http://localhost:8080/x">x</a></body></html>"#;
        let keys = eval(&html_url(0), html);
        assert!(keys.contains(&FilterKey::LinksLocalHostOutlinks));
    }

    #[test]
    fn high_external_outlinks_flagged() {
        let mut body = String::from("<html><body>");
        for i in 0..15 {
            body.push_str(&format!(r#"<a href="https://ext{i}.test/">ext{i}</a>"#));
        }
        body.push_str("</body></html>");
        let keys = eval(&html_url(0), &body);
        assert!(keys.contains(&FilterKey::LinksHighExternalOutlinks));
    }
}
