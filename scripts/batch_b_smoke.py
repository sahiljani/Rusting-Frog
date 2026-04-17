"""
Batch B smoke: exercises the /headers + /cookies detail endpoints and
the newly-wired http_headers_all + cookies_all reports against a live
crawl. Creates a fresh github.com crawl (guaranteed to set cookies),
waits for it to populate, then asserts every Batch-B surface returns
the shape documented in docs/openapi.yaml 0.11.0.
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


def req_raw(path, token=None):
    r = urllib.request.Request(
        BASE + path,
        headers={"Authorization": f"Bearer {token or tok()}"},
    )
    with urllib.request.urlopen(r, timeout=30) as resp:
        return resp.read().decode()


def main():
    t = tok()

    # Allow reusing an existing crawl via $SF_CRAWL_ID — avoids waiting for
    # the worker when it's already processing something else.
    cid = os.environ.get("SF_CRAWL_ID")
    if cid:
        print(f"[1/6] reusing existing crawl {cid}")
        s = req("GET", f"/v1/crawls/{cid}", token=t)
        assert s["status"] == "completed", f"reused crawl not completed: {s}"
        print(f"      crawl done, {s['urls_crawled']} urls")
    else:
        print(f"[1/6] creating project + github.com crawl …")
        proj = req("POST", "/v1/projects", {"name": "batch_b_smoke", "seed_url": "https://github.com/"}, t)
        pid = proj["id"]
        crawl = req("POST", f"/v1/projects/{pid}/crawls", {
            "seed_urls": ["https://github.com/"],
            "max_depth": 0,
            "max_urls": 1,
        }, t)
        cid = crawl["id"]
        print(f"      project={pid} crawl={cid}")

        # Wait up to 90s for the worker (it may still be draining a prior crawl)
        for i in range(90):
            s = req("GET", f"/v1/crawls/{cid}", token=t)
            if s["status"] == "completed":
                break
            if i >= 10 and s.get("urls_crawled", 0) >= 1:
                req("POST", f"/v1/crawls/{cid}/stop", {}, t)
                time.sleep(2)
                break
            time.sleep(1)
        s = req("GET", f"/v1/crawls/{cid}", token=t)
        assert s["status"] == "completed", f"crawl not completed: {s}"
        print(f"      crawl done, {s['urls_crawled']} urls")

    print(f"[2/6] pick first URL")
    urls = req("GET", f"/v1/crawls/{cid}/urls?limit=5", token=t)
    items = urls.get("data") or urls.get("items") or []
    assert items, "no urls returned"
    # Prefer the raw github.com/ root
    gh = next((u for u in items if u["url"] in ("https://github.com/", "https://github.com")), items[0])
    uid = gh["id"]
    print(f"      url_id={uid} url={gh['url']}")

    print(f"[3/6] /urls/:id/headers envelope")
    h = req("GET", f"/v1/crawls/{cid}/urls/{uid}/headers", token=t)
    for k in ("url", "header_count", "headers"):
        assert k in h, f"headers missing {k}: {h}"
    assert h["header_count"] > 0, "no headers captured"
    assert all("name" in x and "value" in x for x in h["headers"])
    print(f"      {h['header_count']} headers, final_url={h.get('final_url')}")

    print(f"[4/6] /urls/:id/cookies parses Set-Cookie attributes")
    c = req("GET", f"/v1/crawls/{cid}/urls/{uid}/cookies", token=t)
    for k in ("url", "count", "cookies"):
        assert k in c, f"cookies missing {k}: {c}"
    assert c["count"] > 0, f"expected github.com to set cookies, got {c}"
    first = c["cookies"][0]
    for k in ("name", "value", "domain", "path", "expires", "max_age", "secure", "http_only", "same_site", "raw"):
        assert k in first, f"cookie row missing {k}: {first}"
    print(f"      {c['count']} cookies, first={first['name']} (SameSite={first['same_site']}, HttpOnly={first['http_only']})")

    print(f"[5/6] http_headers_all report returns non-empty rows")
    r = req("GET", f"/v1/crawls/{cid}/reports/http_headers_all?format=json&limit=10", token=t)
    assert r["columns"] == ["url", "name", "value"], r["columns"]
    assert r["count"] > 0, f"no header rows: {r}"
    assert len(r["rows"]) > 0
    assert "name" in r["rows"][0] and "value" in r["rows"][0]
    print(f"      {r['count']} header rows across crawl")

    print(f"[6/6] cookies_all report returns parsed rows")
    r2 = req("GET", f"/v1/crawls/{cid}/reports/cookies_all?format=json&limit=10", token=t)
    expected_cols = ["url", "name", "value", "domain", "path", "expires", "secure", "http_only", "same_site"]
    assert r2["columns"] == expected_cols, r2["columns"]
    assert r2["count"] > 0, f"no cookie rows: {r2}"
    row = r2["rows"][0]
    for k in expected_cols:
        assert k in row, f"cookie report row missing {k}: {row}"
    print(f"      {r2['count']} cookie rows, first={row['name']} path={row['path']} secure={row['secure']}")

    # And CSV smoke
    csv = req_raw(f"/v1/crawls/{cid}/reports/cookies_all?format=csv", t)
    header_line = csv.splitlines()[0]
    assert header_line == ",".join(expected_cols), f"csv header mismatch: {header_line}"
    print(f"      CSV header OK: {header_line}")

    print("\nALL BATCH B SMOKE CHECKS PASSED")


if __name__ == "__main__":
    try:
        main()
    except (AssertionError, urllib.error.HTTPError) as e:
        print(f"\nFAILED: {e}", file=sys.stderr)
        if isinstance(e, urllib.error.HTTPError):
            print(e.read().decode(), file=sys.stderr)
        sys.exit(1)
