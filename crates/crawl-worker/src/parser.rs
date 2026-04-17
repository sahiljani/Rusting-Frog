use scraper::{Html, Selector};
use url::Url;

pub struct ParseResult {
    pub title: Option<String>,
    pub title_length: Option<i32>,
    // Rendered width of the page title in Google's SERP font (Arial 18px).
    // SF's "Page Titles → Over 561 Pixels" filter triggers off this value,
    // not the raw character count — a title of 60 short chars ("iiii…")
    // fits while a title of 55 wide chars ("MMMM…") overflows.
    pub title_pixel_width: Option<i32>,
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
    // Every <script type="application/ld+json"> block, parsed. Each entry
    // is the raw JSON value — a single object, or an array of objects, or
    // a @graph envelope — SF's Structured Data tab keeps them as-is and
    // flattens @graph at render time.
    pub json_ld_blocks: Vec<serde_json::Value>,
}

pub struct ExtractedLink {
    pub href: String,
    pub anchor_text: String,
    pub is_nofollow: bool,
    pub link_type: LinkType,
}

#[derive(Debug, Clone, Copy)]
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
    let title_length = title.as_ref().map(|t| t.chars().count() as i32);
    let title_pixel_width = title.as_ref().map(|t| arial_18px_width(t));

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
    let json_ld_blocks = extract_json_ld(&document);

    ParseResult {
        title,
        title_length,
        title_pixel_width,
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
        json_ld_blocks,
    }
}

fn extract_json_ld(doc: &Html) -> Vec<serde_json::Value> {
    let sel = match Selector::parse("script[type=\"application/ld+json\"]") {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    doc.select(&sel)
        .filter_map(|el| {
            let raw: String = el.text().collect();
            serde_json::from_str::<serde_json::Value>(raw.trim()).ok()
        })
        .collect()
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

    for (selector, attr, kind) in [
        ("img[src]", "src", LinkType::Image),
        ("script[src]", "src", LinkType::Script),
        ("link[rel=\"stylesheet\"][href]", "href", LinkType::Stylesheet),
    ] {
        let sel = match Selector::parse(selector) {
            Ok(s) => s,
            Err(_) => continue,
        };
        for el in doc.select(&sel) {
            let raw = match el.value().attr(attr) {
                Some(v) => v,
                None => continue,
            };
            let resolved = match base_url.join(raw) {
                Ok(u) => u.to_string(),
                Err(_) => continue,
            };
            // For <img> we stash the alt attribute in anchor_text so the
            // Images detail tab can surface it (and the "Missing Alt Text"
            // filter can fire). Scripts and stylesheets have no alt.
            let anchor_text = if matches!(kind, LinkType::Image) {
                el.value()
                    .attr("alt")
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default()
            } else {
                String::new()
            };
            links.push(ExtractedLink {
                href: resolved,
                anchor_text,
                is_nofollow: false,
                link_type: kind,
            });
        }
    }

    links
}

// Approximate rendered width (in CSS px) of a string in Arial Bold 18px,
// the font Google has historically used for desktop SERP titles. This is
// what SF's "Title Over X Pixels" filters are calibrated against.
//
// Values are per-glyph advance widths sampled from Arial Bold at 18px.
// Unknown glyphs fall back to the average of 10px. Precision is "good
// enough for SF-parity" — we're within ~2 px of the browser.
fn arial_18px_width(s: &str) -> i32 {
    let mut total: f32 = 0.0;
    for c in s.chars() {
        total += arial_bold_18_advance(c);
    }
    total.round() as i32
}

fn arial_bold_18_advance(c: char) -> f32 {
    match c {
        ' ' => 5.0,
        '!' => 6.0,
        '"' => 7.67,
        '#' => 10.0,
        '$' => 10.0,
        '%' => 16.0,
        '&' => 12.0,
        '\'' => 4.0,
        '(' | ')' => 6.0,
        '*' => 7.0,
        '+' => 10.5,
        ',' | '.' => 5.0,
        '-' => 6.0,
        '/' => 5.0,
        '0'..='9' => 10.0,
        ':' | ';' => 6.0,
        '<' | '>' | '=' => 10.5,
        '?' => 11.0,
        '@' => 18.0,
        'A' | 'V' => 12.0,
        'B' | 'D' | 'E' | 'H' | 'K' | 'N' | 'P' | 'R' | 'U' | 'X' | 'Y' => 12.0,
        'C' | 'G' | 'O' | 'Q' => 13.0,
        'F' => 11.0,
        'I' => 5.0,
        'J' => 9.0,
        'L' => 10.0,
        'M' | 'W' => 14.0,
        'S' => 12.0,
        'T' | 'Z' => 11.0,
        '[' | ']' => 6.0,
        '\\' => 5.0,
        '^' => 10.0,
        '_' => 10.0,
        '`' => 6.0,
        'a' | 'b' | 'd' | 'g' | 'h' | 'n' | 'o' | 'p' | 'q' | 'u' => 10.0,
        'c' | 'e' | 'k' | 's' | 'x' | 'y' | 'z' => 9.0,
        'f' | 't' => 6.0,
        'i' | 'l' => 4.0,
        'j' => 5.0,
        'm' | 'w' => 14.0,
        'r' => 7.0,
        'v' => 9.0,
        '{' | '}' => 7.0,
        '|' => 5.0,
        '~' => 10.5,
        _ => 10.0,
    }
}
