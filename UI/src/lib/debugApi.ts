// Thin wrapper over /v1/debug/* endpoints. The auth-bearing fetch
// helper lives in api.ts; for now the debug page is dev-only and goes
// over the same Vite proxy so we just call fetch directly with the
// Bearer token if one is in localStorage.

const API_BASE = '';

function authHeaders(): HeadersInit {
  try {
    const token = localStorage.getItem('sf-clone.jwt');
    return token ? { Authorization: `Bearer ${token}` } : {};
  } catch {
    return {};
  }
}

export interface DebugCrawl {
  crawl_id: string;
  seed_url: string | null;
  started_at: string | null;
  ended_at: string | null;
  status: string | null;
  log_bytes: number;
  rotated_files: number;
}

export async function listDebugCrawls(): Promise<DebugCrawl[]> {
  const res = await fetch(`${API_BASE}/v1/debug/crawls`, { headers: authHeaders() });
  if (!res.ok) throw new Error(`debug/crawls ${res.status}`);
  return res.json();
}

export interface DebugLogEvent {
  t?: string;
  kind?: 'phase' | 'sample' | 'log' | string;
  phase?: string;
  url?: string;
  ms?: number;
  ok?: boolean;
  level?: string;
  message?: string;
  meta?: Record<string, unknown>;
  // sample fields
  worker_rss_bytes?: number;
  host_mem_used_bytes?: number;
  host_mem_total_bytes?: number;
  host_disk_used_bytes?: number;
  host_disk_total_bytes?: number;
  pg_db_bytes?: number | null;
  urls_done?: number;
  urls_queued?: number;
}

export interface LogPage {
  lines: DebugLogEvent[];
  next_cursor: number | null;
  file_size_bytes: number;
  is_running: boolean;
}

export async function readLog(
  crawlId: string,
  cursor = 0,
  limit = 500,
  order: 'asc' | 'desc' = 'desc',
): Promise<LogPage> {
  const u = new URL(`${API_BASE}/v1/debug/crawls/${crawlId}/log`, window.location.origin);
  u.searchParams.set('cursor', String(cursor));
  u.searchParams.set('limit', String(limit));
  u.searchParams.set('order', order);
  const res = await fetch(u.toString(), { headers: authHeaders() });
  if (!res.ok) throw new Error(`debug/log ${res.status}`);
  return res.json();
}

export interface TailResponse {
  lines: DebugLogEvent[];
  next_since: number;
  is_running: boolean;
}

export async function tailLog(crawlId: string, since: number): Promise<TailResponse> {
  const u = new URL(`${API_BASE}/v1/debug/crawls/${crawlId}/tail`, window.location.origin);
  u.searchParams.set('since', String(since));
  const res = await fetch(u.toString(), { headers: authHeaders() });
  if (!res.ok) throw new Error(`debug/tail ${res.status}`);
  return res.json();
}

export interface PhaseStats {
  count: number;
  total_ms: number;
  avg_ms: number;
  p95_ms: number;
}

export interface DebugSummary {
  phases: Record<string, PhaseStats>;
  totals: {
    wall_ms: number;
    urls_crawled: number;
    bytes_downloaded: number;
    peak_worker_rss_bytes: number;
    peak_host_mem_used_bytes: number;
    pg_db_growth_bytes: number;
  };
  is_running: boolean;
}

export async function getSummary(crawlId: string): Promise<DebugSummary> {
  const res = await fetch(`${API_BASE}/v1/debug/crawls/${crawlId}/summary`, {
    headers: authHeaders(),
  });
  if (!res.ok) throw new Error(`debug/summary ${res.status}`);
  return res.json();
}

export function downloadUrl(crawlId: string): string {
  return `${API_BASE}/v1/debug/crawls/${crawlId}/log.jsonl`;
}
