use std::collections::HashMap;

use anyhow::{Context, Result};
use scraper::Html;
use sf_core::config::CrawlConfig;
use sf_core::crawl::ContentType;
use sf_core::id::{CrawlId, CrawlUrlId};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use url::Url;
use uuid::Uuid;

use crate::fetcher::{FetchResult, Fetcher};
use crate::frontier::Frontier;
use crate::parser;
use crate::robots::RobotsGate;
use crate::sitemap::SitemapCapture;

pub struct CrawlPipeline {
    db: PgPool,
    crawl_id: CrawlId,
    tenant_id: String,
    config: CrawlConfig,
    fetcher: Fetcher,
    frontier: Frontier,
    seed_url: Url,
    seed_host: String,
    urls_crawled: i64,
}

impl CrawlPipeline {
    pub fn new(
        db: PgPool,
        crawl_id: CrawlId,
        tenant_id: String,
        seed_url: Url,
        config: CrawlConfig,
    ) -> Result<Self> {
        let seed_host = seed_url.host_str().unwrap_or_default().to_string();

        let fetcher = Fetcher::new(
            &config.user_agent.user_agent_string,
            config.limits.max_response_size_bytes,
        )?;

        let mut frontier = Frontier::new(config.limits.max_crawl_depth, config.limits.max_urls);
        frontier.add(seed_url.clone(), 0);

        Ok(Self {
            db,
            crawl_id,
            tenant_id,
            config,
            fetcher,
            frontier,
            seed_url,
            seed_host,
            urls_crawled: 0,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        self.set_status("running").await?;

        // Fetch robots.txt once per crawl and persist it alongside the
        // crawl row so (a) the UI's "Response Headers → Blocked by
        // Robots.txt" filter has a source of truth and (b) /v1/crawls/:id/robots
        // can return the exact body the matcher used.
        let gate = RobotsGate::fetch(
            self.fetcher.client(),
            &self.seed_url,
            &self.config.user_agent.user_agent_string,
        )
        .await;
        sqlx::query!(
            "UPDATE crawls SET robots_txt_raw = $1, robots_txt_status = $2 WHERE id = $3",
            gate.raw(),
            gate.status(),
            self.crawl_id.as_uuid(),
        )
        .execute(&self.db)
        .await?;

        // Fetch sitemap.xml (or sitemap index) and persist both the raw body
        // + every <loc> URL into crawl_sitemap_urls. SF's "URLs in Sitemap"
        // and "Orphan URLs" filters read from this set — we don't gate the
        // crawl on it (unlike robots), just record coverage.
        let sitemap = SitemapCapture::fetch(self.fetcher.client(), &self.seed_url).await;
        sqlx::query!(
            "UPDATE crawls SET sitemap_xml_raw = $1, sitemap_xml_status = $2 WHERE id = $3",
            sitemap.raw.as_deref(),
            sitemap.status,
            self.crawl_id.as_uuid(),
        )
        .execute(&self.db)
        .await?;
        for sm_url in &sitemap.urls {
            sqlx::query!(
                r#"
                INSERT INTO crawl_sitemap_urls (crawl_id, url)
                VALUES ($1, $2)
                ON CONFLICT (crawl_id, url) DO NOTHING
                "#,
                self.crawl_id.as_uuid(),
                sm_url,
            )
            .execute(&self.db)
            .await
            .ok();
        }
        tracing::info!(
            sitemap_status = ?sitemap.status,
            sitemap_url_count = sitemap.urls.len(),
            "sitemap captured"
        );

        let evaluators = sf_evaluators::phase1_evaluators();
        let mut status_check_countdown = 0_u32;

        while let Some(entry) = self.frontier.next() {
            // External stop signal: the /v1/crawls/:id/stop endpoint
            // flips crawls.status to 'completed' but has no IPC channel
            // to us. Poll the DB once every 5 fetches — cheap, and means
            // stop takes effect within ~5s at default rate limits. Break
            // out so the post-loop analysis + set_status still runs.
            if status_check_countdown == 0 {
                if let Ok(row) = sqlx::query!(
                    "SELECT status FROM crawls WHERE id = $1",
                    self.crawl_id.as_uuid(),
                )
                .fetch_one(&self.db)
                .await
                {
                    if matches!(row.status.as_str(), "completed" | "failed" | "cancelled") {
                        tracing::info!(
                            crawl_id = %self.crawl_id,
                            status = %row.status,
                            "external stop signal received — ending fetch loop",
                        );
                        break;
                    }
                }
                status_check_countdown = 5;
            } else {
                status_check_countdown -= 1;
            }

            // Robots gate: if Disallow matches, we still record the URL so
            // the "Blocked by Robots.txt" filter has a row to surface, but we
            // don't fetch it. This mirrors Screaming Frog's "discover but
            // don't crawl" behaviour for disallowed URLs.
            if !gate.is_allowed(&entry.url) {
                tracing::info!(url = %entry.url, "blocked by robots.txt");
                self.write_blocked_url(&entry.url.to_string(), entry.depth)
                    .await?;
                self.urls_crawled += 1;
                self.update_counters().await?;
                continue;
            }
            let url_str = entry.url.to_string();
            tracing::info!(url = %url_str, depth = entry.depth, "fetching");

            // Fetch the URL
            let fetch_result = match self.fetcher.fetch(&entry.url).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!(url = %url_str, error = %e, "fetch failed");
                    self.write_failed_url(&url_str, entry.depth).await?;
                    continue;
                }
            };

            let content_type = ContentType::from_mime(&fetch_result.content_type);
            let is_internal = self.is_internal(&entry.url);

            // Parse HTML if applicable
            let parse_result = if content_type == ContentType::Html && !fetch_result.body.is_empty()
            {
                Some(parser::parse_html(&fetch_result.body, &entry.url))
            } else {
                None
            };

            // Write the crawl_url row
            let url_id = CrawlUrlId::new();
            let url_hash = format!("{:x}", md5::compute(url_str.as_bytes()));

            self.write_crawl_url(
                &url_id,
                &url_str,
                &url_hash,
                &content_type,
                &fetch_result,
                is_internal,
                entry.depth as i32,
                &parse_result,
            )
            .await?;

            // Run evaluators and write findings
            if let Some(ref pr) = parse_result {
                let crawl_url = sf_core::crawl::CrawlUrl {
                    id: url_id,
                    crawl_id: self.crawl_id,
                    url: url_str.clone(),
                    url_hash: url_hash.clone(),
                    content_type,
                    status_code: Some(fetch_result.status_code as i16),
                    is_internal,
                    depth: entry.depth as i32,
                    title: pr.title.clone(),
                    title_length: pr.title_length,
                    title_pixel_width: pr.title_pixel_width,
                    meta_description: pr.meta_description.clone(),
                    meta_description_length: pr.meta_description_length,
                    meta_description_pixel_width: pr.meta_description_pixel_width,
                    h1_first: pr.h1_first.clone(),
                    h1_count: pr.h1_count,
                    h2_first: pr.h2_first.clone(),
                    h2_count: pr.h2_count,
                    word_count: Some(pr.word_count),
                    response_time_ms: Some(fetch_result.response_time_ms as i64),
                    content_length: Some(fetch_result.content_length as i64),
                    redirect_url: None,
                    canonical_url: pr.canonical_url.clone(),
                    meta_robots: pr.meta_robots.clone(),
                    crawled_at: Some(chrono::Utc::now()),
                };

                let parsed_html = Html::parse_document(&fetch_result.body);
                let eval_ctx = sf_evaluators::EvalContext {
                    config: &self.config,
                    html: Some(&fetch_result.body),
                    parsed: Some(&parsed_html),
                };

                for evaluator in &evaluators {
                    let findings = evaluator.evaluate(&crawl_url, &eval_ctx);
                    for finding in findings {
                        self.write_finding(&url_id, &finding.filter_key).await?;
                    }
                }

                // Enqueue discovered links. We only follow links out of
                // *internal* pages — once we cross the host boundary the
                // crawl would be unbounded, so external pages become leaves.
                // External URLs still get enqueued (and later fetched) so
                // the External evaluator has rows to fire against, but none
                // of their outlinks are followed.
                if is_internal {
                    for link in &pr.links {
                        if let Ok(link_url) = Url::parse(&link.href) {
                            self.frontier.add(link_url, entry.depth + 1);
                        }
                    }
                }

                // Write link edges
                for link in &pr.links {
                    if let Ok(link_url) = Url::parse(&link.href) {
                        self.write_link_edge(
                            &url_id,
                            &link.href,
                            &link.anchor_text,
                            link.is_nofollow,
                        )
                        .await
                        .ok(); // best-effort, don't fail the crawl
                    }
                }

                // Write page resources. Unlike link edges these are stored
                // regardless of whether the target was crawled, so external
                // CDN scripts/stylesheets still appear in the Resources
                // detail tab.
                for link in &pr.links {
                    let rtype = match link.link_type {
                        parser::LinkType::Script => Some("script"),
                        parser::LinkType::Stylesheet => Some("stylesheet"),
                        parser::LinkType::Image => Some("image"),
                        _ => None,
                    };
                    if let Some(rtype) = rtype {
                        self.write_resource(&url_id, &link.href, rtype).await.ok();
                    }
                }
            } else {
                // Non-HTML: still run evaluators that don't need parsed HTML
                let crawl_url = sf_core::crawl::CrawlUrl {
                    id: url_id,
                    crawl_id: self.crawl_id,
                    url: url_str.clone(),
                    url_hash: url_hash.clone(),
                    content_type,
                    status_code: Some(fetch_result.status_code as i16),
                    is_internal,
                    depth: entry.depth as i32,
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
                    response_time_ms: Some(fetch_result.response_time_ms as i64),
                    content_length: Some(fetch_result.content_length as i64),
                    redirect_url: None,
                    canonical_url: None,
                    meta_robots: None,
                    crawled_at: Some(chrono::Utc::now()),
                };

                let eval_ctx = sf_evaluators::EvalContext {
                    config: &self.config,
                    html: None,
                    parsed: None,
                };

                for evaluator in &evaluators {
                    let findings = evaluator.evaluate(&crawl_url, &eval_ctx);
                    for finding in findings {
                        self.write_finding(&url_id, &finding.filter_key).await?;
                    }
                }
            }

            self.urls_crawled += 1;
            self.update_counters().await?;

            // Rate limiting: simple delay between fetches
            tokio::time::sleep(std::time::Duration::from_millis(
                1000 / self.config.speed.max_uri_per_second.max(1) as u64,
            ))
            .await;
        }

        self.run_duplicate_analysis().await?;

        self.set_status("completed").await?;
        tracing::info!(
            crawl_id = %self.crawl_id,
            urls_crawled = self.urls_crawled,
            "crawl completed"
        );

        Ok(())
    }

