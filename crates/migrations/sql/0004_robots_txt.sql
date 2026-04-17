-- Batch D: robots.txt capture + enforcement.
--
-- `crawls.robots_txt_raw` holds the literal robots.txt body we fetched
-- at crawl start (or NULL if the site had no robots file). This backs
-- the new GET /v1/crawls/:id/robots endpoint so the UI can show the
-- directives that governed the crawl.
--
-- `crawls.robots_txt_status` records the HTTP status returned by the
-- robots fetch so the UI can distinguish "served a 200" from "site has
-- no robots.txt" (404) from "fetch errored" (NULL).
--
-- `crawl_urls.blocked_by_robots` lights up SF's Response Codes →
-- "Blocked by Robots.txt" filter. True means we *discovered* the URL
-- but the robots matcher refused it, so we never fetched.

ALTER TABLE crawls
    ADD COLUMN IF NOT EXISTS robots_txt_raw    TEXT,
    ADD COLUMN IF NOT EXISTS robots_txt_status SMALLINT;

ALTER TABLE crawl_urls
    ADD COLUMN IF NOT EXISTS blocked_by_robots BOOLEAN NOT NULL DEFAULT FALSE;

CREATE INDEX IF NOT EXISTS idx_crawl_urls_blocked_by_robots
    ON crawl_urls (crawl_id)
    WHERE blocked_by_robots = TRUE;
