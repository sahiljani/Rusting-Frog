-- Batch H: meta description pixel width.
--
-- SF's "Meta Description → Over 990 Pixels" filter is the mirror of the
-- title's "Over 561 Pixels" rule: Google renders the description in
-- Arial 13px and clips at ~990px across three lines. We persist the
-- computed pixel width per URL so grid filters can do a cheap column
-- comparison instead of recomputing per query.

ALTER TABLE crawl_urls
    ADD COLUMN IF NOT EXISTS meta_description_pixel_width INTEGER;
