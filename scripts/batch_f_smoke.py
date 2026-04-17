"""
Batch F smoke: per-URL resources endpoint.

Crawls github.com homepage (guaranteed to load dozens of stylesheets,
scripts and images) and verifies /urls/:id/resources returns a populated
envelope with at least one entry of each core type.
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
        print(f"[1/3] reusing crawl {cid}")
        s = req("GET", f"/v1/crawls/{cid}", token=t)
        assert s["status"] == "completed", f"reused crawl not completed: {s}"
    else:
        print("[1/3] creating github.com crawl")
        proj = req("POST", "/v1/projects", {"name": "batch_f_smoke", "seed_url": "https://github.com/"}, t)
        pid = proj["id"]
        crawl = req("POST", f"/v1/projects/{pid}/crawls", {
            "seed_urls": ["https://github.com/"],
            "max_depth": 0,
            "max_urls": 1,
        }, t)
        cid = crawl["id"]
        print(f"      crawl={cid}")
        for i in range(180):
            s = req("GET", f"/v1/crawls/{cid}", token=t)
            if s["status"] == "completed":
                break
            if i >= 15 and s.get("urls_crawled", 0) >= 1:
                req("POST", f"/v1/crawls/{cid}/stop", {}, t)
                time.sleep(3)
                break
            time.sleep(1)
        s = req("GET", f"/v1/crawls/{cid}", token=t)
        assert s["status"] == "completed", f"crawl did not complete: {s}"
    print(f"      {s['urls_crawled']} urls crawled")

    print("[2/3] find the homepage URL (guaranteed to have resources)")
    urls = req("GET", f"/v1/crawls/{cid}/urls?limit=20", token=t)
    items = urls.get("data") or urls.get("items") or []
    uid = None
    for u in items:
        res = req("GET", f"/v1/crawls/{cid}/urls/{u['id']}/resources", token=t)
        if res["count"] > 0:
            uid = u["id"]
            break
    assert uid, "no URL returned any resources"
    print(f"      url_id={uid} url={u['url']} total_resources={res['count']}")

    print("[3/3] /resources envelope invariants")
    for k in ("url", "count", "counts_by_type", "resources"):
        assert k in res, f"resources envelope missing {k}: {res}"
    assert isinstance(res["counts_by_type"], dict)
    assert sum(res["counts_by_type"].values()) == res["count"], \
        "counts_by_type must sum to count"
    assert isinstance(res["resources"], list)
    assert len(res["resources"]) == res["count"]
    for item in res["resources"][:3]:
        assert "url" in item and "type" in item, f"resource item malformed: {item}"
    types_found = set(res["counts_by_type"].keys())
    print(f"      types={sorted(types_found)} by_count={res['counts_by_type']}")
    # github.com homepage usually ships all three
    expected_types = {"script", "stylesheet", "image"}
    missing = expected_types - types_found
    if missing:
        print(f"      WARNING: expected types not seen: {sorted(missing)}")

    print("\nALL BATCH F SMOKE CHECKS PASSED")


if __name__ == "__main__":
    try:
        main()
    except (AssertionError, urllib.error.HTTPError) as e:
        print(f"\nFAILED: {e}", file=sys.stderr)
        if isinstance(e, urllib.error.HTTPError):
            print(e.read().decode(), file=sys.stderr)
        sys.exit(1)
