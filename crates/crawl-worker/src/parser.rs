use scraper::{Html, Selector};
use url::Url;

pub struct ParseResult {
    pub title: Option<String>,
    pub title_length: Option<i32>,
    pub meta_description: Option<String>,
    pub meta_description_length: Option<i32>,
    pub h1_first: Option<String>,
    pub h1_count: i32,
    pub h2_first: Option<String>,
    pub h2_count: i32,
    pub word_count: i32,
    pub canonical_url: Option<String>,
    pub meta_robots: Option<String>,
    pub links: Vec<ExtractedLink>,
}

pub struct ExtractedLink {
    pub href: String,
    pub anchor_text: String,
    pub is_nofollow: bool,
    pub link_type: LinkType,
}

pub enum LinkType {
    Anchor,
    Image,
    Script,
    Stylesheet,
    Canonical,
}

pub fn parse_html(html_str: &str, base_url: &Url) -> ParseResult {
    let document = Html::parse_document(html_str);

    let title = extract_first_text(&document, "title");
    let title_length = title.as_ref().map(|t| t.len() as i32);

    let meta_description = extract_meta_content(&document, "description");
    let meta_description_length = meta_description.as_ref().map(|d| d.len() as i32);

    let h1s = extract_all_text(&document, "h1");
    let h1_first = h1s.first().cloned();
    let h1_count = h1s.len() as i32;

    let h2s = extract_all_text(&document, "h2");
    let h2_first = h2s.first().cloned();
    let h2_count = h2s.len() as i32;

    let body_text = extract_first_text(&document, "body").unwrap_or_default();
    let word_count = body_text.split_whitespace().count() as i32;

    let canonical_url = extract_canonical(&document);
    let meta_robots = extract_meta_content(&document, "robots");
    let links = extract_links(&document, base_url);

    ParseResult {
        title,
        title_length,
        meta_description,
        meta_description_length,
        h1_first,
        h1_count,
        h2_first,
        h2_count,
        word_count,
        canonical_url,
        meta_robots,
        links,
    }
}

fn extract_first_text(doc: &Html, selector: &str) -> Option<String> {
    let sel = Selector::parse(selector).ok()?;
    doc.select(&sel)
        .next()
        .map(|el| el.text().collect::<String>().trim().to_string())
        .filter(|s| !s.is_empty())
}

fn extract_all_text(doc: &Html, selector: &str) -> Vec<String> {
    let sel = match Selector::parse(selector) {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    doc.select(&sel)
        .map(|el| el.text().collect::<String>().trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn extract_meta_content(doc: &Html, name: &str) -> Option<String> {
    let sel = Selector::parse(&format!("meta[name=\"{name}\"]")).ok()?;
    doc.select(&sel)
        .next()
        .and_then(|el| el.value().attr("content"))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn extract_canonical(doc: &Html) -> Option<String> {
    let sel = Selector::parse("link[rel=\"canonical\"]").ok()?;
    doc.select(&sel)
        .next()
        .and_then(|el| el.value().attr("href"))
        .map(|s| s.to_string())
}

fn extract_links(doc: &Html, base_url: &Url) -> Vec<ExtractedLink> {
    let mut links = Vec::new();

    if let Ok(sel) = Selector::parse("a[href]") {
        for el in doc.select(&sel) {
            let href = match el.value().attr("href") {
                Some(h) => h,
                None => continue,
            };

            let resolved = match base_url.join(href) {
                Ok(u) => u.to_string(),
                Err(_) => continue,
            };

            let anchor_text = el.text().collect::<String>().trim().to_string();
            let is_nofollow = el
                .value()
                .attr("rel")
                .map(|r| r.contains("nofollow"))
                .unwrap_or(false);

            links.push(ExtractedLink {
                href: resolved,
                anchor_text,
                is_nofollow,
                link_type: LinkType::Anchor,
            });
        }
    }

    links
}
