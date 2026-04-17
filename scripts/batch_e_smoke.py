"""
Batch E smoke: sitemap.xml capture + enforcement.

Exercises /v1/crawls/:id/sitemap end-to-end. cloudflare.com serves
/sitemap.xml publicly (github returns 406 for non-browser clients), so
the fetcher + parser get a real URL set to persist. Set $SF_CRAWL_ID to
reuse a completed crawl (faster iteration).
"""
import json
import os
import sys
import time
import urllib.request
import urllib.error

BASE = "http://localhost:3000"


def tok():
    t = os.environ.get("SF_TOKEN")
    if t:
        return t
    with open(os.path.expanduser("~/.sf_tok.txt")) as f:
        return f.read().strip()


def req(method, path, body=None, token=None):
    data = json.dumps(body).encode() if body is not None else None
    r = urllib.request.Request(
        BASE + path,
        data=data,
        method=method,
        headers={
            "Authorization": f"Bearer {token or tok()}",
            "Content-Type": "application/json",
        },
    )
    with urllib.request.urlopen(r, timeout=60) as resp:
        return json.loads(resp.read())


def main():
    t = tok()

    cid = os.environ.get("SF_CRAWL_ID")
    if cid:
        print(f"[1/4] reusing crawl {cid}")
        s = req("GET", f"/v1/crawls/{cid}", token=t)
        assert s["status"] == "completed", f"reused crawl not completed: {s}"
    else:
        print("[1/4] creating cloudflare.com crawl (serves sitemap.xml publicly)")
        proj = req("POST", "/v1/projects", {"name": "batch_e_smoke", "seed_url": "https://www.cloudflare.com/"}, t)
        pid = proj["id"]
        crawl = req("POST", f"/v1/projects/{pid}/crawls", {
            "seed_urls": ["https://www.cloudflare.com/"],
            "max_depth": 0,
            "max_urls": 1,
        }, t)
        cid = crawl["id"]
        print(f"      crawl={cid}")
        # depth=0 max_urls=1 — the crawl itself finishes fast; we just
        # need the pipeline to run through sitemap capture before the loop
        # exhausts.
        for i in range(180):
            s = req("GET", f"/v1/crawls/{cid}", token=t)
            if s["status"] == "completed":
                break
            if i >= 20 and s.get("urls_crawled", 0) >= 1:
                req("POST", f"/v1/crawls/{cid}/stop", {}, t)
                time.sleep(3)
                break
            time.sleep(1)
        s = req("GET", f"/v1/crawls/{cid}", token=t)
        assert s["status"] == "completed", f"crawl did not complete: {s}"
    print(f"      {s['urls_crawled']} urls crawled")

    print("[2/4] /sitemap envelope shape")
    r = req("GET", f"/v1/crawls/{cid}/sitemap", token=t)
    for k in ("url", "status", "raw", "url_count", "crawled_count", "orphan_count", "urls"):
        assert k in r, f"sitemap envelope missing {k}: {r}"
    print(f"      url={r['url']} status={r['status']} raw_len={len(r['raw'] or '')} "
          f"url_count={r['url_count']} crawled={r['crawled_count']} orphan={r['orphan_count']}")
    assert r["status"] == 200, f"cloudflare sitemap.xml should return 200, got {r['status']}"
    assert r["raw"], "sitemap raw body must be non-empty"

    print("[3/4] URL set populated + invariants")
    assert r["url_count"] > 0, "cloudflare's sitemap should produce URLs"
    assert r["orphan_count"] == r["url_count"] - r["crawled_count"], \
        "orphan_count must equal url_count - crawled_count"
    assert isinstance(r["urls"], list)
    assert len(r["urls"]) <= 100, "default limit is 100"
    print(f"      {r['url_count']} sitemap URLs persisted, {len(r['urls'])} returned (limit)")

    print("[4/4] pagination (?limit=10) honoured")
    r2 = req("GET", f"/v1/crawls/{cid}/sitemap?limit=10", token=t)
    assert len(r2["urls"]) <= 10, f"limit=10 returned {len(r2['urls'])} URLs"
    assert r2["url_count"] == r["url_count"], "total count shouldn't change with limit"
    print(f"      limit=10 -> {len(r2['urls'])} URLs returned")

    print("\nALL BATCH E SMOKE CHECKS PASSED")


if __name__ == "__main__":
    try:
        main()
    except (AssertionError, urllib.error.HTTPError) as e:
        print(f"\nFAILED: {e}", file=sys.stderr)
        if isinstance(e, urllib.error.HTTPError):
            print(e.read().decode(), file=sys.stderr)
        sys.exit(1)
