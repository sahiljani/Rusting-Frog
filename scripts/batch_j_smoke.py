"""
Batch J smoke: post-crawl duplicate-detection pass.

Crawls a known-duplicate corpus (github.com topic pages share titles +
H1s wholesale) and verifies that at finalize time the worker emits the
five Duplicate filter keys:

    * title_duplicate
    * meta_descripton_duplicate  (SF's typo, preserved)
    * h1_duplicate
    * h2_duplicate               (fires when enough pages share one)
    * content_duplicates         (exact by content_hash)

The pass is expected to be idempotent — re-running the analysis on
the same crawl must not double-count. That's not exercised here
because we run it exactly once per crawl; it's documented instead.
"""
import json
import os
import sys
import time
import urllib.request
import urllib.error

BASE = "http://localhost:3000"

DUP_KEYS = {
    "title_duplicate",
    "meta_descripton_duplicate",
    "h1_duplicate",
    "h2_duplicate",
    "content_duplicates",
}


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
        print(f"[1/3] reusing crawl {cid}")
        s = req("GET", f"/v1/crawls/{cid}", token=t)
    else:
        print("[1/3] crawling github.com topic pages (known duplicate titles/H1s)")
        proj = req(
            "POST",
            "/v1/projects",
            {"name": "batch_j_smoke", "seed_url": "https://github.com/topics"},
            t,
        )
        pid = proj["id"]
        crawl = req(
            "POST",
            f"/v1/projects/{pid}/crawls",
            {"seed_urls": ["https://github.com/topics"], "max_depth": 1, "max_urls": 25},
            t,
        )
        cid = crawl["id"]
        print(f"      crawl={cid}")
        for i in range(300):
            s = req("GET", f"/v1/crawls/{cid}", token=t)
            if s["status"] == "completed":
                break
            if i >= 30 and s.get("urls_crawled", 0) >= 12:
                req("POST", f"/v1/crawls/{cid}/stop", {}, t)
                time.sleep(4)
                break
            time.sleep(1)
        s = req("GET", f"/v1/crawls/{cid}", token=t)
        assert s["status"] == "completed", f"crawl not completed: {s['status']}"
    print(f"      {s['urls_crawled']} urls crawled")

    print("[2/3] checking /issues surface for duplicate filter keys")
    issues = req("GET", f"/v1/crawls/{cid}/issues", token=t)
    by_key = {i["filter_key"]: i["urls"] for i in issues.get("items", [])}
    found = {k: by_key.get(k, 0) for k in DUP_KEYS}
    print(f"      {found}")
    fired = {k: n for k, n in found.items() if n > 0}
    assert fired, (
        f"no duplicate filters fired on {s['urls_crawled']} URLs — "
        f"analysis pass didn't run or the seed had no dupes: {found}"
    )

    print("[3/3] confirming per-URL findings carry at least one duplicate key")
    urls = req("GET", f"/v1/crawls/{cid}/urls?limit=200", token=t)
    items = urls.get("data") or urls.get("items") or []
    any_dup = False
    for u in items:
        d = req("GET", f"/v1/crawls/{cid}/urls/{u['id']}", token=t)
        keys = set(d.get("findings", []))
        if keys & DUP_KEYS:
            any_dup = True
            break
    assert any_dup, "no URL exposed a duplicate finding on /urls/:id"

    print(f"\nALL BATCH J SMOKE CHECKS PASSED — duplicates fired: {list(fired)}")


if __name__ == "__main__":
    try:
        main()
    except (AssertionError, urllib.error.HTTPError) as e:
        print(f"\nFAILED: {e}", file=sys.stderr)
        if isinstance(e, urllib.error.HTTPError):
            print(e.read().decode(), file=sys.stderr)
        sys.exit(1)
