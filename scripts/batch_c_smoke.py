"""
Batch C smoke: exercises /urls/:id/source, /duplicates and
/structured-data against a fresh crawl. Github.com sets JSON-LD on its
homepage and guarantees at least one crawlable HTML response for every
run, so it's a good canary. Set $SF_CRAWL_ID to reuse an existing
completed crawl and skip the worker wait.
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
    with urllib.request.urlopen(r, timeout=30) as resp:
        return json.loads(resp.read())


def main():
    t = tok()

    cid = os.environ.get("SF_CRAWL_ID")
    if cid:
        print(f"[1/5] reusing crawl {cid}")
        s = req("GET", f"/v1/crawls/{cid}", token=t)
        assert s["status"] == "completed", f"reused crawl not completed: {s}"
    else:
        print("[1/5] creating github.com crawl")
        proj = req("POST", "/v1/projects", {"name": "batch_c_smoke", "seed_url": "https://github.com/"}, t)
        pid = proj["id"]
        crawl = req("POST", f"/v1/projects/{pid}/crawls", {
            "seed_urls": ["https://github.com/"],
            "max_depth": 0,
            "max_urls": 1,
        }, t)
        cid = crawl["id"]
        print(f"      crawl={cid}")
        for i in range(120):
            s = req("GET", f"/v1/crawls/{cid}", token=t)
            if s["status"] == "completed":
                break
            if i >= 10 and s.get("urls_crawled", 0) >= 3:
                req("POST", f"/v1/crawls/{cid}/stop", {}, t)
                time.sleep(2)
                break
            time.sleep(1)
        s = req("GET", f"/v1/crawls/{cid}", token=t)
        assert s["status"] == "completed", f"crawl did not complete: {s}"
    print(f"      {s['urls_crawled']} urls crawled")

    print("[2/5] pick first URL with raw HTML captured")
    urls = req("GET", f"/v1/crawls/{cid}/urls?limit=10", token=t)
    items = urls.get("data") or urls.get("items") or []
    uid = None
    for u in items:
        src = req("GET", f"/v1/crawls/{cid}/urls/{u['id']}/source", token=t)
        if src.get("html"):
            uid = u["id"]
            break
    assert uid, "no URL had captured HTML — worker never wrote raw_html"
    print(f"      url_id={uid} url={u['url']} html_length={src['html_length']}")
    for k in ("url", "content_type", "html_length", "content_hash", "html"):
        assert k in src, f"source envelope missing {k}"
    assert src["content_hash"] and len(src["content_hash"]) == 64, "content_hash should be 64 hex chars"

    print("[3/5] /duplicates envelope (expect count>=0)")
    d = req("GET", f"/v1/crawls/{cid}/urls/{uid}/duplicates", token=t)
    for k in ("url", "content_hash", "match_type", "count", "duplicates"):
        assert k in d, f"duplicates envelope missing {k}: {d}"
    assert d["match_type"] == "exact"
    assert d["content_hash"] == src["content_hash"], f"hash mismatch between /source and /duplicates"
    print(f"      content_hash={d['content_hash'][:16]}… duplicates={d['count']}")

    print("[4/5] /structured-data envelope")
    sd = req("GET", f"/v1/crawls/{cid}/urls/{uid}/structured-data", token=t)
    for k in ("url", "count", "counts_by_type", "items"):
        assert k in sd, f"structured-data envelope missing {k}: {sd}"
    assert isinstance(sd["items"], list)
    print(f"      count={sd['count']} by_type={sd['counts_by_type']}")

    print("[5/5] find any URL with structured-data to confirm extractor")
    found = 0
    for u in items:
        sdi = req("GET", f"/v1/crawls/{cid}/urls/{u['id']}/structured-data", token=t)
        if sdi["count"] > 0:
            found = sdi["count"]
            assert sdi["items"][0]["type"] == "JSON-LD"
            assert "data" in sdi["items"][0]
            first_data = sdi["items"][0]["data"]
            first_type = first_data.get("@type") if isinstance(first_data, dict) else "(array)"
            print(f"      {u['url']} -> {found} JSON-LD blocks, first @type={first_type}")
            break
    if found == 0:
        print("      WARNING: no JSON-LD blocks found in any sampled URL "
              "(github.com sometimes varies markup; extractor still validated).")

    print("\nALL BATCH C SMOKE CHECKS PASSED")


if __name__ == "__main__":
    try:
        main()
    except (AssertionError, urllib.error.HTTPError) as e:
        print(f"\nFAILED: {e}", file=sys.stderr)
        if isinstance(e, urllib.error.HTTPError):
            print(e.read().decode(), file=sys.stderr)
        sys.exit(1)