    // Post-crawl pass: SF groups URLs that share a normalised title,
    // meta description, H1 or raw-HTML content hash and tags every
    // member of a group with a Duplicate finding. We run this once at
    // finalize because the signal is cross-URL — a row doesn't know
    // it's a duplicate until the whole corpus is in.
    async fn run_duplicate_analysis(&self) -> Result<()> {
        let groups: &[(&str, &str)] = &[
            ("LOWER(TRIM(title))", "title_duplicate"),
            ("LOWER(TRIM(meta_description))", "meta_descripton_duplicate"),
            ("LOWER(TRIM(h1_first))", "h1_duplicate"),
            ("LOWER(TRIM(h2_first))", "h2_duplicate"),
            ("encode(content_hash, 'hex')", "content_duplicates"),
        ];

        let mut total = 0_i64;
        for (expr, filter_key) in groups {
            let q = format!(
                r#"
                INSERT INTO crawl_url_findings (crawl_id, crawl_url_id, filter_key)
                SELECT crawl_id, id, $2
                FROM crawl_urls u
                WHERE u.crawl_id = $1
                  AND u.is_internal = TRUE
                  AND ({expr}) IS NOT NULL
                  AND ({expr}) <> ''
                  AND ({expr}) IN (
                      SELECT {expr}
                      FROM crawl_urls
                      WHERE crawl_id = $1
                        AND is_internal = TRUE
                        AND ({expr}) IS NOT NULL
                        AND ({expr}) <> ''
                      GROUP BY {expr}
                      HAVING COUNT(*) >= 2
                  )
                "#,
                expr = expr
            );
            let n = sqlx::query(&q)
                .bind(self.crawl_id.as_uuid())
                .bind(*filter_key)
                .execute(&self.db)
                .await
                .with_context(|| format!("duplicate analysis for {filter_key}"))?
                .rows_affected() as i64;
            total += n;
            if n > 0 {
                tracing::info!(filter_key, rows = n, "duplicate findings emitted");
            }
        }

        tracing::info!(
            crawl_id = %self.crawl_id,
            total_duplicate_findings = total,
            "duplicate analysis pass complete"
        );

        Ok(())
    }

