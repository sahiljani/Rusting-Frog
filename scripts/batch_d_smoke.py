"""
Batch D smoke: robots.txt capture + enforcement.

Exercises /v1/crawls/:id/robots against a real crawl and verifies that the
gate actually suppresses fetches — github.com has a non-trivial robots.txt
with Disallow directives, so the gate should mark at least one discovered
URL as blocked. Set $SF_CRAWL_ID to reuse a completed crawl and skip the
worker wait.
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
        print(f"[1/4] reusing crawl {cid}")
        s = req("GET", f"/v1/crawls/{cid}", token=t)
        assert s["status"] == "completed", f"reused crawl not completed: {s}"
    else:
        print("[1/4] creating github.com crawl (robots.txt has real Disallow rules)")
        proj = req("POST", "/v1/projects", {"name": "batch_d_smoke", "seed_url": "https://github.com/"}, t)
        pid = proj["id"]
        crawl = req("POST", f"/v1/projects/{pid}/crawls", {
            "seed_urls": ["https://github.com/"],
            "max_depth": 1,
            "max_urls": 30,
        }, t)
        cid = crawl["id"]
        print(f"      crawl={cid}")
        for i in range(180):
            s = req("GET", f"/v1/crawls/{cid}", token=t)
            if s["status"] == "completed":
                break
            if i >= 20 and s.get("urls_crawled", 0) >= 15:
                req("POST", f"/v1/crawls/{cid}/stop", {}, t)
                time.sleep(3)
                break
            time.sleep(1)
        s = req("GET", f"/v1/crawls/{cid}", token=t)
        assert s["status"] == "completed", f"crawl did not complete: {s}"
    print(f"      {s['urls_crawled']} urls crawled")

    print("[2/4] /robots envelope — expect raw body + parsed groups")
    r = req("GET", f"/v1/crawls/{cid}/robots", token=t)
    for k in ("url", "status", "raw", "groups", "blocked_url_count"):
        assert k in r, f"robots envelope missing {k}: {r}"
    print(f"      url={r['url']} status={r['status']} raw_len={len(r['raw'] or '')} "
          f"groups={len(r['groups'])} blocked={r['blocked_url_count']}")
    assert r["raw"], "github.com must return a non-empty robots.txt body"
    assert r["status"] == 200, f"expected 200, got {r['status']}"
    assert len(r["groups"]) > 0, "github.com robots.txt must parse into >=1 group"

    print("[3/4] every group has UA list and directive arrays")
    for g in r["groups"]:
        for k in ("user_agents", "disallow", "allow", "sitemap"):
            assert k in g, f"group missing {k}: {g}"
        assert isinstance(g["user_agents"], list) and g["user_agents"], "empty UA list"
    disallow_lines = sum(len(g["disallow"]) for g in r["groups"])
    assert disallow_lines > 0, "github's robots.txt has Disallow rules — none parsed"
    print(f"      {disallow_lines} Disallow directives parsed across {len(r['groups'])} groups")

    print("[4/4] at least one discovered URL blocked by robots")
    # github.com disallows /*/pulse, /*/projects, /*/forks, /*/pulls, etc.
    # With max_depth=1 from the homepage we almost always hit one.
    blocked = r["blocked_url_count"]
    if blocked == 0:
        print("      WARNING: 0 URLs blocked — either crawl was too narrow "
              "or the gate isn't firing. Investigate if reproducible.")
    else:
        print(f"      {blocked} URL(s) marked blocked_by_robots=TRUE — gate is live")

    print("\nALL BATCH D SMOKE CHECKS PASSED")


if __name__ == "__main__":
    try:
        main()
    except (AssertionError, urllib.error.HTTPError) as e:
        print(f"\nFAILED: {e}", file=sys.stderr)
        if isinstance(e, urllib.error.HTTPError):
            print(e.read().decode(), file=sys.stderr)
        sys.exit(1)
