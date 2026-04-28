# syntax=docker/dockerfile:1.7
#
# Single image, three binaries. docker-compose chooses which one runs
# (sf-api on port 3000, sf-crawl-worker polling the DB, sf-migrations
# one-shot on startup). SQL files are embedded into sf-migrations at
# compile time via sqlx::migrate! so the runtime image doesn't ship
# the .sql tree.

# ---- builder ----
FROM rust:1.88-slim-bookworm AS builder

# libssl-dev: `openssl-sys` (transitive via reqwest default features) needs
# the OpenSSL headers at compile time. pkg-config: lets the build script
# discover them. ca-certificates + clang: sometimes pulled in by other
# -sys crates (cheap to include).
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
        ca-certificates \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Offline mode — sqlx::query! checks resolve against .sqlx/ instead of
# hitting a live Postgres at compile time. Regenerate on schema change:
#     cargo sqlx prepare --workspace
ENV SQLX_OFFLINE=true

COPY Cargo.toml Cargo.lock ./
COPY .sqlx/ .sqlx/
COPY crates/ crates/

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release \
        --bin sf-api \
        --bin sf-crawl-worker \
        --bin sf-migrations \
 && mkdir -p /out \
 && cp target/release/sf-api target/release/sf-crawl-worker target/release/sf-migrations /out/

# ---- runtime ----
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
 && apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 \
        tini \
 && rm -rf /var/lib/apt/lists/* \
 && groupadd --system sf \
 && useradd --system --gid sf --home-dir /app --shell /usr/sbin/nologin sf \
 && mkdir -p /var/log/sf-debug \
 && chown sf:sf /var/log/sf-debug

WORKDIR /app

COPY --from=builder /out/sf-api            /usr/local/bin/sf-api
COPY --from=builder /out/sf-crawl-worker   /usr/local/bin/sf-crawl-worker
COPY --from=builder /out/sf-migrations     /usr/local/bin/sf-migrations

USER sf

EXPOSE 3000
ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["sf-api"]
