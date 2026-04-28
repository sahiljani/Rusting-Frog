import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  type DebugCrawl,
  type DebugLogEvent,
  type DebugSummary,
  downloadUrl,
  getSummary,
  listDebugCrawls,
  readLog,
  tailLog,
} from '@/lib/debugApi';
import { formatVictoria, formatVictoriaTime, nowInVictoria, victoriaZoneAbbr } from '@/lib/victoriaTime';

const POLL_MS = 1000;
const PAGE_SIZE = 500;

function getQueryCrawlId(): string | null {
  try {
    const u = new URL(window.location.href);
    return u.searchParams.get('crawl');
  } catch {
    return null;
  }
}

function setQueryCrawlId(id: string | null) {
  try {
    const u = new URL(window.location.href);
    if (id) u.searchParams.set('crawl', id);
    else u.searchParams.delete('crawl');
    window.history.replaceState(null, '', u.toString());
  } catch {
    /* ignore */
  }
}

function fmtBytes(n: number | null | undefined): string {
  if (n == null || !Number.isFinite(n)) return '—';
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`;
  return `${(n / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

function fmtPct(used: number, total: number): string {
  if (!total) return '';
  return `${((used / total) * 100).toFixed(0)}%`;
}

export default function DebugPage() {
  const [crawls, setCrawls] = useState<DebugCrawl[]>([]);
  const [crawlId, setCrawlId] = useState<string | null>(getQueryCrawlId());
  const [summary, setSummary] = useState<DebugSummary | null>(null);
  const [latestSample, setLatestSample] = useState<DebugLogEvent | null>(null);
  const [lines, setLines] = useState<DebugLogEvent[]>([]);
  const [page, setPage] = useState(0);
  const [hasMoreOlder, setHasMoreOlder] = useState(false);
  const [filter, setFilter] = useState<'all' | 'phase' | 'sample' | 'log'>('all');
  const [autoScroll, setAutoScroll] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [now, setNow] = useState<string>(nowInVictoria());
  const [isRunning, setIsRunning] = useState(false);

  const tailOffsetRef = useRef<number>(0);
  const summaryTickRef = useRef<number>(0);
  const logEndRef = useRef<HTMLDivElement>(null);

  // Header clock: tick once a second so the user can confirm the page
  // is alive and on Victoria time.
  useEffect(() => {
    const id = window.setInterval(() => setNow(nowInVictoria()), 1000);
    return () => window.clearInterval(id);
  }, []);

  // Load crawl list on mount.
  useEffect(() => {
    listDebugCrawls()
      .then(setCrawls)
      .catch((e) => setError(String(e)));
  }, []);

  // When crawlId changes, reset state and load page 0 + summary.
  useEffect(() => {
    if (!crawlId) {
      setSummary(null);
      setLines([]);
      setLatestSample(null);
      tailOffsetRef.current = 0;
      return;
    }
    setQueryCrawlId(crawlId);
    setLines([]);
    setPage(0);
    setError(null);
    tailOffsetRef.current = 0;

    Promise.all([readLog(crawlId, 0, PAGE_SIZE, 'desc'), getSummary(crawlId)])
      .then(([page0, sum]) => {
        setLines(page0.lines);
        setHasMoreOlder(page0.next_cursor != null);
        tailOffsetRef.current = page0.file_size_bytes;
        const lastSample = [...page0.lines].reverse().find((l) => l.kind === 'sample');
        if (lastSample) setLatestSample(lastSample);
        setSummary(sum);
        setIsRunning(sum.is_running);
      })
      .catch((e) => setError(String(e)));
  }, [crawlId]);

  // Live polling while the crawl is running.
  useEffect(() => {
    if (!crawlId || !isRunning) return;
    let cancelled = false;
    const id = window.setInterval(async () => {
      try {
        const tail = await tailLog(crawlId, tailOffsetRef.current);
        if (cancelled) return;
        if (tail.lines.length) {
          // New events: prepend (since UI is desc), update sample, advance offset.
          setLines((prev) => [...tail.lines.slice().reverse(), ...prev].slice(0, 5000));
          const sample = [...tail.lines].reverse().find((l) => l.kind === 'sample');
          if (sample) setLatestSample(sample);
        }
        tailOffsetRef.current = tail.next_since;
        setIsRunning(tail.is_running);

        // Refresh summary every 5 polls so phase tables stay current.
        summaryTickRef.current += 1;
        if (summaryTickRef.current >= 5) {
          summaryTickRef.current = 0;
          getSummary(crawlId).then((s) => !cancelled && setSummary(s)).catch(() => {});
        }
      } catch (e) {
        if (!cancelled) setError(String(e));
      }
    }, POLL_MS);
    return () => {
      cancelled = true;
      window.clearInterval(id);
    };
  }, [crawlId, isRunning]);

  // Auto-scroll: only when new events arrive AND the user hasn't paged back.
  useEffect(() => {
    if (!autoScroll) return;
    logEndRef.current?.scrollIntoView({ block: 'nearest' });
  }, [lines, autoScroll]);

  const loadOlder = useCallback(async () => {
    if (!crawlId || !hasMoreOlder) return;
    const nextPage = page + 1;
    try {
      const r = await readLog(crawlId, nextPage * PAGE_SIZE, PAGE_SIZE, 'desc');
      setLines((prev) => [...prev, ...r.lines]);
      setPage(nextPage);
      setHasMoreOlder(r.next_cursor != null);
    } catch (e) {
      setError(String(e));
    }
  }, [crawlId, hasMoreOlder, page]);

  const filteredLines = useMemo(() => {
    if (filter === 'all') return lines;
    return lines.filter((l) => l.kind === filter);
  }, [lines, filter]);

  const selectedCrawl = useMemo(
    () => crawls.find((c) => c.crawl_id === crawlId) ?? null,
    [crawls, crawlId],
  );

  return (
    <div className="flex h-full min-h-screen flex-col bg-background text-foreground font-sans">
      <header className="flex items-center justify-between border-b px-4 py-2">
        <div className="flex items-center gap-3">
          <span className="font-mono text-[11px] font-semibold uppercase tracking-widest text-primary">
            Rusting&nbsp;Frog &middot; Debug
          </span>
        </div>
        <div className="font-mono text-xs text-muted-foreground">
          now: {now} ({victoriaZoneAbbr()})
        </div>
      </header>

      <section className="flex flex-wrap items-center gap-3 border-b px-4 py-2 text-xs">
        <label className="flex items-center gap-2">
          <span className="text-muted-foreground">Crawl:</span>
          <select
            className="rounded border bg-background px-2 py-1 text-xs"
            value={crawlId ?? ''}
            onChange={(e) => setCrawlId(e.target.value || null)}
          >
            <option value="">— pick a crawl —</option>
            {crawls.map((c) => (
              <option key={c.crawl_id} value={c.crawl_id}>
                {(c.seed_url ?? '?')} · {c.status ?? 'unknown'} ·{' '}
                {c.started_at ? formatVictoria(c.started_at) : 'no start'}
              </option>
            ))}
          </select>
        </label>
        {selectedCrawl && (
          <>
            <span className={isRunning ? 'text-green-600' : 'text-muted-foreground'}>
              {isRunning ? '● running' : '○ stopped'}
            </span>
            <span className="text-muted-foreground">
              started {selectedCrawl.started_at ? formatVictoria(selectedCrawl.started_at) : '—'}
            </span>
            <span className="text-muted-foreground">
              log {fmtBytes(selectedCrawl.log_bytes)} ({selectedCrawl.rotated_files} rotated)
            </span>
            <a
              className="rounded border px-2 py-0.5 hover:bg-muted"
              href={downloadUrl(selectedCrawl.crawl_id)}
            >
              download .jsonl
            </a>
          </>
        )}
      </section>

      {error && (
        <div className="border-b border-destructive/30 bg-destructive/10 px-4 py-1 text-xs text-destructive">
          {error}{' '}
          <button onClick={() => setError(null)} className="ml-2 underline">
            dismiss
          </button>
        </div>
      )}

      {crawlId ? (
        <main className="grid flex-1 grid-cols-1 gap-3 p-3 md:grid-cols-2">
          <PhaseTable summary={summary} />
          <ResourcePanel sample={latestSample} totals={summary?.totals ?? null} />
          <div className="md:col-span-2">
            <LogPanel
              lines={filteredLines}
              autoScroll={autoScroll}
              setAutoScroll={setAutoScroll}
              filter={filter}
              setFilter={setFilter}
              onLoadOlder={loadOlder}
              hasMoreOlder={hasMoreOlder}
              page={page}
              endRef={logEndRef}
            />
          </div>
        </main>
      ) : (
        <main className="flex flex-1 items-center justify-center p-6 text-sm text-muted-foreground">
          Pick a crawl above to start. Crawls with debug logs will appear in the dropdown.
        </main>
      )}
    </div>
  );
}

function PhaseTable({ summary }: { summary: DebugSummary | null }) {
  const phases = summary?.phases ?? {};
  const rows = Object.entries(phases).sort((a, b) => b[1].total_ms - a[1].total_ms);
  return (
    <section className="rounded border">
      <header className="border-b px-3 py-2 text-xs font-semibold uppercase tracking-wide">
        Phase timings
      </header>
      <div className="overflow-x-auto">
        <table className="w-full text-xs">
          <thead className="bg-muted text-muted-foreground">
            <tr>
              <th className="px-3 py-1 text-left">phase</th>
              <th className="px-3 py-1 text-right">count</th>
              <th className="px-3 py-1 text-right">total ms</th>
              <th className="px-3 py-1 text-right">avg ms</th>
              <th className="px-3 py-1 text-right">p95 ms</th>
            </tr>
          </thead>
          <tbody>
            {rows.length === 0 && (
              <tr>
                <td colSpan={5} className="px-3 py-3 text-center text-muted-foreground">
                  no phase data yet
                </td>
              </tr>
            )}
            {rows.map(([k, v]) => (
              <tr key={k} className="border-t">
                <td className="px-3 py-1 font-mono">{k}</td>
                <td className="px-3 py-1 text-right">{v.count}</td>
                <td className="px-3 py-1 text-right">{v.total_ms.toLocaleString()}</td>
                <td className="px-3 py-1 text-right">{v.avg_ms.toLocaleString()}</td>
                <td className="px-3 py-1 text-right">{v.p95_ms.toLocaleString()}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}

function ResourcePanel({
  sample,
  totals,
}: {
  sample: DebugLogEvent | null;
  totals: DebugSummary['totals'] | null;
}) {
  return (
    <section className="rounded border">
      <header className="border-b px-3 py-2 text-xs font-semibold uppercase tracking-wide">
        Resources (1s sample)
      </header>
      <dl className="grid grid-cols-2 gap-x-4 gap-y-1 px-3 py-2 text-xs">
        <Row label="Worker RSS" value={fmtBytes(sample?.worker_rss_bytes)} />
        <Row
          label="Host mem"
          value={
            sample?.host_mem_total_bytes
              ? `${fmtBytes(sample.host_mem_used_bytes)} / ${fmtBytes(
                  sample.host_mem_total_bytes,
                )} (${fmtPct(
                  sample.host_mem_used_bytes ?? 0,
                  sample.host_mem_total_bytes ?? 0,
                )})`
              : '—'
          }
        />
        <Row
          label="Host disk"
          value={
            sample?.host_disk_total_bytes
              ? `${fmtBytes(sample.host_disk_used_bytes)} / ${fmtBytes(
                  sample.host_disk_total_bytes,
                )} (${fmtPct(
                  sample.host_disk_used_bytes ?? 0,
                  sample.host_disk_total_bytes ?? 0,
                )})`
              : '—'
          }
        />
        <Row label="Postgres DB" value={fmtBytes(sample?.pg_db_bytes ?? undefined)} />
        <Row
          label="URLs done / queued"
          value={`${sample?.urls_done ?? 0} / ${sample?.urls_queued ?? 0}`}
        />
        <Row
          label="Last sample"
          value={sample?.t ? formatVictoriaTime(sample.t) : '—'}
        />
        {totals && (
          <>
            <Row label="Peak worker RSS" value={fmtBytes(totals.peak_worker_rss_bytes)} />
            <Row label="Bytes downloaded" value={fmtBytes(totals.bytes_downloaded)} />
            <Row label="DB growth" value={fmtBytes(totals.pg_db_growth_bytes)} />
            <Row label="Wall time" value={`${totals.wall_ms.toLocaleString()} ms`} />
          </>
        )}
      </dl>
    </section>
  );
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <>
      <dt className="text-muted-foreground">{label}</dt>
      <dd className="text-right font-mono">{value}</dd>
    </>
  );
}

function LogPanel({
  lines,
  autoScroll,
  setAutoScroll,
  filter,
  setFilter,
  onLoadOlder,
  hasMoreOlder,
  page,
  endRef,
}: {
  lines: DebugLogEvent[];
  autoScroll: boolean;
  setAutoScroll: (v: boolean) => void;
  filter: 'all' | 'phase' | 'sample' | 'log';
  setFilter: (v: 'all' | 'phase' | 'sample' | 'log') => void;
  onLoadOlder: () => void;
  hasMoreOlder: boolean;
  page: number;
  endRef: React.RefObject<HTMLDivElement>;
}) {
  return (
    <section className="rounded border">
      <header className="flex flex-wrap items-center justify-between gap-2 border-b px-3 py-2">
        <div className="text-xs font-semibold uppercase tracking-wide">Logs</div>
        <div className="flex items-center gap-3 text-xs">
          <label className="flex items-center gap-1">
            <span className="text-muted-foreground">filter:</span>
            <select
              className="rounded border bg-background px-1 py-0.5"
              value={filter}
              onChange={(e) => setFilter(e.target.value as typeof filter)}
            >
              <option value="all">all</option>
              <option value="phase">phase only</option>
              <option value="sample">sample only</option>
              <option value="log">log only</option>
            </select>
          </label>
          <label className="flex items-center gap-1">
            <input
              type="checkbox"
              checked={autoScroll}
              onChange={(e) => setAutoScroll(e.target.checked)}
            />
            auto-scroll
          </label>
          <span className="text-muted-foreground">page {page + 1}</span>
          <button
            disabled={!hasMoreOlder}
            onClick={onLoadOlder}
            className="rounded border px-2 py-0.5 disabled:opacity-40 hover:bg-muted"
          >
            load older
          </button>
        </div>
      </header>
      <div className="max-h-[60vh] overflow-y-auto px-3 py-2 font-mono text-[11px] leading-snug">
        {lines.length === 0 && (
          <div className="text-muted-foreground">no log lines yet</div>
        )}
        {lines.map((l, i) => (
          <LogLine key={i} line={l} />
        ))}
        <div ref={endRef} />
      </div>
    </section>
  );
}

function LogLine({ line }: { line: DebugLogEvent }) {
  const t = line.t ? formatVictoriaTime(line.t) : '—';
  if (line.kind === 'phase') {
    const ok = line.ok !== false;
    return (
      <div className={ok ? '' : 'text-destructive'}>
        <span className="text-muted-foreground">{t}</span>{' '}
        <span className="font-semibold">{line.phase}</span>{' '}
        <span>{line.ms ?? 0}ms</span>{' '}
        {line.url && <span className="text-blue-600">{line.url}</span>}{' '}
        <span className="text-muted-foreground">{summarizeMeta(line.meta)}</span>
      </div>
    );
  }
  if (line.kind === 'sample') {
    return (
      <div className="text-muted-foreground">
        <span>{t}</span>{' '}
        <span className="font-semibold">sample</span>{' '}
        rss={fmtBytes(line.worker_rss_bytes)} pg={fmtBytes(line.pg_db_bytes ?? undefined)}{' '}
        done={line.urls_done ?? 0}/{line.urls_queued ?? 0}
      </div>
    );
  }
  if (line.kind === 'log') {
    const cls =
      line.level === 'error'
        ? 'text-destructive'
        : line.level === 'warn'
          ? 'text-yellow-600'
          : '';
    return (
      <div className={cls}>
        <span className="text-muted-foreground">{t}</span>{' '}
        <span className="font-semibold">[{line.level}]</span> {line.message}
      </div>
    );
  }
  return (
    <div>
      <span className="text-muted-foreground">{t}</span> {JSON.stringify(line)}
    </div>
  );
}

function summarizeMeta(meta?: Record<string, unknown>): string {
  if (!meta) return '';
  const entries = Object.entries(meta).slice(0, 4);
  return entries.map(([k, v]) => `${k}=${formatMetaValue(v)}`).join(' ');
}

function formatMetaValue(v: unknown): string {
  if (v == null) return '';
  if (typeof v === 'string') return v.length > 60 ? `${v.slice(0, 60)}…` : v;
  if (typeof v === 'number' || typeof v === 'boolean') return String(v);
  return JSON.stringify(v).slice(0, 60);
}
