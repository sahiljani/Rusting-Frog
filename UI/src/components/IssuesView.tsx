import { useEffect, useMemo, useState } from 'react';
import { Download, AlertCircle, AlertTriangle, Lightbulb, Minus } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';
import { cn } from '@/lib/utils';
import {
  getIssues,
  type IssueItem,
  type IssuesPayload,
  type IssuePriority,
  type IssueType,
} from '@/api';

interface Props {
  crawlId: string;
  refreshKey: number;
}

type SortKey = 'name' | 'type' | 'priority' | 'urls' | 'percent';
type SortDir = 'asc' | 'desc';

const TYPE_RANK: Record<IssueType, number> = { Issue: 0, Warning: 1, Opportunity: 2 };
const PRIORITY_RANK: Record<string, number> = { High: 0, Medium: 1, Low: 2 };

function typeVariant(t: IssueType): 'issue' | 'warning' | 'opportunity' {
  if (t === 'Issue') return 'issue';
  if (t === 'Warning') return 'warning';
  return 'opportunity';
}

function TypeIcon({ type }: { type: IssueType }) {
  if (type === 'Issue')
    return <AlertCircle className="h-3.5 w-3.5 text-severity-issue" aria-hidden />;
  if (type === 'Warning')
    return <AlertTriangle className="h-3.5 w-3.5 text-severity-warning" aria-hidden />;
  return <Lightbulb className="h-3.5 w-3.5 text-severity-opportunity" aria-hidden />;
}

function priorityDot(p: IssuePriority | null): string {
  if (p === 'High') return 'bg-severity-issue';
  if (p === 'Medium') return 'bg-severity-warning';
  if (p === 'Low') return 'bg-severity-opportunity';
  return 'bg-severity-stat';
}

