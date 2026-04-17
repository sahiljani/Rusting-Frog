# Rusting Frog — UI

A Screaming-Frog-style single-page app on top of the `sf-clone-rust` API.

## Run

Terminal 1 — **API** (dev mode so it mints a bearer token for the UI):

```
cd ..
SF_DEV_MODE=1 SF_ALLOW_PRIVATE_IPS=1 ./target/debug/sf-api.exe
```

Terminal 2 — **crawl worker**:

```
cd ..
SF_ALLOW_PRIVATE_IPS=1 ./target/debug/sf-crawl-worker.exe
```

Terminal 3 — **UI**:

```
npm install
npm run dev
```

Open http://localhost:5173. Type a URL, click **Start Audit**.

## How it talks to the API

- `GET /v1/dev/token` (dev-only) — cached for ~55 min in memory.
- `GET /v1/catalog/tabs` — tab + filter registry.
- `POST /v1/projects`, `POST /v1/projects/:id/crawls` — kick a crawl.
- `GET /v1/crawls/:id` + `GET /v1/crawls/:id/overview` — polled every 2s while running.
- `GET /v1/crawls/:id/urls?filter=...` — grid content when a filter is clicked.
- `GET /v1/crawls/:id/urls/:url_id` — right-pane detail.

Vite proxies `/v1/*` to `http://localhost:3000`, so no CORS hoops.
