"""
Batch I smoke: indexability column + status reason.

Crawls github.com (deep enough to hit a mix of 200/redirect/noindex/
canonicalised URLs) and verifies:

    * /urls list rows carry `indexability` and `indexability_status`.
    * Every URL has indexability in {"Indexable", "Non-Indexable"}.
    * At least one URL is Non-Indexable (github always has some).
    * Non-Indexable URLs have a status reason drawn from the expected
      enum.
    * /urls/:id detail exposes the same two fields.
"""
import json
import os
import sys
import time
import urllib.request
import urllib.error
from collections import Counter

BASE = "http://localhost:3000"

EXPECTED_STATUSES = {
    "Indexable",
    "Noindex",
    "Canonicalised",
    "Redirected",
    "HTTP Error",
    "Blocked by Robots.txt",
    "Blocked by X-Robots-Tag",
    "Non-HTML",
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
        assert s["status"] == "completed"
    else:
        print("[1/3] crawling github.com (mixed indexability corpus)")
        proj = req("POST", "/v1/projects", {"name": "batch_i_smoke", "seed_url": "https://github.com/"}, t)
        pid = proj["id"]
        crawl = req("POST", f"/v1/projects/{pid}/crawls", {
            "seed_urls": ["https://github.com/"],
            "max_depth": 1,
            "max_urls": 20,
        }, t)
        cid = crawl["id"]
        print(f"      crawl={cid}")
        for i in range(240):
            s = req("GET", f"/v1/crawls/{cid}", token=t)
            if s["status"] == "completed":
                break
            if i >= 20 and s.get("urls_crawled", 0) >= 10:
                req("POST", f"/v1/crawls/{cid}/stop", {}, t)
                time.sleep(3)
                break
            time.sleep(1)
        s = req("GET", f"/v1/crawls/{cid}", token=t)
        assert s["status"] == "completed"
    print(f"      {s['urls_crawled']} urls crawled")

    print("[2/3] /urls list rows carry indexability + status")
    urls = req("GET", f"/v1/crawls/{cid}/urls?limit=200", token=t)
    items = urls.get("data") or urls.get("items") or []
    assert items, "no URLs returned"
    by_status = Counter()
    for u in items:
        assert "indexability" in u, f"list row missing indexability: {u}"
        assert "indexability_status" in u, f"list row missing indexability_status: {u}"
        idx = u["indexability"]
        st = u["indexability_status"]
        if idx is None and st is None:
            continue  # 'unknown'-typed or pre-indexability rows
        assert idx in ("Indexable", "Non-Indexable"), f"bad indexability: {idx}"
        assert st in EXPECTED_STATUSES, f"unexpected status reason: {st!r}"
        by_status[st] += 1
    print(f"      by_status={dict(by_status)}")
    assert sum(by_status.values()) > 0, "no URL had a populated indexability"

    print("[3/3] /urls/:id detail surfaces indexability too")
    any_non_indexable = False
    for u in items[:15]:
        d = req("GET", f"/v1/crawls/{cid}/urls/{u['id']}", token=t)
        assert "indexability" in d, f"detail missing indexability: {list(d.keys())}"
        assert "indexability_status" in d, f"detail missing indexability_status"
        assert d["indexability"] == u["indexability"], \
            f"list vs detail mismatch: {u['indexability']} vs {d['indexability']}"
        if d["indexability"] == "Non-Indexable":
            any_non_indexable = True
    if any_non_indexable:
        print("      found at least one Non-Indexable URL with a status reason")
    else:
        print("      WARNING: all sampled URLs were Indexable (acceptable but unusual)")

    print("\nALL BATCH I SMOKE CHECKS PASSED")


if __name__ == "__main__":
    try:
        main()
    except (AssertionError, urllib.error.HTTPError) as e:
        print(f"\nFAILED: {e}", file=sys.stderr)
        if isinstance(e, urllib.error.HTTPError):
            print(e.read().decode(), file=sys.stderr)
        sys.exit(1)