    fn is_internal(&self, url: &Url) -> bool {
        url.host_str()
            .map(|h| h.eq_ignore_ascii_case(&self.seed_host))
            .unwrap_or(false)
    }

    async fn set_status(&self, status: &str) -> Result<()> {
        let now = chrono::Utc::now();
        if status == "running" {
            sqlx::query!(
                "UPDATE crawls SET status = $1, started_at = $2 WHERE id = $3",
                status,
                now,
                self.crawl_id.as_uuid(),
            )
            .execute(&self.db)
            .await?;
        } else if status == "completed" || status == "failed" {
            sqlx::query!(
                "UPDATE crawls SET status = $1, completed_at = $2 WHERE id = $3",
                status,
                now,
                self.crawl_id.as_uuid(),
            )
            .execute(&self.db)
            .await?;
        } else {
            sqlx::query!(
                "UPDATE crawls SET status = $1 WHERE id = $2",
                status,
                self.crawl_id.as_uuid(),
            )
            .execute(&self.db)
            .await?;
        }
        Ok(())
    }

    async fn update_counters(&self) -> Result<()> {
        sqlx::query!(
            "UPDATE crawls SET urls_crawled = $1, urls_discovered = $2 WHERE id = $3",
            self.urls_crawled,
            self.frontier.discovered_count() as i64,
            self.crawl_id.as_uuid(),
        )
        .execute(&self.db)
        .await?;
        Ok(())
    }