function csvEscape(v: string | number | null): string {
  if (v == null) return '';
  const s = String(v);
  if (/[",\n]/.test(s)) return `"${s.replace(/"/g, '""')}"`;
  return s;
}

function buildCsv(items: IssueItem[]): string {
  const header = ['Issue Name', 'Issue Type', 'Issue Priority', 'URLs', '% of Total'];
  const lines = [header.join(',')];
  for (const it of items) {
    lines.push(
      [
        csvEscape(it.issue_name),
        csvEscape(it.issue_type),
        csvEscape(it.priority),
        csvEscape(it.urls),
        csvEscape(it.percent_of_total),
      ].join(','),
    );
  }
  return lines.join('\r\n');
}

function downloadCsv(filename: string, csv: string) {
  const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

export function IssuesView({ crawlId, refreshKey }: Props) {
  const [payload, setPayload] = useState<IssuesPayload | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [sortKey, setSortKey] = useState<SortKey>('priority');
  const [sortDir, setSortDir] = useState<SortDir>('asc');

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    getIssues(crawlId)
      .then((p) => {
        if (cancelled) return;
        setPayload(p);
        if (p.items.length > 0 && (!selected || !p.items.find((i) => i.filter_key === selected))) {
          setSelected(p.items[0].filter_key);
        }
      })
      .catch((e) => {
        if (!cancelled) setError((e as Error).message);
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [crawlId, refreshKey]);

  const items = payload?.items ?? [];
  const summary = payload?.summary;

  const sorted = useMemo(() => {
    const arr = [...items];
    const dir = sortDir === 'asc' ? 1 : -1;
    arr.sort((a, b) => {
      switch (sortKey) {
        case 'name':
          return dir * a.issue_name.localeCompare(b.issue_name);
        case 'type':
          return dir * (TYPE_RANK[a.issue_type] - TYPE_RANK[b.issue_type]);
        case 'priority': {
          const pa = a.priority ? PRIORITY_RANK[a.priority] : 99;
          const pb = b.priority ? PRIORITY_RANK[b.priority] : 99;
          return dir * (pa - pb);
        }
        case 'urls':
          return dir * (a.urls - b.urls);
        case 'percent':
          return dir * (a.percent_of_total - b.percent_of_total);
      }
    });
    return arr;
  }, [items, sortKey, sortDir]);

  const selectedItem = useMemo(
    () => sorted.find((i) => i.filter_key === selected) ?? sorted[0] ?? null,
    [sorted, selected],
  );

  const onSort = (k: SortKey) => {
    if (sortKey === k) {
      setSortDir((d) => (d === 'asc' ? 'desc' : 'asc'));
    } else {
      setSortKey(k);
      setSortDir(k === 'name' ? 'asc' : 'desc');
    }
  };

  const sortIndicator = (k: SortKey) => (sortKey === k ? (sortDir === 'asc' ? ' ▲' : ' ▼') : '');

  const onExport = () => {
    if (sorted.length === 0) return;
    const stamp = new Date().toISOString().slice(0, 19).replace(/[:T]/g, '-');
    downloadCsv(`issues-${crawlId.slice(0, 8)}-${stamp}.csv`, buildCsv(sorted));
  };

  return (
    <section className="flex min-w-0 flex-1 flex-col overflow-hidden bg-background">
      {/* Header: export + summary chips */}
      <div className="flex items-center justify-between gap-3 border-b border-border px-3 py-1.5">
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={onExport}
            disabled={sorted.length === 0}
            className="inline-flex items-center gap-1.5 rounded border border-border bg-background px-2 py-1 text-[11px] font-medium text-foreground hover:bg-accent disabled:cursor-not-allowed disabled:opacity-50"
          >
            <Download className="h-3 w-3" />
            Export
          </button>
          <div className="text-[11px] text-muted-foreground">
            {loading
              ? 'Loading…'
              : summary
                ? `${sorted.length} row${sorted.length === 1 ? '' : 's'}`
                : ''}
          </div>
        </div>
        <div className="flex items-center gap-1.5">
          <SummaryChip label="Issues" value={summary?.issues ?? 0} variant="issue" />
          <SummaryChip label="Warnings" value={summary?.warnings ?? 0} variant="warning" />
          <SummaryChip
            label="Opportunities"
            value={summary?.opportunities ?? 0}
            variant="opportunity"
          />
          <SummaryChip label="Total" value={summary?.total ?? 0} variant="total" />
        </div>
      </div>

      {error && (
        <div className="border-b border-destructive/30 bg-destructive/10 px-3 py-1 text-[11px] text-destructive">
          {error}
        </div>
      )}

      {/* Grid */}
      <div className="min-h-0 flex-[3] overflow-auto">
        <table className="sf-grid">
          <thead>
            <tr>
              <th
                className="w-[44%] cursor-pointer select-none"
                onClick={() => onSort('name')}
              >
                Issue Name{sortIndicator('name')}
              </th>
              <th className="cursor-pointer select-none" onClick={() => onSort('type')}>
                Issue Type{sortIndicator('type')}
              </th>
              <th className="cursor-pointer select-none" onClick={() => onSort('priority')}>
                Issue Priority{sortIndicator('priority')}
              </th>
              <th className="cursor-pointer select-none text-right" onClick={() => onSort('urls')}>
                URLs{sortIndicator('urls')}
              </th>
              <th
                className="cursor-pointer select-none text-right"
                onClick={() => onSort('percent')}
              >
                % of Total{sortIndicator('percent')}
              </th>
            </tr>
          </thead>
          <tbody>
            {sorted.map((it) => (
              <tr
                key={it.filter_key}
                data-selected={selected === it.filter_key}
                onClick={() => setSelected(it.filter_key)}
                className={cn(
                  selected === it.filter_key &&
                    '!bg-emerald-500/80 !text-white hover:!bg-emerald-500/80',
                )}
              >
                <td
                  className={cn(
                    'truncate',
                    selected === it.filter_key && '!bg-emerald-500/80 !text-white',
                  )}
                  title={it.issue_name}
                >
                  {it.issue_name}
                </td>
                <td
                  className={cn(selected === it.filter_key && '!bg-emerald-500/80 !text-white')}
                >
                  <span className="inline-flex items-center gap-1.5">
                    <TypeIcon type={it.issue_type} />
                    {selected === it.filter_key ? (
                      <span className="text-[11px] font-medium">{it.issue_type}</span>
                    ) : (
                      <Badge variant={typeVariant(it.issue_type)}>{it.issue_type}</Badge>
                    )}
                  </span>
                </td>
                <td
                  className={cn(selected === it.filter_key && '!bg-emerald-500/80 !text-white')}
                >
                  <span className="inline-flex items-center gap-1.5">
                    <span className={cn('h-1.5 w-1.5 rounded-full', priorityDot(it.priority))} />
                    <span className="text-[11px]">{it.priority ?? '—'}</span>
                  </span>
                </td>
                <td
                  className={cn(
                    'text-right tabular-nums',
                    selected === it.filter_key && '!bg-emerald-500/80 !text-white',
                  )}
                >
                  {it.urls}
                </td>
                <td
                  className={cn(
                    'text-right tabular-nums',
                    selected === it.filter_key && '!bg-emerald-500/80 !text-white',
                  )}
                >
                  {it.percent_of_total.toFixed(2)}%
                </td>
              </tr>
            ))}
            {!loading && sorted.length === 0 && (
              <tr>
                <td colSpan={5} className="py-6 text-center text-muted-foreground">
                  No issues detected for this crawl yet.
                </td>
              </tr>
            )}
            {loading && sorted.length === 0 && (
              <tr>
                <td colSpan={5} className="py-6 text-center text-muted-foreground">
                  Loading issues…
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      {/* Issue Details pane */}
      <div className="flex min-h-[180px] flex-[2] flex-col border-t border-border">
        <div className="flex items-center justify-between border-b border-border bg-muted/40 px-3 py-1.5">
          <div className="text-[11px] font-semibold uppercase tracking-wide text-foreground">
            Issue Details
          </div>
          {selectedItem && (
            <code className="font-mono text-[10px] text-muted-foreground">
              {selectedItem.filter_key}
            </code>
          )}
        </div>
        <ScrollArea className="flex-1">
          <div className="space-y-4 px-4 py-3">
            {selectedItem ? (
              <>
                <div className="flex items-center gap-2">
                  <Minus className="h-3 w-3 text-muted-foreground" />
                  <span className="text-sm font-semibold text-foreground">
                    {selectedItem.issue_name}
                  </span>
                </div>
                <Section heading="Description" body={selectedItem.description} />
                <Section heading="How To Fix" body={selectedItem.how_to_fix} />
              </>
            ) : (
              <div className="text-xs text-muted-foreground">
                Select an issue to see its description and remediation guidance.
              </div>
            )}
          </div>
        </ScrollArea>
      </div>
    </section>
  );
}

function SummaryChip({
  label,
  value,
  variant,
}: {
  label: string;
  value: number;
  variant: 'issue' | 'warning' | 'opportunity' | 'total';
}) {
  const tone: Record<typeof variant, string> = {
    issue: 'border-severity-issue/30 bg-severity-issue/10 text-severity-issue',
    warning: 'border-severity-warning/30 bg-severity-warning/10 text-severity-warning',
    opportunity:
      'border-severity-opportunity/30 bg-severity-opportunity/10 text-severity-opportunity',
    total: 'border-emerald-500/30 bg-emerald-500/10 text-emerald-700',
  };
  return (
    <span
      className={cn(
        'inline-flex items-center gap-1.5 rounded-md border px-2 py-0.5 text-[11px] font-medium tabular-nums',
        tone[variant],
      )}
    >
      <span>{label}:</span>
      <span className="font-semibold">{value}</span>
    </span>
  );
}

function Section({ heading, body }: { heading: string; body: string }) {
  return (
    <div>
      <h3 className="mb-1.5 text-[13px] font-semibold text-foreground">{heading}</h3>
      <p className="whitespace-pre-wrap text-[12px] leading-relaxed text-muted-foreground">
        {body || '—'}
      </p>
    </div>
  );
}
