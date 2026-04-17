"""
Batch H smoke: SERP snippet envelope + meta description pixel width.

Crawls a known page that serves both a <title> and a <meta name="description">
(en.wikipedia.org is reliable — every article has both), then asserts:

    * /v1/crawls/:id/urls/:url_id/serp returns the full envelope with
      `title.length_pixels`, `description.length_pixels` populated.
    * Both pixel widths are positive integers for a URL with content.
    * Per-URL detail surfaces `meta_description_pixel_width`.
    * `description.truncated` is a bool reflecting >max_pixels logic.
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
        seed = "https://en.wikipedia.org/wiki/Search_engine_optimization"
        print(f"[1/4] crawling {seed}")
        proj = req("POST", "/v1/projects", {"name": "batch_h_smoke", "seed_url": seed}, t)
        pid = proj["id"]
        crawl = req("POST", f"/v1/projects/{pid}/crawls", {
            "seed_urls": [seed],
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

    print("[2/4] find a URL with both title + meta description")
    urls = req("GET", f"/v1/crawls/{cid}/urls?limit=20", token=t)
    items = urls.get("data") or urls.get("items") or []
    target = None
    for u in items:
        d = req("GET", f"/v1/crawls/{cid}/urls/{u['id']}", token=t)
        if d.get("title") and d.get("meta_description"):
            target = (u, d)
            break
    assert target, "no URL had both title + meta_description"
    u, d = target
    print(f"      url={d['url']}")

    print("[3/4] /urls/:id surfaces meta_description_pixel_width")
    assert "meta_description_pixel_width" in d, \
        f"detail missing meta_description_pixel_width: {list(d.keys())}"
    mpw = d["meta_description_pixel_width"]
    assert isinstance(mpw, int) and mpw > 0, \
        f"meta_description_pixel_width must be > 0: {mpw}"
    print(f"      meta_description_length={d['meta_description_length']}  meta_description_pixel_width={mpw}")

    print("[4/4] /serp envelope invariants")
    serp = req("GET", f"/v1/crawls/{cid}/urls/{u['id']}/serp", token=t)
    for k in ("url", "breadcrumb", "title", "description"):
        assert k in serp, f"serp missing top-level {k}: {list(serp.keys())}"
    for k in ("value", "length_chars", "length_pixels", "max_pixels", "truncated"):
        assert k in serp["title"], f"serp.title missing {k}"
        assert k in serp["description"], f"serp.description missing {k}"
    desc = serp["description"]
    title = serp["title"]
    assert isinstance(desc["length_pixels"], int) and desc["length_pixels"] > 0, \
        f"description.length_pixels must be > 0: {desc['length_pixels']}"
    assert desc["length_pixels"] == mpw, \
        f"serp description.length_pixels ({desc['length_pixels']}) must match /urls detail meta_description_pixel_width ({mpw})"
    assert isinstance(desc["truncated"], bool)
    assert isinstance(title["truncated"], bool)
    expected_trunc = desc["length_pixels"] > desc["max_pixels"]
    assert desc["truncated"] == expected_trunc, \
        f"truncated={desc['truncated']} but pixels={desc['length_pixels']} > max={desc['max_pixels']} gives {expected_trunc}"
    print(f"      title:        '{title['value']}'")
    print(f"      title_pixels: {title['length_pixels']} / {title['max_pixels']} (trunc={title['truncated']})")
    print(f"      desc_pixels:  {desc['length_pixels']} / {desc['max_pixels']} (trunc={desc['truncated']})")

    print("\nALL BATCH H SMOKE CHECKS PASSED")


if __name__ == "__main__":
    try:
        main()
    except (AssertionError, urllib.error.HTTPError) as e:
        print(f"\nFAILED: {e}", file=sys.stderr)
        if isinstance(e, urllib.error.HTTPError):
            print(e.read().decode(), file=sys.stderr)
        sys.exit(1)
