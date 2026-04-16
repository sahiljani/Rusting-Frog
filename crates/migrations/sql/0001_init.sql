-- Phase 1 schema: projects, crawls, crawl_urls, crawl_url_findings, crawl_links
-- All tables carry tenant_id for multi-tenant isolation.

CREATE TABLE IF NOT EXISTS projects (
    id          UUID PRIMARY KEY,
    tenant_id   TEXT NOT NULL,
    name        TEXT NOT NULL,
    seed_url    TEXT NOT NULL,
    config      JSONB NOT NULL DEFAULT '{}',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_projects_tenant ON projects (tenant_id);

CREATE TABLE IF NOT EXISTS crawls (
    id              UUID PRIMARY KEY,
    project_id      UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    tenant_id       TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'queued',
    seed_urls       JSONB NOT NULL DEFAULT '[]',
    urls_discovered BIGINT NOT NULL DEFAULT 0,
    urls_crawled    BIGINT NOT NULL DEFAULT 0,
    started_at      TIMESTAMPTZ,
    completed_at    TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_crawls_project ON crawls (project_id);
CREATE INDEX IF NOT EXISTS idx_crawls_tenant ON crawls (tenant_id);
CREATE INDEX IF NOT EXISTS idx_crawls_status ON crawls (status);

CREATE TABLE IF NOT EXISTS crawl_urls (
    id                      UUID PRIMARY KEY,
    crawl_id                UUID NOT NULL REFERENCES crawls(id) ON DELETE CASCADE,
    url                     TEXT NOT NULL,
    url_hash                TEXT NOT NULL,
    content_type            TEXT NOT NULL DEFAULT 'unknown',
    status_code             SMALLINT,
    is_internal             BOOLEAN NOT NULL DEFAULT true,
    depth                   INTEGER NOT NULL DEFAULT 0,

    -- Extracted fields from HTML parsing
    title                   TEXT,
    title_length            INTEGER,
    title_pixel_width       INTEGER,
    meta_description        TEXT,
    meta_description_length INTEGER,
    h1_first                TEXT,
    h1_count                INTEGER NOT NULL DEFAULT 0,
    h2_first                TEXT,
    h2_count                INTEGER NOT NULL DEFAULT 0,
    word_count              INTEGER,
    response_time_ms        BIGINT,
    content_length          BIGINT,
    redirect_url            TEXT,
    canonical_url           TEXT,
    meta_robots             TEXT,

    crawled_at              TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_crawl_urls_crawl ON crawl_urls (crawl_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_crawl_urls_dedupe ON crawl_urls (crawl_id, url_hash);

-- Findings: one row per (url, filter_key) match.
-- This is the read-hot table — the grid queries join through it.
CREATE TABLE IF NOT EXISTS crawl_url_findings (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    crawl_id     UUID NOT NULL REFERENCES crawls(id) ON DELETE CASCADE,
    crawl_url_id UUID NOT NULL REFERENCES crawl_urls(id) ON DELETE CASCADE,
    filter_key   TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_findings_crawl_filter ON crawl_url_findings (crawl_id, filter_key);
CREATE INDEX IF NOT EXISTS idx_findings_url ON crawl_url_findings (crawl_url_id);

-- Links: edges between crawled URLs (inlinks/outlinks).
CREATE TABLE IF NOT EXISTS crawl_links (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    crawl_id       UUID NOT NULL REFERENCES crawls(id) ON DELETE CASCADE,
    source_url_id  UUID NOT NULL REFERENCES crawl_urls(id) ON DELETE CASCADE,
    target_url_id  UUID NOT NULL REFERENCES crawl_urls(id) ON DELETE CASCADE,
    anchor_text    TEXT,
    link_type      TEXT NOT NULL DEFAULT 'anchor',
    is_nofollow    BOOLEAN NOT NULL DEFAULT false
);

CREATE INDEX IF NOT EXISTS idx_links_crawl ON crawl_links (crawl_id);
CREATE INDEX IF NOT EXISTS idx_links_source ON crawl_links (source_url_id);
CREATE INDEX IF NOT EXISTS idx_links_target ON crawl_links (target_url_id);
