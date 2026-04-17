-- Batch F: per-URL static resources.
--
-- Every <script src>, <link rel=stylesheet href>, <img src> etc. on an
-- HTML page gets one row here keyed to the page that loaded it. Unlike
-- crawl_links this table does NOT require the target to be crawled — an
-- external CDN script still produces a row — which is what SF's
-- "Resources" detail tab expects (it lists every asset the page pulled
-- regardless of whether the crawler ever fetched it).
--
-- `resource_type` is one of 'script', 'stylesheet', 'image', 'font',
-- 'iframe', 'other'. Kept as free text so the worker can add new types
-- without a schema change.

CREATE TABLE IF NOT EXISTS crawl_url_resources (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    crawl_id        UUID NOT NULL REFERENCES crawls(id) ON DELETE CASCADE,
    source_url_id   UUID NOT NULL REFERENCES crawl_urls(id) ON DELETE CASCADE,
    url             TEXT NOT NULL,
    resource_type   TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_resources_source
    ON crawl_url_resources (source_url_id);

CREATE INDEX IF NOT EXISTS idx_resources_crawl_type
    ON crawl_url_resources (crawl_id, resource_type);
