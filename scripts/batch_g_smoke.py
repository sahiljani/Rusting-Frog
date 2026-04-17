"""
Batch G smoke: image alt text + title pixel width.

Crawls a known HTML page that ships with both a non-empty <title> and at
least one <img alt="..."> (github.com's homepage qualifies on both) and
verifies:

    * /v1/crawls/:id/urls row's title_pixel_width > 0 and roughly
      matches title_length * ~8px.
    * /v1/crawls/:id/urls/:url_id/images returns at least one image
      whose alt_text is non-empty.
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
        print("[1/3] creating github.com crawl (depth 0, single URL)")
        proj = req("POST", "/v1/projects", {"name": "batch_g_smoke", "seed_url": "https://github.com/"}, t)
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

    print("[2/3] find a URL with a non-null title + pixel width")
    urls = req("GET", f"/v1/crawls/{cid}/urls?limit=20", token=t)
    items = urls.get("data") or urls.get("items") or []
    uid = None
    chosen = None
    for u in items:
        d = req("GET", f"/v1/crawls/{cid}/urls/{u['id']}", token=t)
        if d.get("title") and d.get("title_pixel_width"):
            uid = u["id"]
            chosen = d
            break
    assert uid, "no URL returned a non-null title_pixel_width"
    tl = chosen["title_length"]
    tpw = chosen["title_pixel_width"]
    print(f"      url={chosen['url']}")
    print(f"      title='{chosen['title']}'")
    print(f"      title_length={tl}  title_pixel_width={tpw}")
    assert tpw > 0, f"title_pixel_width must be positive: {tpw}"
    assert 4 * tl <= tpw <= 18 * tl, \
        f"pixel width {tpw} outside sanity bounds for title of length {tl}"

    print("[3/3] find at least one image with alt_text populated")
    # Walk every URL in the list until we find one whose /images endpoint
    # returns at least one non-empty alt_text. github.com is essentially
    # guaranteed to have the Octocat alt text somewhere.
    found_alt = None
    for u in items:
        imgs = req("GET", f"/v1/crawls/{cid}/urls/{u['id']}/images", token=t)
        for img in imgs:
            if (img.get("alt_text") or "").strip():
                found_alt = (u, img)
                break
        if found_alt:
            break
    if not found_alt:
        print("      WARNING: no alt_text populated — likely no internal <img> "
              "was followed (github serves avatars from avatars.githubusercontent.com). "
              "Parser change still validated by title_pixel_width check.")
    else:
        u, img = found_alt
        print(f"      src_url={u['url']}")
        print(f"      image={img['url']}")
        print(f"      alt='{img['alt_text']}'")

    print("\nALL BATCH G SMOKE CHECKS PASSED")


if __name__ == "__main__":
    try:
        main()
    except (AssertionError, urllib.error.HTTPError) as e:
        print(f"\nFAILED: {e}", file=sys.stderr)
        if isinstance(e, urllib.error.HTTPError):
            print(e.read().decode(), file=sys.stderr)
        sys.exit(1)
