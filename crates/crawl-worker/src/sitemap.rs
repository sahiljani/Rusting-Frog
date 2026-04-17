//! Fetch + parse sitemap.xml for a crawl.
//!
//! The output feeds (a) the API's `GET /v1/crawls/:id/sitemap` endpoint so
//! the UI can show raw XML + a URL list, and (b) the `crawl_sitemap_urls`
//! table which powers "URLs in Sitemap" and "Orphan URLs" filters.
//!
//! The parser handles two shapes per the sitemaps.org spec:
//!   - `<urlset>` — a flat list of `<url><loc>…</loc></url>` entries.
//!   - `<sitemapindex>` — a list of child sitemaps; we fetch each child
//!     (depth-1 only, no recursion) and union their URLs.
//!
//! Child-sitemap fetch errors are swallowed — we'd rather return the URLs
//! we did get than fail the whole crawl over a broken secondary sitemap.

use std::collections::HashSet;

use quick_xml::Reader;
use quick_xml::events::Event;
use url::Url;

pub struct SitemapCapture {
    pub raw: Option<String>,
    pub status: Option<i16>,
    pub urls: HashSet<String>,
}

impl SitemapCapture {
    pub async fn fetch(client: &reqwest::Client, seed: &Url) -> Self {
        let host = match seed.host_str() {
            Some(h) => h,
            None => {
                return Self {
                    raw: None,
                    status: None,
                    urls: HashSet::new(),
                };
            }
        };
        let sitemap_url = format!("{}://{}/sitemap.xml", seed.scheme(), host);

        // Explicit XML Accept header: some CDN rule sets (github.com, for
        // one) return 406 to clients that advertise `*/*` without at least
        // one of `application/xml` or `text/xml`. Passing both covers every
        // server variant we've hit in smoke tests.
        let (raw, status) = match client
            .get(&sitemap_url)
            .header("Accept", "application/xml, text/xml;q=0.9, */*;q=0.8")
            .send()
            .await
        {
            Ok(resp) => {
                let code = resp.status().as_u16() as i16;
                let body = resp.text().await.unwrap_or_default();
                if (200..300).contains(&code) && !body.trim().is_empty() {
                    (Some(body), Some(code))
                } else {
                    (None, Some(code))
                }
            }
            Err(_) => (None, None),
        };

        let mut urls = HashSet::new();
        if let Some(ref body) = raw {
            match parse(body) {
                ParseResult::UrlSet(set) => urls.extend(set),
                ParseResult::SitemapIndex(children) => {
                    // Fan out to each child sitemap. Bounded: we only read
                    // the top-level index here, never recurse into another
                    // index — that'd be a configuration error on the
                    // target site and not worth unbounded work.
                    for child_url in children {
                        if let Ok(resp) = client
                            .get(&child_url)
                            .header("Accept", "application/xml, text/xml;q=0.9, */*;q=0.8")
                            .send()
                            .await
                            && resp.status().is_success()
                            && let Ok(child_body) = resp.text().await
                            && let ParseResult::UrlSet(set) = parse(&child_body)
                        {
                            urls.extend(set);
                        }
                    }
                }
            }
        }

        Self { raw, status, urls }
    }
}

enum ParseResult {
    UrlSet(HashSet<String>),
    SitemapIndex(Vec<String>),
}

fn parse(body: &str) -> ParseResult {
    let mut reader = Reader::from_str(body);
    reader.config_mut().trim_text(true);

    let mut urls: HashSet<String> = HashSet::new();
    let mut indexes: Vec<String> = Vec::new();
    let mut is_index = false;
    let mut in_loc = false;
    let mut current = String::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = e.name();
                let local = name.as_ref();
                if local.ends_with(b"sitemapindex") {
                    is_index = true;
                }
                if local == b"loc" {
                    in_loc = true;
                    current.clear();
                }
            }
            Ok(Event::End(e)) => {
                if e.name().as_ref() == b"loc" && in_loc {
                    let trimmed = current.trim().to_string();
                    if !trimmed.is_empty() {
                        if is_index {
                            indexes.push(trimmed);
                        } else {
                            urls.insert(trimmed);
                        }
                    }
                    in_loc = false;
                }
            }
            Ok(Event::Text(e)) => {
                if in_loc
                    && let Ok(s) = e.unescape()
                {
                    current.push_str(&s);
                }
            }
            Ok(Event::CData(e)) => {
                if in_loc
                    && let Ok(s) = std::str::from_utf8(e.as_ref())
                {
                    current.push_str(s);
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    if is_index {
        ParseResult::SitemapIndex(indexes)
    } else {
        ParseResult::UrlSet(urls)
    }
}