    async fn write_crawl_url(
        &self,
        url_id: &CrawlUrlId,
        url: &str,
        url_hash: &str,
        content_type: &ContentType,
        fetch: &FetchResult,
        is_internal: bool,
        depth: i32,
        parse_result: &Option<parser::ParseResult>,
    ) -> Result<()> {
        let ct = format!("{:?}", content_type).to_lowercase();
        let now = chrono::Utc::now();

        let (
            title,
            title_len,
            title_px,
            meta_desc,
            meta_desc_len,
            meta_desc_px,
            h1,
            h1c,
            h2,
            h2c,
            wc,
            canonical,
            robots,
        ) = match parse_result {
            Some(pr) => (
                pr.title.as_deref(),
                pr.title_length,
                pr.title_pixel_width,
                pr.meta_description.as_deref(),
                pr.meta_description_length,
                pr.meta_description_pixel_width,
                pr.h1_first.as_deref(),
                pr.h1_count,
                pr.h2_first.as_deref(),
                pr.h2_count,
                Some(pr.word_count),
                pr.canonical_url.as_deref(),
                pr.meta_robots.as_deref(),
            ),
            None => (
                None, None, None, None, None, None, None, 0, None, 0, None, None, None,
            ),
        };

        // Persist response headers as a JSON array of [name, value] pairs so
        // downstream /headers + /cookies endpoints can read them without a
        // join. `final_url` captures the post-redirect URL so SF's HTTP
        // Headers detail pane can show the original-vs-final mapping.
        let headers_json: serde_json::Value = serde_json::Value::Array(
            fetch
                .headers
                .iter()
                .map(|(k, v)| serde_json::json!([k, v]))
                .collect(),
        );

        // Batch C storage: raw HTML, a normalised SHA-256 content hash (for
        // the Duplicate Details tab) and extracted JSON-LD blocks (for the
        // Structured Data tab). Only populated for HTML responses — binary
        // and redirect bodies leave these NULL / empty.
        let is_html = *content_type == ContentType::Html && !fetch.body.is_empty();
        let raw_html_opt: Option<&str> = if is_html {
            Some(fetch.body.as_str())
        } else {
            None
        };
        let content_hash_opt: Option<Vec<u8>> = if is_html {
            Some(sha256_normalised(&fetch.body))
        } else {
            None
        };
        let structured_data_json: serde_json::Value = match parse_result {
            Some(pr) if !pr.json_ld_blocks.is_empty() => serde_json::Value::Array(
                pr.json_ld_blocks
                    .iter()
                    .map(|v| serde_json::json!({ "type": "JSON-LD", "data": v }))
                    .collect(),
            ),
            _ => serde_json::Value::Array(vec![]),
        };

        let (indexability, indexability_status) = compute_indexability(
            fetch.status_code,
            content_type,
            url,
            &fetch.headers,
            robots,
            canonical,
        );

        sqlx::query!(
            r#"
            INSERT INTO crawl_urls (
                id, crawl_id, url, url_hash, content_type, status_code,
                is_internal, depth, title, title_length, title_pixel_width,
                meta_description, meta_description_length,
                meta_description_pixel_width, h1_first, h1_count,
                h2_first, h2_count, word_count, response_time_ms, content_length,
                canonical_url, meta_robots, response_headers, final_url,
                crawled_at, raw_html, content_hash, structured_data,
                indexability, indexability_status
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14,
                $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29,
                $30, $31
            )
            ON CONFLICT (crawl_id, url_hash) DO NOTHING
            "#,
            url_id.as_uuid(),
            self.crawl_id.as_uuid(),
            url,
            url_hash,
            ct,
            fetch.status_code as i16,
            is_internal,
            depth,
            title,
            title_len,
            title_px,
            meta_desc,
            meta_desc_len,
            meta_desc_px,
            h1,
            h1c,
            h2,
            h2c,
            wc,
            fetch.response_time_ms as i64,
            fetch.content_length as i64,
            canonical,
            robots,
            headers_json,
            fetch.final_url,
            now,
            raw_html_opt,
            content_hash_opt.as_deref(),
            structured_data_json,
            indexability,
            indexability_status,
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    async fn write_failed_url(&self, url: &str, depth: u32) -> Result<()> {
        let url_id = CrawlUrlId::new();
        let url_hash = format!("{:x}", md5::compute(url.as_bytes()));
        let now = chrono::Utc::now();

        sqlx::query!(
            r#"
            INSERT INTO crawl_urls (id, crawl_id, url, url_hash, content_type, is_internal, depth, crawled_at)
            VALUES ($1, $2, $3, $4, 'unknown', true, $5, $6)
            ON CONFLICT (crawl_id, url_hash) DO NOTHING
            "#,
            url_id.as_uuid(),
            self.crawl_id.as_uuid(),
            url,
            url_hash,
            depth as i32,
            now,
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    async fn write_blocked_url(&self, url: &str, depth: u32) -> Result<()> {
        let url_id = CrawlUrlId::new();
        let url_hash = format!("{:x}", md5::compute(url.as_bytes()));
        let now = chrono::Utc::now();
        let is_internal = Url::parse(url)
            .ok()
            .and_then(|u| {
                u.host_str()
                    .map(|h| h.eq_ignore_ascii_case(&self.seed_host))
            })
            .unwrap_or(false);

        sqlx::query!(
            r#"
            INSERT INTO crawl_urls (
                id, crawl_id, url, url_hash, content_type, is_internal, depth,
                crawled_at, blocked_by_robots, indexability, indexability_status
            )
            VALUES ($1, $2, $3, $4, 'unknown', $5, $6, $7, TRUE,
                    'Non-Indexable', 'Blocked by Robots.txt')
            ON CONFLICT (crawl_id, url_hash) DO UPDATE SET
                blocked_by_robots = TRUE,
                indexability = 'Non-Indexable',
                indexability_status = 'Blocked by Robots.txt'
            "#,
            url_id.as_uuid(),
            self.crawl_id.as_uuid(),
            url,
            url_hash,
            is_internal,
            depth as i32,
            now,
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    async fn write_finding(
        &self,
        url_id: &CrawlUrlId,
        filter_key: &sf_core::filter_key::FilterKey,
    ) -> Result<()> {
        let key_str = serde_json::to_value(filter_key)?
            .as_str()
            .unwrap_or_default()
            .to_string();

        sqlx::query!(
            r#"
            INSERT INTO crawl_url_findings (crawl_id, crawl_url_id, filter_key)
            VALUES ($1, $2, $3)
            "#,
            self.crawl_id.as_uuid(),
            url_id.as_uuid(),
            key_str,
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    async fn write_resource(
        &self,
        source_url_id: &CrawlUrlId,
        url: &str,
        resource_type: &str,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO crawl_url_resources (crawl_id, source_url_id, url, resource_type)
            VALUES ($1, $2, $3, $4)
            "#,
            self.crawl_id.as_uuid(),
            source_url_id.as_uuid(),
            url,
            resource_type,
        )
        .execute(&self.db)
        .await?;
        Ok(())
    }

    async fn write_link_edge(
        &self,
        source_url_id: &CrawlUrlId,
        target_url: &str,
        anchor_text: &str,
        is_nofollow: bool,
    ) -> Result<()> {
        // Look up target URL ID if it exists; skip if not yet crawled
        let target = sqlx::query_scalar!(
            "SELECT id FROM crawl_urls WHERE crawl_id = $1 AND url = $2",
            self.crawl_id.as_uuid(),
            target_url,
        )
        .fetch_optional(&self.db)
        .await?;

        if let Some(target_id) = target {
            sqlx::query!(
                r#"
                INSERT INTO crawl_links (crawl_id, source_url_id, target_url_id, anchor_text, is_nofollow)
                VALUES ($1, $2, $3, $4, $5)
                "#,
                self.crawl_id.as_uuid(),
                source_url_id.as_uuid(),
                target_id,
                anchor_text,
                is_nofollow,
            )
            .execute(&self.db)
            .await?;
        }

        Ok(())
    }
}

// Normalised SHA-256 for the Duplicate Details tab.
//
// Two pages that differ only in whitespace / casing / HTML comments are
// treated as byte-identical duplicates by SF. We mirror that by lowercasing,
// stripping comments, and collapsing runs of whitespace before hashing —
// this keeps the hash insensitive to cosmetic noise (build-time cachebusters,
// pretty-printer variance) while still being exact enough that a genuine
// content diff produces a different hash.
fn sha256_normalised(body: &str) -> Vec<u8> {
    let mut out = String::with_capacity(body.len());
    let bytes = body.as_bytes();
    let mut i = 0;
    let mut last_was_ws = false;
    while i < bytes.len() {
        if bytes[i..].starts_with(b"<!--") {
            if let Some(end) = body[i..].find("-->") {
                i += end + 3;
                continue;
            } else {
                break;
            }
        }
        let c = bytes[i] as char;
        if c.is_ascii_whitespace() {
            if !last_was_ws {
                out.push(' ');
                last_was_ws = true;
            }
        } else {
            out.push(c.to_ascii_lowercase());
            last_was_ws = false;
        }
        i += 1;
    }
    let mut hasher = Sha256::new();
    hasher.update(out.as_bytes());
    hasher.finalize().to_vec()
}

// Classifies a fetched URL as Indexable vs Non-Indexable and records the
// reason that knocked it out. Order matters: the first matching rule wins
// because that's how SF reports it (a redirected noindex page surfaces as
// "Redirected", not "Noindex"). Returns (indexability, status_reason).
fn compute_indexability(
    status_code: u16,
    content_type: &ContentType,
    url: &str,
    headers: &[(String, String)],
    meta_robots: Option<&str>,
    canonical_url: Option<&str>,
) -> (&'static str, &'static str) {
    // 3xx => Redirected (even if the final target is 200)
    if (300..400).contains(&status_code) {
        return ("Non-Indexable", "Redirected");
    }
    // 4xx/5xx => HTTP Error
    if status_code >= 400 {
        return ("Non-Indexable", "HTTP Error");
    }

    // X-Robots-Tag: noindex (case-insensitive header name, value may be a
    // CSV like "noindex, nofollow"). We only look for the "noindex" token.
    for (k, v) in headers {
        if k.eq_ignore_ascii_case("x-robots-tag") && v.to_ascii_lowercase().contains("noindex") {
            return ("Non-Indexable", "Blocked by X-Robots-Tag");
        }
    }

    if let Some(mr) = meta_robots {
        if mr.to_ascii_lowercase().contains("noindex") {
            return ("Non-Indexable", "Noindex");
        }
    }

    // Canonicalised: canonical points somewhere other than this URL. We
    // compare after trimming a trailing slash since SF treats "/foo" and
    // "/foo/" as the same URL.
    if let Some(can) = canonical_url {
        let norm_self = url.trim_end_matches('/');
        let norm_can = can.trim_end_matches('/');
        if !can.is_empty() && norm_can != norm_self {
            return ("Non-Indexable", "Canonicalised");
        }
    }

    // Non-HTML binary content is not considered indexable by SF.
    if !matches!(content_type, ContentType::Html) {
        return ("Non-Indexable", "Non-HTML");
    }

    ("Indexable", "Indexable")
}
