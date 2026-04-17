-- Batch C: persist raw HTML, a content hash (for SF's Duplicate Details
-- tab) and extracted JSON-LD blocks (for Structured Data tab).
--
-- Shape choices:
--   * `raw_html`  — TEXT, TOAST-compressed by Postgres. Nullable because
--     non-HTML responses (images, PDFs, binary) have nothing to store.
--   * `content_hash` — BYTEA(32) SHA-256 of a normalised form of the
--     body (lowercased, whitespace collapsed). Two URLs sharing a hash
--     are exact-duplicate pages. Indexable for cheap duplicate lookup.
--   * `structured_data` — JSONB array of {"type": "JSON-LD" | ...,
--     "data": <parsed>}. Microdata/RDFa left as a stub until we pick
--     a parser; for now only JSON-LD blocks populate.

ALTER TABLE crawl_urls
    ADD COLUMN IF NOT EXISTS raw_html        TEXT,
    ADD COLUMN IF NOT EXISTS content_hash    BYTEA,
    ADD COLUMN IF NOT EXISTS structured_data JSONB NOT NULL DEFAULT '[]';

-- Duplicate Details is always "find all other crawl_urls with this hash"
-- — so an index on (crawl_id, content_hash) lets us answer it in one
-- lookup. Partial on IS NOT NULL so rows without captured HTML don't
-- bloat the index.
CREATE INDEX IF NOT EXISTS idx_crawl_urls_content_hash
    ON crawl_urls (crawl_id, content_hash)
    WHERE content_hash IS NOT NULL;

-- Structured Data tab needs a "has any extracted items" filter so the
-- grid can highlight pages with JSON-LD. Cheap expression index.
CREATE INDEX IF NOT EXISTS idx_crawl_urls_has_structured_data
    ON crawl_urls (crawl_id)
    WHERE jsonb_array_length(structured_data) > 0;
