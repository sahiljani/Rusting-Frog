import type { CrawlUrlRow } from '@/api';
import { cn } from '@/lib/utils';

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

interface Props {
  rows: CrawlUrlRow[];
  loading: boolean;
  selectedId: string | null;
  onSelect: (id: string) => void;
  title: string;
  subtitle: string;
  filterKey: string;
}

export function DataGrid({
  rows,
  loading,
  selectedId,
  onSelect,
  title,
  subtitle,
  filterKey,
}: Props) {
  return (
    <section className="flex min-w-0 flex-1 flex-col overflow-hidden bg-background">
      <div className="flex items-center justify-between gap-3 border-b border-border px-3 py-1.5">
        <div className="min-w-0">
          <div className="truncate text-xs font-semibold text-foreground">{title}</div>
          <div className="text-[11px] text-muted-foreground">{subtitle}</div>
        </div>
        <code className="shrink-0 font-mono text-[10px] text-muted-foreground">{filterKey}</code>
      </div>
      <div className="min-h-0 flex-1 overflow-auto">
        <table className="sf-grid">
          <thead>
            <tr>
              <th className="w-[40%]">URL</th>
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
                data-selected={selectedId === r.id}
                onClick={() => onSelect(r.id)}
              >
                <td>
                  <span className="sf-url" title={r.url}>
                    {r.url}
                  </span>
                </td>
                <td className={cn('tabular-nums', statusClass(r.status_code))}>
                  {r.status_code ?? '—'}
                </td>
                <td className="text-muted-foreground">{r.content_type ?? '—'}</td>
                <td className="max-w-[320px] truncate" title={r.title ?? ''}>
                  {r.title ?? '—'}
                </td>
                <td className="tabular-nums">{fmt(r.title_length)}</td>
                <td className="max-w-[240px] truncate" title={r.h1_first ?? ''}>
                  {r.h1_first ?? '—'}
                </td>
                <td className="tabular-nums">{fmt(r.word_count)}</td>
                <td className="tabular-nums">{fmt(r.depth)}</td>
                <td className="tabular-nums">{fmt(r.response_time_ms)}</td>
              </tr>
            ))}
            {!loading && rows.length === 0 && (
              <tr>
                <td colSpan={9} className="py-6 text-center text-muted-foreground">
                  No URLs match this filter.
                </td>
              </tr>
            )}
            {loading && (
              <tr>
                <td colSpan={9} className="py-6 text-center text-muted-foreground">
                  Loading…
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </section>
  );
}
