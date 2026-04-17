import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { TooltipProvider } from '@/components/ui/tooltip';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { TopMenuBar } from '@/components/TopMenuBar';
import { CrawlBar } from '@/components/CrawlBar';
import { Sidebar, type FilterSel } from '@/components/Sidebar';
import { DataGrid } from '@/components/DataGrid';
import { DetailPane } from '@/components/DetailPane';
import { StatusFooter } from '@/components/StatusFooter';
import {
  clearToken,
  createProject,
  fetchTabs,
  getCrawl,
  getOverview,
  listUrls,
  pauseCrawl,
  resumeCrawl,
  startCrawl,
  stopCrawl,
  type CrawlStatus,
  type CrawlUrlRow,
  type OverviewCounts,
  type TabDef,
} from '@/api';

export default function App() {
  const [seedUrl, setSeedUrl] = useState('https://example.com/');
  const [tabs, setTabs] = useState<TabDef[]>([]);
  const [crawl, setCrawl] = useState<CrawlStatus | null>(null);
  const [overview, setOverview] = useState<OverviewCounts>({});
  const [expanded, setExpanded] = useState<Record<string, boolean>>({});
  const [sel, setSel] = useState<FilterSel>(null);
  const [rows, setRows] = useState<CrawlUrlRow[]>([]);
  const [loadingRows, setLoadingRows] = useState(false);
  const [selectedUrlId, setSelectedUrlId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [configOpen, setConfigOpen] = useState(false);
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

  const tick = useCallback(
    async (crawlId: string) => {
      try {
        const [c, ov] = await Promise.all([getCrawl(crawlId), getOverview(crawlId)]);
        setCrawl(c);
        setOverview(ov);
        if (c.status !== 'running' && c.status !== 'queued') {
          stopPolling();
        }
      } catch (e) {
        console.warn('poll error', e);
      }
    },
    [stopPolling],
  );

  const startPolling = useCallback(
    (crawlId: string) => {
      stopPolling();
      void tick(crawlId);
      pollRef.current = window.setInterval(() => {
        void tick(crawlId);
      }, 2000);
    },
    [stopPolling, tick],
  );

  useEffect(() => () => stopPolling(), [stopPolling]);

  const onStart = useCallback(async () => {
    setError(null);
    setBusy(true);
    if (!crawl || crawl.status !== 'paused') {
      setRows([]);
      setSel(null);
      setSelectedUrlId(null);
      setOverview({});
    }
    try {
      if (crawl && crawl.status === 'paused') {
        await resumeCrawl(crawl.id);
        startPolling(crawl.id);
        return;
      }
      const u = seedUrl.trim();
      if (!u) throw new Error('Enter a URL');
      const name = `Audit ${new Date().toISOString().slice(0, 19)}`;
      const p = await createProject(name, u);
      const cr = await startCrawl(p.id);
      const initial = await getCrawl(cr.id);
      setCrawl(initial);
      startPolling(cr.id);
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }, [seedUrl, crawl, startPolling]);

  const onPause = useCallback(async () => {
    if (!crawl) return;
    try {
      await pauseCrawl(crawl.id);
      await tick(crawl.id);
    } catch (e) {
      setError((e as Error).message);
    }
  }, [crawl, tick]);

  const onStop = useCallback(async () => {
    if (!crawl) return;
    try {
      await stopCrawl(crawl.id);
      await tick(crawl.id);
    } catch (e) {
      setError((e as Error).message);
    }
  }, [crawl, tick]);

  const loadRows = useCallback(async (crawlId: string, filterKey: string) => {
    setLoadingRows(true);
    try {
      const page = await listUrls(crawlId, { filter: filterKey, limit: 500 });
      setRows(page.data);
    } catch (e) {
      setError((e as Error).message);
      setRows([]);
    } finally {
      setLoadingRows(false);
    }
  }, []);

  useEffect(() => {
    if (!crawl || !sel) return;
    void loadRows(crawl.id, sel.filterKey);
  }, [crawl?.id, sel?.filterKey, loadRows, crawl, sel]);

  // Refresh rows when overview changes mid-crawl
  useEffect(() => {
    if (!crawl || !sel) return;
    if (crawl.status === 'running' || crawl.status === 'queued') {
      void loadRows(crawl.id, sel.filterKey);
    }
  }, [overview, crawl, sel, loadRows]);

  const tabTotals = useMemo(() => {
    const out: Record<string, number> = {};
    for (const t of tabs) {
      let total = 0;
      for (const f of t.filters) total += overview[f.key] ?? 0;
      out[t.key] = total;
    }
    return out;
  }, [tabs, overview]);

  const running = crawl?.status === 'running' || crawl?.status === 'queued';
  const paused = crawl?.status === 'paused';

  const selectedTab = tabs.find((t) => t.key === sel?.tabKey);
  const selectedFilter = selectedTab?.filters.find((f) => f.key === sel?.filterKey);

  const onMenuCommand = useCallback(
    (id: string) => {
      switch (id) {
        case 'file.new':
          setCrawl(null);
          setRows([]);
          setSel(null);
          setSelectedUrlId(null);
          setOverview({});
          break;
        case 'file.clear_token':
          clearToken();
          setError('Saved token cleared. A fresh one will mint on next request.');
          break;
        case 'mode.spider':
          break; // already in Spider mode
        case 'config.open':
          setConfigOpen(true);
          break;
      }
    },
    [],
  );

  return (
    <TooltipProvider delayDuration={200} skipDelayDuration={100}>
      <div className="flex h-full flex-col bg-background text-foreground">
        <TopMenuBar mode="spider" onCommand={onMenuCommand} />
        <CrawlBar
          seedUrl={seedUrl}
          setSeedUrl={setSeedUrl}
          busy={busy}
          running={running}
          paused={paused}
          onStart={onStart}
          onPause={onPause}
          onStop={onStop}
        />
        {error && (
          <div className="flex items-center justify-between gap-2 border-b border-destructive/30 bg-destructive/10 px-3 py-1 text-xs text-destructive">
            <span className="break-all">{error}</span>
            <button
              onClick={() => setError(null)}
              className="shrink-0 rounded px-2 py-0.5 text-[10px] font-medium hover:bg-destructive/20"
            >
              dismiss
            </button>
          </div>
        )}

        <div className="flex min-h-0 flex-1">
          <Sidebar
            tabs={tabs}
            overview={overview}
            expanded={expanded}
            setExpanded={setExpanded}
            sel={sel}
            onSelect={(s) => {
              setSel(s);
              setSelectedUrlId(null);
            }}
            tabTotals={tabTotals}
          />

          {!crawl ? (
            <div className="flex flex-1 items-center justify-center bg-muted/30">
              <div className="max-w-md rounded-lg border border-border bg-background p-6 shadow-sm">
                <h1 className="mb-2 text-lg font-semibold">Rusting Frog — SEO Audit</h1>
                <p className="text-sm text-muted-foreground">
                  Enter a URL in the bar above and click <b className="text-foreground">Start</b> to
                  begin crawling.
                </p>
                <p className="mt-3 text-[11px] text-muted-foreground">
                  Tip: hover any info icon to see Screaming Frog's user-guide text for that field.
                </p>
              </div>
            </div>
          ) : !sel ? (
            <div className="flex flex-1 items-center justify-center bg-muted/30 text-sm text-muted-foreground">
              Pick a filter from the sidebar to see matching URLs.
            </div>
          ) : (
            <DataGrid
              rows={rows}
              loading={loadingRows}
              selectedId={selectedUrlId}
              onSelect={setSelectedUrlId}
              title={`${selectedTab?.display_name ?? ''} · ${selectedFilter?.display_name ?? ''}`}
              subtitle={
                loadingRows
                  ? 'Loading…'
                  : `${rows.length} URL${rows.length === 1 ? '' : 's'}`
              }
              filterKey={sel.filterKey}
            />
          )}

          <DetailPane crawlId={crawl?.id ?? null} urlId={selectedUrlId} />
        </div>

        <StatusFooter crawl={crawl} />

        <Dialog open={configOpen} onOpenChange={setConfigOpen}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Spider Configuration</DialogTitle>
              <DialogDescription>
                Configuration UI is wired in the next batch (K4). For now the crawler uses its
                defaults; all Configuration menu items are placeholders.
              </DialogDescription>
            </DialogHeader>
          </DialogContent>
        </Dialog>
      </div>
    </TooltipProvider>
  );
}
