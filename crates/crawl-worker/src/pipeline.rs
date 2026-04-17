use std::collections::HashMap;

use anyhow::{Context, Result};
use scraper::Html;
use sf_core::config::CrawlConfig;
use sf_core::crawl::ContentType;
use sf_core::id::{CrawlId, CrawlUrlId};
use sqlx::PgPool;
use url::Url;
use uuid::Uuid;

use crate::fetcher::{FetchResult, Fetcher};
use crate::frontier::Frontier;
use crate::parser;

pub struct CrawlPipeline {
    db: PgPool,
    crawl_id: CrawlId,
    tenant_id: String,
    config: CrawlConfig,
    fetcher: Fetcher,
    frontier: Frontier,
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
        let seed_host = seed_url
            .host_str()
            .unwrap_or_default()
            .to_string();

        let fetcher = Fetcher::new(
            &config.user_agent.user_agent_string,
            config.limits.max_response_size_bytes,
        )?;

        let mut frontier = Frontier::new(
            config.limits.max_crawl_depth,
            config.limits.max_urls,
        );
        frontier.add(seed_url, 0);

        Ok(Self {
            db,
            crawl_id,
            tenant_id,
            config,
            fetcher,
            frontier,
            seed_host,
            urls_crawled: 0,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        self.set_status("running").await?;

        let evaluators = sf_evaluators::phase1_evaluators();

        while let Some(entry) = self.frontier.next() {
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
                    title_pixel_width: None,
                    meta_description: pr.meta_description.clone(),
                    meta_description_length: pr.meta_description_length,
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

        self.set_status("completed").await?;
        tracing::info!(
            crawl_id = %self.crawl_id,
            urls_crawled = self.urls_crawled,
            "crawl completed"
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

        let (title, title_len, meta_desc, meta_desc_len, h1, h1c, h2, h2c, wc, canonical, robots) =
            match parse_result {
                Some(pr) => (
                    pr.title.as_deref(),
                    pr.title_length,
                    pr.meta_description.as_deref(),
                    pr.meta_description_length,
                    pr.h1_first.as_deref(),
                    pr.h1_count,
                    pr.h2_first.as_deref(),
                    pr.h2_count,
                    Some(pr.word_count),
                    pr.canonical_url.as_deref(),
                    pr.meta_robots.as_deref(),
                ),
                None => (None, None, None, None, None, 0, None, 0, None, None, None),
            };

        sqlx::query!(
            r#"
            INSERT INTO crawl_urls (
                id, crawl_id, url, url_hash, content_type, status_code,
                is_internal, depth, title, title_length, meta_description,
                meta_description_length, h1_first, h1_count, h2_first, h2_count,
                word_count, response_time_ms, content_length, canonical_url,
                meta_robots, crawled_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14,
                $15, $16, $17, $18, $19, $20, $21, $22
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
            meta_desc,
            meta_desc_len,
            h1,
            h1c,
            h2,
            h2c,
            wc,
            fetch.response_time_ms as i64,
            fetch.content_length as i64,
            canonical,
            robots,
            now,
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

    async fn write_finding(&self, url_id: &CrawlUrlId, filter_key: &sf_core::filter_key::FilterKey) -> Result<()> {
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
