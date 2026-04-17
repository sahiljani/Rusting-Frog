//! Content tab evaluator.
//!
//! Online-pass implementation of the subset of Content filters that need
//! only the current page's text:
//!
//! - `ContentAll` — every HTML page.
//! - `ContentLowContentPages` — body word count below
//!   `config.content.min_word_count` (SF default 200).
//! - `ContentLoremIpsumPlaceholder` — body text contains the string
//!   "lorem ipsum" (case-insensitive).
//! - `ContentReadabilityDifficult` / `ContentReadabilityVeryDifficult` —
//!   Flesch-Kincaid grade level exceeds configured thresholds
//!   (12.0 / 16.0 by default).
//!
//! Deferred (need cross-URL context or external integrations):
//! `ContentDuplicates`, `ContentNearDuplicates`, `ContentCosineSimilarity`,
//! `ContentLowRelevance`, `ContentLanguageErrorsMisspelt`,
//! `ContentLanguageErrorsGrammar`.

use scraper::Selector;
use sf_core::crawl::{ContentType, CrawlUrl};
use sf_core::filter_key::FilterKey;
use sf_core::tab::TabKey;

use crate::{EvalContext, Evaluator, Finding};

pub struct ContentEvaluator;

impl Evaluator for ContentEvaluator {
    fn tab(&self) -> TabKey {
        TabKey::Content
    }

    fn evaluate(&self, url: &CrawlUrl, ctx: &EvalContext) -> Vec<Finding> {
        if url.content_type != ContentType::Html {
            return vec![];
        }

        let mut findings = vec![Finding {
            filter_key: FilterKey::ContentAll,
        }];

        let body_text = extract_body_text(ctx);

        let word_count = body_text.split_whitespace().count() as u32;
        if word_count < ctx.config.content.min_word_count && word_count > 0 {
            findings.push(Finding {
                filter_key: FilterKey::ContentLowContentPages,
            });
        }

        if body_text.to_ascii_lowercase().contains("lorem ipsum") {
            findings.push(Finding {
                filter_key: FilterKey::ContentLoremIpsumPlaceholder,
            });
        }

        if let Some(grade) = flesch_kincaid_grade(&body_text) {
            if grade >= ctx.config.content.readability_very_difficult {
                findings.push(Finding {
                    filter_key: FilterKey::ContentReadabilityVeryDifficult,
                });
            } else if grade >= ctx.config.content.readability_difficult {
                findings.push(Finding {
                    filter_key: FilterKey::ContentReadabilityDifficult,
                });
            }
        }

        findings
    }
}

fn extract_body_text(ctx: &EvalContext) -> String {
    let Some(doc) = ctx.parsed else { return String::new() };
    let sel = match Selector::parse("body") {
        Ok(s) => s,
        Err(_) => return String::new(),
    };
    doc.select(&sel)
        .next()
        .map(|el| el.text().collect::<Vec<_>>().join(" "))
        .unwrap_or_default()
}

/// Flesch-Kincaid grade level. Returns None if the text is too short to
/// measure meaningfully (< 10 words or < 1 sentence).
///
/// Formula: 0.39 * (words/sentences) + 11.8 * (syllables/words) − 15.59.
fn flesch_kincaid_grade(text: &str) -> Option<f64> {
    let words: Vec<&str> = text
        .split_whitespace()
        .filter(|w| w.chars().any(|c| c.is_alphabetic()))
        .collect();
    if words.len() < 10 {
        return None;
    }

    let sentence_count = count_sentences(text);
    if sentence_count == 0 {
        return None;
    }
    let syllable_count: u32 = words.iter().map(|w| count_syllables(w)).sum();
    if syllable_count == 0 {
        return None;
    }

    let words_per_sentence = words.len() as f64 / sentence_count as f64;
    let syllables_per_word = syllable_count as f64 / words.len() as f64;

    Some(0.39 * words_per_sentence + 11.8 * syllables_per_word - 15.59)
}

fn count_sentences(text: &str) -> u32 {
    text.chars().filter(|c| matches!(c, '.' | '!' | '?')).count() as u32
}

fn count_syllables(word: &str) -> u32 {
    let w: String = word
        .chars()
        .filter(|c| c.is_alphabetic())
        .map(|c| c.to_ascii_lowercase())
        .collect();
    if w.is_empty() {
        return 0;
    }
    let chars: Vec<char> = w.chars().collect();
    let vowels = ['a', 'e', 'i', 'o', 'u', 'y'];
    let mut count = 0u32;
    let mut prev_vowel = false;
    for c in &chars {
        let is_vowel = vowels.contains(c);
        if is_vowel && !prev_vowel {
            count += 1;
        }
        prev_vowel = is_vowel;
    }
    // Silent trailing 'e'.
    if w.ends_with('e') && count > 1 {
        count -= 1;
    }
    count.max(1)
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
        ContentEvaluator
            .evaluate(&page(), &ctx)
            .into_iter()
            .map(|f| f.filter_key)
            .collect()
    }

    #[test]
    fn every_html_page_emits_all() {
        let keys = eval("<html><body>hi</body></html>");
        assert!(keys.contains(&FilterKey::ContentAll));
    }

    #[test]
    fn non_html_emits_nothing() {
        let cfg = CrawlConfig::default();
        let ctx = EvalContext {
            config: &cfg,
            html: None,
            parsed: None,
        };
        let mut u = page();
        u.content_type = ContentType::Image;
        let keys = ContentEvaluator.evaluate(&u, &ctx);
        assert!(keys.is_empty());
    }

    #[test]
    fn low_content_flagged() {
        let keys = eval("<html><body>only five tiny words here</body></html>");
        assert!(keys.contains(&FilterKey::ContentLowContentPages));
    }

    #[test]
    fn long_page_not_flagged_low_content() {
        let body = "word ".repeat(300);
        let html = format!("<html><body>{body}</body></html>");
        let keys = eval(&html);
        assert!(!keys.contains(&FilterKey::ContentLowContentPages));
    }

    #[test]
    fn lorem_ipsum_flagged() {
        let keys = eval(
            r#"<html><body>Lorem ipsum dolor sit amet, consectetur adipiscing elit.</body></html>"#,
        );
        assert!(keys.contains(&FilterKey::ContentLoremIpsumPlaceholder));
    }

    #[test]
    fn very_complex_prose_is_very_difficult() {
        // Long sentences with many polysyllabic words push FK grade > 16.
        let body = "Antidisestablishmentarianism characteristically institutionalises incomprehensibly unmanageable bureaucratic impediments, complicating contemporaneous legislative deliberations and obfuscating constitutional interpretations.".repeat(4);
        let html = format!("<html><body>{body}</body></html>");
        let keys = eval(&html);
        assert!(keys.contains(&FilterKey::ContentReadabilityVeryDifficult));
    }

    #[test]
    fn syllables_basic() {
        assert_eq!(count_syllables("hello"), 2);
        assert_eq!(count_syllables("the"), 1);
        assert_eq!(count_syllables("cat"), 1);
        assert_eq!(count_syllables("banana"), 3);
    }
}
