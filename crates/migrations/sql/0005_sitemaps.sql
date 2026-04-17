-- Batch E: sitemap.xml capture.
--
-- `crawls.sitemap_xml_raw` holds the literal XML body we fetched at crawl
-- start (or NULL if the site had no sitemap). `sitemap_xml_status` records
-- the HTTP status so the UI can distinguish a 200 from a 404 from a
-- network error (NULL).
--
-- `crawl_sitemap_urls` is the persisted URL set extracted from the
-- sitemap(s). SF's "URLs in Sitemap" filter, "Orphan URLs" filter, and the
-- post-crawl coverage rollup all read from this table. Keyed by (crawl_id,
-- url) so a URL appearing in multiple child sitemaps collapses to one row.

ALTER TABLE crawls
    ADD COLUMN IF NOT EXISTS sitemap_xml_raw    TEXT,
    ADD COLUMN IF NOT EXISTS sitemap_xml_status SMALLINT;

CREATE TABLE IF NOT EXISTS crawl_sitemap_urls (
    crawl_id  UUID NOT NULL REFERENCES crawls(id) ON DELETE CASCADE,
    url       TEXT NOT NULL,
    PRIMARY KEY (crawl_id, url)
);

CREATE INDEX IF NOT EXISTS idx_crawl_sitemap_urls_crawl
    ON crawl_sitemap_urls (crawl_id);
