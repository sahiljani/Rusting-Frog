const API_BASE = ''; // via Vite proxy — same-origin /v1/*

const TOKEN_KEY = 'sf-clone.jwt';
const TOKEN_EXP_KEY = 'sf-clone.jwt.exp';

function readStoredToken(): { token: string; exp: number } | null {
  try {
    const token = localStorage.getItem(TOKEN_KEY);
    const exp = Number(localStorage.getItem(TOKEN_EXP_KEY) ?? 0);
    if (!token || !exp) return null;
    return { token, exp };
  } catch {
    return null;
  }
}

function writeStoredToken(token: string, exp: number) {
  try {
    localStorage.setItem(TOKEN_KEY, token);
    localStorage.setItem(TOKEN_EXP_KEY, String(exp));
  } catch {
    /* quota / private-mode — proceed without persisting */
  }
}

export function clearToken() {
  try {
    localStorage.removeItem(TOKEN_KEY);
    localStorage.removeItem(TOKEN_EXP_KEY);
  } catch {
    /* ignore */
  }
}

export function hasStoredToken(): boolean {
  const stored = readStoredToken();
  if (!stored) return false;
  return stored.exp - 60 > Math.floor(Date.now() / 1000);
}

export async function getToken(): Promise<string> {
  const now = Math.floor(Date.now() / 1000);
  const stored = readStoredToken();
  if (stored && stored.exp - 60 > now) return stored.token;
  const r = await fetch(`${API_BASE}/v1/dev/token`);
  if (!r.ok) {
    throw new Error(
      `/v1/dev/token returned ${r.status}. Is sf-api running with SF_DEV_MODE=1?`,
    );
  }
  const j = await r.json();
  const token = j.token as string;
  const exp = now + (j.expires_in ?? 3600);
  writeStoredToken(token, exp);
  return token;
}

async function authedFetch(path: string, init: RequestInit = {}): Promise<Response> {
  const token = await getToken();
  const headers = new Headers(init.headers);
  headers.set('Authorization', `Bearer ${token}`);
  if (init.body && !headers.has('Content-Type')) headers.set('Content-Type', 'application/json');
  return fetch(`${API_BASE}${path}`, { ...init, headers });
}

async function json<T>(r: Response): Promise<T> {
  if (!r.ok) {
    let body = '';
    try {
      body = await r.text();
    } catch {
      /* empty */
    }
    throw new Error(`${r.status} ${r.statusText}: ${body}`);
  }
  return r.json();
}

export interface FilterDef {
  key: string;
  display_name: string;
  severity: string;
  filter_type: string;
  has_watermark: boolean;
}

export interface TabDef {
  key: string;
  display_name: string;
  i18n_key: string;
  has_dynamic_filters: boolean;
  filters: FilterDef[];
}

export async function fetchTabs(): Promise<TabDef[]> {
  return json<TabDef[]>(await authedFetch('/v1/catalog/tabs'));
}

export interface Project {
  id: string;
  name: string;
  seed_url: string;
  created_at: string;
  updated_at: string;
}

export async function createProject(name: string, seedUrl: string): Promise<Project> {
  return json<Project>(
    await authedFetch('/v1/projects', {
      method: 'POST',
      body: JSON.stringify({ name, seed_url: seedUrl }),
    }),
  );
}

export interface CrawlStart {
  id: string;
  status: string;
  message?: string;
}

export async function startCrawl(projectId: string): Promise<CrawlStart> {
  return json<CrawlStart>(
    await authedFetch(`/v1/projects/${projectId}/crawls`, {
      method: 'POST',
      body: JSON.stringify({}),
    }),
  );
}

export interface CrawlStatus {
  id: string;
  project_id: string;
  seed_urls: string[];
  status: 'queued' | 'running' | 'completed' | 'failed' | 'paused';
  started_at?: string | null;
  completed_at?: string | null;
  created_at: string;
  urls_discovered?: number;
  urls_crawled?: number;
}

export async function getCrawl(crawlId: string): Promise<CrawlStatus> {
  return json<CrawlStatus>(await authedFetch(`/v1/crawls/${crawlId}`));
}

export async function stopCrawl(crawlId: string): Promise<void> {
  await authedFetch(`/v1/crawls/${crawlId}/stop`, { method: 'POST' });
}

export async function pauseCrawl(crawlId: string): Promise<void> {
  await authedFetch(`/v1/crawls/${crawlId}/pause`, { method: 'POST' });
}

export async function resumeCrawl(crawlId: string): Promise<void> {
  await authedFetch(`/v1/crawls/${crawlId}/resume`, { method: 'POST' });
}

export type OverviewCounts = Record<string, number>;

export async function getOverview(crawlId: string): Promise<OverviewCounts> {
  return json<OverviewCounts>(await authedFetch(`/v1/crawls/${crawlId}/overview`));
}

export interface CrawlUrlRow {
  id: string;
  url: string;
  status_code: number | null;
  content_type: string | null;
  is_internal: boolean;
  depth: number | null;
  title: string | null;
  title_length: number | null;
  h1_first: string | null;
  h1_count: number | null;
  word_count: number | null;
  response_time_ms: number | null;
  content_length: number | null;
  crawled_at: string | null;
  indexability?: string | null;
  indexability_status?: string | null;
}

export interface UrlsPage {
  data: CrawlUrlRow[];
  next_cursor: string | null;
}

