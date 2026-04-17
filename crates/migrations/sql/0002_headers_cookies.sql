-- Batch B: persist HTTP response headers + final (post-redirect) URL
-- so the API can feed SF's HTTP Headers and Cookies detail tabs + the
-- http_headers_all / cookies_all reports.
--
-- Shape choice: a single JSONB array of two-element arrays
-- (`[["header-name", "value"], ...]`) rather than a separate table.
-- Reads are always "give me everything for one url" so a nested scan
-- is cheaper than a join; writes are append-only at crawl time.

ALTER TABLE crawl_urls
    ADD COLUMN IF NOT EXISTS response_headers JSONB NOT NULL DEFAULT '[]',
    ADD COLUMN IF NOT EXISTS final_url        TEXT;

-- For cookies_all / http_headers_all we need a cheap filter on "has any
-- headers at all" (some rows — e.g. failed fetches — have nothing to
-- report). An expression index on the array length keeps that cheap
-- without bloating the regular column index.
CREATE INDEX IF NOT EXISTS idx_crawl_urls_has_headers
    ON crawl_urls (crawl_id)
    WHERE jsonb_array_length(response_headers) > 0;
