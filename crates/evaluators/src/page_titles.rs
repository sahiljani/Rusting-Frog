use scraper::Selector;
use sf_core::config::CrawlConfig;
use sf_core::crawl::{ContentType, CrawlUrl};
use sf_core::filter_key::FilterKey;
use sf_core::tab::TabKey;

use crate::{EvalContext, Evaluator, Finding};

pub struct PageTitleEvaluator;

impl Evaluator for PageTitleEvaluator {
    fn tab(&self) -> TabKey {
        TabKey::PageTitles
    }

    fn evaluate(&self, url: &CrawlUrl, ctx: &EvalContext) -> Vec<Finding> {
        if url.content_type != ContentType::Html {
            return vec![];
        }

        let mut findings = vec![Finding {
            filter_key: FilterKey::TitleAll,
        }];

        let parsed = match ctx.parsed {
            Some(p) => p,
            None => return findings,
        };

        let titles = extract_titles(parsed);

        if titles.is_empty() {
            findings.push(Finding {
                filter_key: FilterKey::TitleMissing,
            });
            return findings;
        }

        if titles.len() > 1 {
            findings.push(Finding {
                filter_key: FilterKey::TitleMultiple,
            });
        }

        let title = &titles[0];

        check_length(title, ctx.config, &mut findings);
        check_pixel_width(url, ctx.config, &mut findings);
        check_same_as_h1(title, url, &mut findings);
        check_outside_head(parsed, &mut findings);

        findings
    }
}

fn extract_titles(html: &scraper::Html) -> Vec<String> {
    let selector = Selector::parse("title").expect("valid selector");
    html.select(&selector)
        .map(|el| el.text().collect::<String>().trim().to_string())
        .filter(|t| !t.is_empty())
        .collect()
}

fn check_length(title: &str, config: &CrawlConfig, findings: &mut Vec<Finding>) {
    let len = title.len() as u32;
    if len > config.page_title.max_title_length {
        findings.push(Finding {
            filter_key: FilterKey::TitleOverXCharacters,
        });
    }
    if len < config.page_title.min_title_length {
        findings.push(Finding {
            filter_key: FilterKey::TitleBelowXCharacters,
        });
    }
}

fn check_pixel_width(url: &CrawlUrl, config: &CrawlConfig, findings: &mut Vec<Finding>) {
    if let Some(pw) = url.title_pixel_width {
        let pw = pw as u32;
        if pw > config.page_title.max_title_pixel_width {
            findings.push(Finding {
                filter_key: FilterKey::TitleOverXPixels,
            });
        }
        if pw < config.page_title.min_title_pixel_width {
            findings.push(Finding {
                filter_key: FilterKey::TitleBelowXPixels,
            });
        }
    }
}

fn check_same_as_h1(title: &str, url: &CrawlUrl, findings: &mut Vec<Finding>) {
    if let Some(ref h1) = url.h1_first {
        if title.eq_ignore_ascii_case(h1) {
            findings.push(Finding {
                filter_key: FilterKey::TitleSameAsH1,
            });
        }
    }
}

fn check_outside_head(html: &scraper::Html, findings: &mut Vec<Finding>) {
    let title_sel = Selector::parse("title").expect("valid selector");
    let head_sel = Selector::parse("head title").expect("valid selector");

    let total_titles = html.select(&title_sel).count();
    let head_titles = html.select(&head_sel).count();

    if total_titles > head_titles {
        findings.push(Finding {
            filter_key: FilterKey::TitleOutsideHead,
        });
    }
}
