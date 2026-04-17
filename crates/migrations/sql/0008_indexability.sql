-- Batch I: per-URL indexability classification.
--
-- Screaming Frog's main grid is dominated by the Indexability column —
-- "Indexable" vs "Non-Indexable" plus a human-readable reason. Rather
-- than recompute the reason on every grid request, we persist it at
-- write time using whichever signal fired first:
--
--     1. blocked_by_robots = TRUE            → "Blocked by Robots.txt"
--     2. status_code in (300..399)           → "Redirected"
--     3. status_code in (400..599)           → "HTTP Error"
--     4. X-Robots-Tag header contains noindex → "Blocked by X-Robots-Tag"
--     5. <meta name=robots content=noindex>   → "Noindex"
--     6. canonical_url present and != self   → "Canonicalised"
--     7. else                                 → "Indexable"
--
-- `indexability` is a two-value flag for the grid's filter bar; the
-- reason column is only shown in the Indexability detail pane.

ALTER TABLE crawl_urls
    ADD COLUMN IF NOT EXISTS indexability        TEXT,
    ADD COLUMN IF NOT EXISTS indexability_status TEXT;

CREATE INDEX IF NOT EXISTS idx_crawl_urls_indexability
    ON crawl_urls (crawl_id, indexability);
