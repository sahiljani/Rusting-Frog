import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  createProject,
  fetchTabs,
  getCrawl,
  getOverview,
  getUrlDetail,
  listUrls,
  startCrawl,
  stopCrawl,
  CrawlStatus,
  CrawlUrlRow,
  OverviewCounts,
  TabDef,
  UrlDetail,
} from './api';

type FilterSel = { tabKey: string; filterKey: string } | null;

function statusClass(code: number | null): string {
  if (code == null) return 'status-unk';
  if (code >= 200 && code < 300) return 'status-ok';
  if (code >= 300 && code < 400) return 'status-redir';
  return 'status-err';
}

function fmt(n: number | null | undefined): string {
  if (n == null) return '—';
  return String(n);
}

export default function App() {
  const [seedUrl, setSeedUrl] = useState('http://localhost:8080/');
  const [tabs, setTabs] = useState<TabDef[]>([]);
  const [crawl, setCrawl] = useState<CrawlStatus | null>(null);
  const [overview, setOverview] = useState<OverviewCounts>({});
  const [expanded, setExpanded] = useState<Record<string, boolean>>({});
  const [sel, setSel] = useState<FilterSel>(null);
  const [rows, setRows] = useState<CrawlUrlRow[]>([]);
  const [loadingRows, setLoadingRows] = useState(false);
  const [selectedUrlId, setSelectedUrlId] = useState<string | null>(null);
  const [detail, setDetail] = useState<UrlDetail | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const pollRef = useRef<number | null>(null);

  useEffect(() => {
    fetchTabs()
      .then(setTabs)
      .catch((e) => setError(String(e)));
  }, []);

  const stopPolling = useCallback(() => {
    if (pollRef.current != null) {
      window.clearInterval(pollRef.current);
      pollRef.current = null;
    }
  }, []);

  const tick = useCallback(async (crawlId: string) => {
    try {
      const [c, ov] = await Promise.all([getCrawl(crawlId), getOverview(crawlId)]);
      setCrawl(c);
      setOverview(ov);
      if (c.status !== 'running' && c.status !== 'queued') {
        stopPolling();
      }
    } catch (e) {
      // keep polling; transient errors are ok
      console.warn('poll error', e);
    }
  }, [stopPolling]);

  const startPolling = useCallback((crawlId: string) => {
    stopPolling();
    void tick(crawlId);
    pollRef.current = window.setInterval(() => { void tick(crawlId); }, 2000);
  }, [stopPolling, tick]);

  useEffect(() => () => stopPolling(), [stopPolling]);

  async function onStart() {
    setError(null);
    setBusy(true);
    setRows([]);
    setSel(null);
    setSelectedUrlId(null);
    setDetail(null);
    setOverview({});
    try {
      const u = seedUrl.trim();
      if (!u) throw new Error('Enter a URL');
      const name = `Audit ${new Date().toISOString().slice(0, 19)}`;
      const p = await createProject(name, u);
      const cr = await startCrawl(p.id);
      const initial = await getCrawl(cr.id);
      setCrawl(initial);
      startPolling(cr.id);
    } catch (e: any) {
      setError(e?.message ?? String(e));
    } finally {
      setBusy(false);
    }
  }

  async function onStop() {
    if (!crawl) return;
    try {
      await stopCrawl(crawl.id);
      await tick(crawl.id);
    } catch (e: any) {
      setError(e?.message ?? String(e));
    }
  }

  const loadRows = useCallback(async (crawlId: string, filterKey: string) => {
    setLoadingRows(true);
    try {
      const page = await listUrls(crawlId, { filter: filterKey, limit: 500 });
      setRows(page.data);
    } catch (e: any) {
      setError(e?.message ?? String(e));
      setRows([]);
    } finally {
      setLoadingRows(false);
    }
  }, []);

  useEffect(() => {
    if (!crawl || !sel) return;
    void loadRows(crawl.id, sel.filterKey);
  }, [crawl?.id, sel?.filterKey, loadRows, crawl, sel]);

  // Refresh rows when overview changes and a filter is selected
  useEffect(() => {
    if (!crawl || !sel) return;
    if (crawl.status === 'running' || crawl.status === 'queued') {
      void loadRows(crawl.id, sel.filterKey);
    }
  }, [overview, crawl, sel, loadRows]);

  useEffect(() => {
    if (!crawl || !selectedUrlId) { setDetail(null); return; }
    let cancelled = false;
    getUrlDetail(crawl.id, selectedUrlId)
      .then((d) => { if (!cancelled) setDetail(d); })
      .catch((e) => { if (!cancelled) setError(String(e)); });
    return () => { cancelled = true; };
  }, [crawl?.id, selectedUrlId, crawl]);

  const tabTotals = useMemo(() => {
    const out: Record<string, number> = {};
    for (const t of tabs) {
      let total = 0;
      for (const f of t.filters) total += overview[f.key] ?? 0;
      out[t.key] = total;
    }
    return out;
  }, [tabs, overview]);

  const progressPct = useMemo(() => {
    if (!crawl) return 0;
    const c = crawl.urls_crawled ?? 0;
    const d = crawl.urls_discovered ?? 0;
    if (!d) return 0;
    return Math.min(100, Math.round((c / d) * 100));
  }, [crawl]);

  const running = crawl?.status === 'running' || crawl?.status === 'queued';

  return (
    <div className="app">
      <div className="topbar">
        <span className="brand">RUSTING FROG</span>
        <input
          type="text"
          placeholder="https://example.com/"
          value={seedUrl}
          onChange={(e) => setSeedUrl(e.target.value)}
          disabled={busy || running}
          onKeyDown={(e) => { if (e.key === 'Enter' && !busy && !running) onStart(); }}
        />
        {running ? (
          <button onClick={onStop}>Stop</button>
        ) : (
          <button onClick={onStart} disabled={busy}>Start Audit</button>
        )}
        <span className="status">
          {crawl ? (
            <>
              {crawl.status.toUpperCase()} · {fmt(crawl.urls_crawled)}/{fmt(crawl.urls_discovered)} URLs
            </>
          ) : (
            'Idle'
          )}
        </span>
      </div>
      {running && (
        <div className="progress-bar"><div className="fill" style={{ width: `${progressPct}%` }} /></div>
      )}
      {error && (
        <div style={{ padding: '6px 12px', background: '#fde8e6', color: '#9b2316', fontSize: 12 }}>
          {error} <button style={{ marginLeft: 8 }} onClick={() => setError(null)}>dismiss</button>
        </div>
      )}

      <div className="main">
        <aside className="sidebar">
          <h3>Overview</h3>
          {tabs.length === 0 && <div style={{ padding: 12, color: '#9097a0' }}>Loading tabs…</div>}
          {tabs.map((t) => {
            const total = tabTotals[t.key] ?? 0;
            const open = expanded[t.key] ?? (total > 0);
            return (
              <div className="tab-group" key={t.key}>
                <div
                  className="tab-header"
                  onClick={() => setExpanded((x) => ({ ...x, [t.key]: !open }))}
                >
                  <span>{open ? '▾' : '▸'} {t.display_name}</span>
                  <span className={'count' + (total > 0 ? ' hot' : '')}>{total}</span>
                </div>
                {open && (
                  <div className="filter-list">
                    {t.filters.map((f) => {
                      const n = overview[f.key] ?? 0;
                      const active = sel?.filterKey === f.key;
                      return (
                        <div
                          key={f.key}
                          className={
                            'filter-row' +
                            (active ? ' active' : '') +
                            (n === 0 ? ' zero' : '')
                          }
                          onClick={() => {
                            setSel({ tabKey: t.key, filterKey: f.key });
                            setSelectedUrlId(null);
                            setDetail(null);
                          }}
                          title={f.key}
                        >
                          <span className={'sev ' + f.severity} />
                          <span className="name">{f.display_name}</span>
                          <span className="count">{n}</span>
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>
            );
          })}
        </aside>

        <section className="center">
          {!crawl ? (
            <div className="welcome">
              <div className="card">
                <h1>Rusting Frog — SEO Audit</h1>
                <p>Enter a URL above and click <b>Start Audit</b>.</p>
                <p style={{ marginTop: 8, fontSize: 12 }}>
                  Tip: set <code>SF_DEV_MODE=1</code> and <code>SF_ALLOW_PRIVATE_IPS=1</code> on the API so
                  the UI can mint a dev token and crawl localhost fixtures.
                </p>
              </div>
            </div>
          ) : !sel ? (
            <div className="empty">
              <h3>Pick a filter</h3>
              <div>Select any filter from the left to see matching URLs.</div>
            </div>
          ) : (
            <>
              <div className="grid-toolbar">
                <div>
                  <div className="title">
                    {tabs.find((t) => t.key === sel.tabKey)?.display_name} ·{' '}
                    {tabs
                      .find((t) => t.key === sel.tabKey)
                      ?.filters.find((f) => f.key === sel.filterKey)?.display_name}
                  </div>
                  <div className="sub">
                    {loadingRows ? 'Loading…' : `${rows.length} URL${rows.length === 1 ? '' : 's'}`}
                  </div>
                </div>
                <code style={{ fontSize: 11, color: '#5a6068' }}>{sel.filterKey}</code>
              </div>
              <div className="grid-wrap">
                <table className="grid">
                  <thead>
                    <tr>
                      <th style={{ width: '40%' }}>URL</th>
                      <th>Code</th>
                      <th>Type</th>
                      <th>Title</th>
                      <th>T-Len</th>
                      <th>H1</th>
                      <th>Words</th>
                      <th>Depth</th>
                      <th>RT (ms)</th>
                    </tr>
                  </thead>
                  <tbody>
                    {rows.map((r) => (
                      <tr
                        key={r.id}
                        className={selectedUrlId === r.id ? 'selected' : ''}
                        onClick={() => setSelectedUrlId(r.id)}
                      >
                        <td className="url-cell" title={r.url}>{r.url}</td>
                        <td className={statusClass(r.status_code)}>
                          {r.status_code ?? '—'}
                        </td>
                        <td>{r.content_type ?? '—'}</td>
                        <td className="url-cell" title={r.title ?? ''}>{r.title ?? '—'}</td>
                        <td>{fmt(r.title_length)}</td>
                        <td className="url-cell" title={r.h1_first ?? ''}>{r.h1_first ?? '—'}</td>
                        <td>{fmt(r.word_count)}</td>
                        <td>{fmt(r.depth)}</td>
                        <td>{fmt(r.response_time_ms)}</td>
                      </tr>
                    ))}
                    {!loadingRows && rows.length === 0 && (
                      <tr>
                        <td colSpan={9} style={{ textAlign: 'center', padding: 24, color: '#9097a0' }}>
                          No URLs match this filter.
                        </td>
                      </tr>
                    )}
                  </tbody>
                </table>
              </div>
            </>
          )}
        </section>

        <aside className="detail">
          {!detail ? (
            <div style={{ color: '#9097a0' }}>
              {selectedUrlId ? 'Loading…' : 'Select a row to see details.'}
            </div>
          ) : (
            <>
              <h3>{detail.url}</h3>
              <dl>
                <dt>Status</dt><dd>{detail.status_code ?? '—'}</dd>
                <dt>Type</dt><dd>{detail.content_type ?? '—'}</dd>
                <dt>Internal</dt><dd>{detail.is_internal ? 'yes' : 'no'}</dd>
                <dt>Depth</dt><dd>{fmt(detail.depth)}</dd>
                <dt>Size</dt><dd>{fmt(detail.content_length)}</dd>
                <dt>RT</dt><dd>{fmt(detail.response_time_ms)} ms</dd>
                <dt>Crawled</dt><dd>{detail.crawled_at ?? '—'}</dd>
              </dl>

              <h4>Title</h4>
              <div>{detail.title ?? <i>—</i>}</div>
              <div style={{ color: '#5a6068', fontSize: 11, marginTop: 2 }}>
                {fmt(detail.title_length)} chars · {fmt(detail.title_pixel_width)} px
              </div>

              <h4>Meta Description</h4>
              <div>{detail.meta_description ?? <i>—</i>}</div>
              <div style={{ color: '#5a6068', fontSize: 11, marginTop: 2 }}>
                {fmt(detail.meta_description_length)} chars
              </div>

              <h4>Headings</h4>
              <dl>
                <dt>H1</dt><dd>{detail.h1_first ?? '—'} ({fmt(detail.h1_count)})</dd>
                <dt>H2</dt><dd>{detail.h2_first ?? '—'} ({fmt(detail.h2_count)})</dd>
              </dl>

              <h4>Directives</h4>
              <dl>
                <dt>Canonical</dt><dd>{detail.canonical_url ?? '—'}</dd>
                <dt>Robots</dt><dd>{detail.meta_robots ?? '—'}</dd>
                <dt>Redirect</dt><dd>{detail.redirect_url ?? '—'}</dd>
              </dl>

              {detail.findings && detail.findings.length > 0 && (
                <>
                  <h4>Findings ({detail.findings.length})</h4>
                  <div>
                    {detail.findings.map((f) => (
                      <span key={f} className="finding-pill">{f}</span>
                    ))}
                  </div>
                </>
              )}
            </>
          )}
        </aside>
      </div>
    </div>
  );
}
