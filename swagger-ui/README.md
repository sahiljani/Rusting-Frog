# Swagger UI app

Standalone API explorer for the Rusting Frog backend. **Not served by the Rust
binary** — this is a separate static app that talks to the API as an external
client. Keep it that way so the production Rust build stays minimal.

## Use

```bash
# 1. Make sure the API is running on :3000
cargo run -p sf-api

# 2. Serve the two static files (openapi.yaml + this dir) on any port:
cd /c/Users/User/Desktop/sf-clone-rust
python -m http.server 8090

# 3. Open http://localhost:8090/swagger-ui/
```

Paste a JWT (mint one with `node scripts/mint_jwt.js`), click **Apply**, then
click **Authorize** — every request from "Try it out" picks up the Bearer token.

Why a dedicated HTTP server instead of `file://`? Browsers block `fetch` of
local files. Any static server works; Python's built-in is the easiest.

## Wiring

- `index.html` loads Swagger UI from unpkg and `../docs/openapi.yaml` via fetch.
- `requestInterceptor` attaches `Authorization: Bearer <jwt>` to every request.
- Server URL is configurable at runtime (top-bar input), so the same app can
  point at prod, staging, or a colleague's box.
- `persistAuthorization: true` keeps the token across reloads.
