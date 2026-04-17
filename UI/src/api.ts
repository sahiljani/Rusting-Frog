const API_BASE = ''; // via Vite proxy — same-origin /v1/*

let cachedToken: string | null = null;
let cachedExp = 0;

export async function getToken(): Promise<string> {
  const now = Math.floor(Date.now() / 1000);
  if (cachedToken && cachedExp - 60 > now) return cachedToken;
  const r = await fetch(`${API_BASE}/v1/dev/token`);
  if (!r.ok) throw new Error(`/v1/dev/token returned ${r.status}. Is sf-api running with SF_DEV_MODE=1?`);
  const j = await r.json();
  cachedToken = j.token as string;
  cachedExp = now + (j.expires_in ?? 3600);
  return cachedToken;
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
    try { body = await r.text(); } catch {}
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
    })
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
    })
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
}

export interface UrlsPage {
  data: CrawlUrlRow[];
  next_cursor: string | null;
}

export async function listUrls(
  crawlId: string,
  opts: { filter?: string; q?: string; limit?: number } = {}
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
