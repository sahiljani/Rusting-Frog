import { useEffect, useMemo, useRef, useState } from 'react';
import { Separator } from '@/components/ui/separator';
import type { CrawlStatus } from '@/api';
import { cn } from '@/lib/utils';

interface Props {
  crawl: CrawlStatus | null;
}

const VICTORIA_TZ = 'America/Vancouver';

const buildLabel = (() => {
  const d = new Date(__APP_BUILD_TIME__);
  if (Number.isNaN(d.getTime())) return '';
  const fmt = new Intl.DateTimeFormat('en-US', {
    timeZone: VICTORIA_TZ,
    month: 'short',
    day: 'numeric',
    hour: 'numeric',
    minute: '2-digit',
    hour12: true,
    timeZoneName: 'short',
  });
  return fmt.format(d);
})();

function statusColor(status?: string): string {
  switch (status) {
    case 'running':
      return 'bg-emerald-500';
    case 'queued':
      return 'bg-sky-500';
    case 'paused':
      return 'bg-amber-500';
    case 'failed':
      return 'bg-destructive';
    case 'completed':
      return 'bg-muted-foreground';
    default:
      return 'bg-border';
  }
}

/**
 * Bottom status strip. Derives rate (URLs/s) from a tiny rolling sample of
 * crawled-count observations so the footer shows current throughput rather
 * than just a total-average.
 */
export function StatusFooter({ crawl }: Props) {
  const samplesRef = useRef<Array<{ t: number; count: number }>>([]);
  const [rate, setRate] = useState(0);

  useEffect(() => {
    if (!crawl) {
      samplesRef.current = [];
      setRate(0);
      return;
    }
    const now = Date.now();
    const count = crawl.urls_crawled ?? 0;
    samplesRef.current.push({ t: now, count });
    // Keep at most the last 15s of samples.
    samplesRef.current = samplesRef.current.filter((s) => now - s.t <= 15_000);
    const s = samplesRef.current;
    if (s.length >= 2) {
      const first = s[0]!;
      const last = s[s.length - 1]!;
      const dt = (last.t - first.t) / 1000;
      const dc = last.count - first.count;
      setRate(dt > 0 ? dc / dt : 0);
    }
  }, [crawl]);

  const progressPct = useMemo(() => {
    if (!crawl) return 0;
    const c = crawl.urls_crawled ?? 0;
    const d = crawl.urls_discovered ?? 0;
    if (!d) return 0;
    return Math.min(100, Math.round((c / d) * 100));
  }, [crawl]);

  const label = crawl?.status ?? 'idle';
  const crawled = crawl?.urls_crawled ?? 0;
  const discovered = crawl?.urls_discovered ?? 0;

  return (
    <footer className="flex h-6 items-center gap-3 border-t border-border bg-muted/40 px-3 text-[11px] tabular-nums text-muted-foreground">
      <span className="inline-flex items-center gap-1.5">
        <span className={cn('h-2 w-2 rounded-full', statusColor(label))} />
        <span className="font-medium uppercase tracking-wide">{label}</span>
      </span>
      <Separator orientation="vertical" className="h-3" />
      <span>
        <b className="text-foreground">{crawled}</b>
        {discovered > 0 && (
          <>
            <span>/</span>
            <b className="text-foreground">{discovered}</b>
          </>
        )}{' '}
        URLs
      </span>
      <Separator orientation="vertical" className="h-3" />
      <span>{rate.toFixed(1)} URL/s</span>
      {crawl?.started_at && (
        <>
          <Separator orientation="vertical" className="h-3" />
          <span>started {new Date(crawl.started_at).toLocaleTimeString()}</span>
        </>
      )}
      <div className="ml-auto flex items-center gap-3">
        {discovered > 0 && (
          <>
            <span>{progressPct}%</span>
            <div className="h-1 w-40 overflow-hidden rounded-full bg-border">
              <div
                className="h-full bg-primary transition-all"
                style={{ width: `${progressPct}%` }}
              />
            </div>
            <Separator orientation="vertical" className="h-3" />
          </>
        )}
        <span
          className="text-muted-foreground/80"
          title={`Build ${__APP_BUILD_TIME__}`}
        >
          v{__APP_VERSION__}
          {buildLabel && <span> · {buildLabel}</span>}
        </span>
      </div>
    </footer>
  );
}
