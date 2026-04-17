# syntax=docker/dockerfile:1.7
#
# Builds the React SPA with Vite, then serves the static bundle via
# nginx and reverse-proxies /v1/* to the Rust API on the docker-compose
# internal network (service name `api`, port 3000). The browser only
# ever talks to port 80 — no CORS, no direct API exposure needed.
#
# Build context is ./UI (see deploy.yml), so all COPY paths below are
# relative to the UI folder, not the repo root.

# ---- builder ----
FROM node:20-alpine AS builder
WORKDIR /app

COPY package.json package-lock.json ./
RUN npm ci

COPY . ./
RUN npm run build

# ---- runtime ----
FROM nginx:1.27-alpine AS runtime

COPY --from=builder /app/dist /usr/share/nginx/html
COPY nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80