export async function listUrls(
  crawlId: string,
  opts: { filter?: string; q?: string; limit?: number } = {},
): Promise<UrlsPage> {
  const qs = new URLSearchParams();
  if (opts.filter) qs.set('filter', opts.filter);
  if (opts.q) qs.set('q', opts.q);
  qs.set('limit', String(opts.limit ?? 200));
  return json<UrlsPage>(await authedFetch(`/v1/crawls/${crawlId}/urls?${qs}`));
}

export interface UrlDetail extends CrawlUrlRow {
  title_pixel_width: number | null;
  meta_description: string | null;
  meta_description_length: number | null;
  meta_description_pixel_width?: number | null;
  h2_first: string | null;
  h2_count: number | null;
  redirect_url: string | null;
  canonical_url: string | null;
  meta_robots: string | null;
  findings: string[];
}

export async function getUrlDetail(crawlId: string, urlId: string): Promise<UrlDetail> {
  return json<UrlDetail>(await authedFetch(`/v1/crawls/${crawlId}/urls/${urlId}`));
}

// ---- Per-URL sub-resources (feed the tabbed DetailPane) ----

export interface InlinkRow {
  source_url_id: string;
  source_url: string;
  anchor_text: string | null;
  link_type: string | null;
}

export async function getInlinks(crawlId: string, urlId: string): Promise<InlinkRow[]> {
  return json<InlinkRow[]>(await authedFetch(`/v1/crawls/${crawlId}/urls/${urlId}/inlinks`));
}

export interface OutlinkRow {
  target_url_id: string;
  target_url: string;
  anchor_text: string | null;
  link_type: string | null;
}

export async function getOutlinks(crawlId: string, urlId: string): Promise<OutlinkRow[]> {
  return json<OutlinkRow[]>(await authedFetch(`/v1/crawls/${crawlId}/urls/${urlId}/outlinks`));
}

export interface ImageRow {
  id: string;
  url: string;
  status_code: number | null;
  content_type: string | null;
  size_bytes: number | null;
  response_time_ms: number | null;
  alt_text: string | null;
}

export async function getImages(crawlId: string, urlId: string): Promise<ImageRow[]> {
  return json<ImageRow[]>(await authedFetch(`/v1/crawls/${crawlId}/urls/${urlId}/images`));
}

export interface ResourceRow {
  url: string;
  type: string;
}

export interface ResourcesPayload {
  url: string;
  count: number;
  counts_by_type: Record<string, number>;
  resources: ResourceRow[];
}

export async function getResources(crawlId: string, urlId: string): Promise<ResourcesPayload> {
  return json<ResourcesPayload>(
    await authedFetch(`/v1/crawls/${crawlId}/urls/${urlId}/resources`),
  );
}

export interface SerpMetric {
  value: string | null;
  length_chars: number;
  length_pixels: number;
  max_chars: number;
  max_pixels: number;
  remaining_chars: number;
  remaining_pixels: number;
  truncated: boolean;
}

export interface SerpPayload {
  url: string;
  breadcrumb: string[];
  title: SerpMetric;
  description: SerpMetric;
}

export async function getSerp(crawlId: string, urlId: string): Promise<SerpPayload> {
  return json<SerpPayload>(await authedFetch(`/v1/crawls/${crawlId}/urls/${urlId}/serp`));
}

export interface HeaderRow {
  name: string;
  value: string;
}

export interface HeadersPayload {
  url: string;
  final_url: string | null;
  status_code: number | null;
  header_count: number;
  headers: HeaderRow[];
}

export async function getHeaders(crawlId: string, urlId: string): Promise<HeadersPayload> {
  return json<HeadersPayload>(await authedFetch(`/v1/crawls/${crawlId}/urls/${urlId}/headers`));
}

export interface CookieRow {
  name: string;
  value: string;
  domain: string | null;
  path: string | null;
  expires: string | null;
  max_age: number | null;
  secure: boolean;
  http_only: boolean;
  same_site: string | null;
  raw: string;
}

export interface CookiesPayload {
  url: string;
  count: number;
  cookies: CookieRow[];
}

export async function getCookies(crawlId: string, urlId: string): Promise<CookiesPayload> {
  return json<CookiesPayload>(await authedFetch(`/v1/crawls/${crawlId}/urls/${urlId}/cookies`));
}

export interface SourcePayload {
  url: string;
  content_type: string | null;
  content_length: number | null;
  html_length: number;
  content_hash: string | null;
  html: string | null;
}

export async function getSource(crawlId: string, urlId: string): Promise<SourcePayload> {
  return json<SourcePayload>(await authedFetch(`/v1/crawls/${crawlId}/urls/${urlId}/source`));
}

export interface DuplicateRow {
  url_id: string;
  url: string;
  similarity: number;
}

export interface DuplicatesPayload {
  url: string;
  content_hash: string | null;
  match_type: string;
  count: number;
  duplicates: DuplicateRow[];
}

export async function getDuplicates(crawlId: string, urlId: string): Promise<DuplicatesPayload> {
  return json<DuplicatesPayload>(
    await authedFetch(`/v1/crawls/${crawlId}/urls/${urlId}/duplicates`),
  );
}

export interface StructuredDataPayload {
  url: string;
  count?: number;
  by_type?: Record<string, number>;
  items: unknown[];
}

export async function getStructuredData(
  crawlId: string,
  urlId: string,
): Promise<StructuredDataPayload> {
  return json<StructuredDataPayload>(
    await authedFetch(`/v1/crawls/${crawlId}/urls/${urlId}/structured-data`),
  );
}
