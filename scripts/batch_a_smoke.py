"""Batch-A smoke: hit every new endpoint and every report on an existing
crawl. Fails fast on non-2xx or bad envelope."""
import json, os, sys, urllib.request, urllib.error

BASE = "http://localhost:3000/v1"
CRAWL = os.environ["CRAWL_ID"]
TOKEN = os.environ.get("SF_TOKEN") or open(os.path.expanduser("~/.sf_tok.txt")).read().strip()

def req(path):
    r = urllib.request.Request(BASE + path, headers={"Authorization": f"Bearer {TOKEN}"})
    try:
        with urllib.request.urlopen(r, timeout=30) as resp:
            body = resp.read()
            ct = resp.headers.get("Content-Type", "")
            return resp.status, ct, body
    except urllib.error.HTTPError as e:
        return e.code, e.headers.get("Content-Type", ""), e.read()

def ok(cond, msg):
    mark = "PASS" if cond else "FAIL"
    print(f"[{mark}] {msg}")
    if not cond:
        sys.exit(1)

# ---------- reports catalog ----------
code, _, body = req(f"/crawls/{CRAWL}/reports")
cat = json.loads(body)
ok(code == 200, f"reports catalog 200 (got {code})")
ok("reports" in cat and isinstance(cat["reports"], list), "catalog has reports[]")
print(f"      catalog: {len(cat['reports'])} reports across {len({r['group'] for r in cat['reports']})} groups")

# ---------- run every report as JSON ----------
report_keys = [r["key"] for r in cat["reports"]]
for key in report_keys:
    code, ct, body = req(f"/crawls/{CRAWL}/reports/{key}?format=json&limit=5")
    if code != 200:
        print(f"[FAIL] {key} -> {code} {body[:200]!r}")
        sys.exit(1)
    env = json.loads(body)
    for f in ("key", "title", "group", "count", "columns", "rows"):
        if f not in env:
            print(f"[FAIL] {key} missing field {f}")
            sys.exit(1)
    print(f"[PASS] {key:36} count={env['count']:>5} cols={len(env['columns'])} rows={len(env['rows'])}" + (" (notes)" if env.get("notes") else ""))

# ---------- CSV format ----------
code, ct, body = req(f"/crawls/{CRAWL}/reports/redirects_all?format=csv&limit=5")
ok(code == 200, "csv redirects_all 200")
ok("text/csv" in ct, f"csv content-type (got {ct})")
ok(b"," in body or len(body.splitlines()) >= 1, "csv has content")
print(f"      CSV bytes={len(body)} first line={body.splitlines()[0][:80]!r}")

# ---------- issues ----------
code, _, body = req(f"/crawls/{CRAWL}/issues")
ok(code == 200, f"issues 200 (got {code})")
iss = json.loads(body)
for f in ("summary", "total_urls", "items"):
    ok(f in iss, f"issues envelope has {f}")
s = iss["summary"]
for f in ("issues", "warnings", "opportunities", "info", "total"):
    ok(f in s, f"summary has {f}")
print(f"      summary = {s}")
print(f"      total_urls = {iss['total_urls']}")
print(f"      top 5 items:")
for it in iss["items"][:5]:
    desc = (it.get("description") or "")[:60]
    print(f"        {it['issue_type']:11} {it['priority']:7} urls={it['urls']:>4} {it['issue_name']}")
    print(f"                    desc: {desc!r}")

# ---------- /urls/:id/images + /serp ----------
# Pick a URL with an image outlink.
import subprocess
env = os.environ.copy()
env["PGPASSWORD"] = "111"
u_id = subprocess.check_output(["psql", "-U", "postgres", "-h", "localhost", "-d", "sf_clone",
    "-tAc", f"SELECT u.id FROM crawl_urls u WHERE u.crawl_id='{CRAWL}' AND u.title IS NOT NULL ORDER BY u.url LIMIT 1;"], env=env).decode().strip()
print(f"      test URL id = {u_id}")

code, _, body = req(f"/crawls/{CRAWL}/urls/{u_id}/images")
ok(code == 200, f"images 200 (got {code})")
imgs = json.loads(body)
ok(isinstance(imgs, list), "images is list")
print(f"      images count = {len(imgs)}")
if imgs:
    print(f"      sample image keys: {sorted(imgs[0].keys())}")

code, _, body = req(f"/crawls/{CRAWL}/urls/{u_id}/serp")
ok(code == 200, f"serp 200 (got {code})")
serp = json.loads(body)
for f in ("url", "breadcrumb", "title", "description"):
    ok(f in serp, f"serp envelope has {f}")
for f in ("value", "length_chars", "length_pixels", "max_chars", "max_pixels", "remaining_chars", "remaining_pixels", "truncated"):
    ok(f in serp["title"], f"serp.title has {f}")
print(f"      serp title: {serp['title']['length_chars']} chars / {serp['title']['length_pixels']} px, truncated={serp['title']['truncated']}")
print(f"      serp desc:  {serp['description']['length_chars']} chars / {serp['description']['length_pixels_approx']} px, truncated={serp['description']['truncated']}")
print(f"      breadcrumb: {' > '.join(serp['breadcrumb'])}")

print("\nALL BATCH A SMOKE CHECKS PASSED")
