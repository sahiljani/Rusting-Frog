import { ChevronDown, ChevronRight } from 'lucide-react';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Badge } from '@/components/ui/badge';
import type { TabDef, OverviewCounts } from '@/api';
import { cn } from '@/lib/utils';

export type FilterSel = { tabKey: string; filterKey: string } | null;

interface Props {
  tabs: TabDef[];
  overview: OverviewCounts;
  expanded: Record<string, boolean>;
  setExpanded: (fn: (x: Record<string, boolean>) => Record<string, boolean>) => void;
  sel: FilterSel;
  onSelect: (sel: FilterSel) => void;
  tabTotals: Record<string, number>;
}

function severityVariant(sev: string): 'issue' | 'warning' | 'opportunity' | 'stat' {
  if (sev === 'issue') return 'issue';
  if (sev === 'warning') return 'warning';
  if (sev === 'opportunity') return 'opportunity';
  return 'stat';
}

export function Sidebar({
  tabs,
  overview,
  expanded,
  setExpanded,
  sel,
  onSelect,
  tabTotals,
}: Props) {
  return (
    <aside className="flex h-full w-[280px] shrink-0 flex-col border-r border-border bg-background">
      <div className="border-b border-border px-3 py-1.5 text-[10px] font-semibold uppercase tracking-widest text-muted-foreground">
        Overview
      </div>
      <ScrollArea className="flex-1">
        {tabs.length === 0 && (
          <div className="p-3 text-xs text-muted-foreground">Loading tabs…</div>
        )}
        <div className="py-1">
          {tabs.map((t) => {
            const total = tabTotals[t.key] ?? 0;
            const open = expanded[t.key] ?? total > 0;
            return (
              <div key={t.key} className="select-none">
                <button
                  type="button"
                  className="flex w-full items-center justify-between gap-2 px-3 py-1 text-left text-[12px] font-medium hover:bg-accent hover:text-accent-foreground"
                  onClick={() => setExpanded((x) => ({ ...x, [t.key]: !open }))}
                >
                  <span className="flex items-center gap-1 truncate">
                    {open ? (
                      <ChevronDown className="h-3 w-3 shrink-0 text-muted-foreground" />
                    ) : (
                      <ChevronRight className="h-3 w-3 shrink-0 text-muted-foreground" />
                    )}
                    <span className="truncate">{t.display_name}</span>
                  </span>
                  {total > 0 && (
                    <Badge variant="stat" className="tabular-nums">
                      {total}
                    </Badge>
                  )}
                </button>
                {open && (
                  <div className="pb-1">
                    {t.filters.map((f) => {
                      const n = overview[f.key] ?? 0;
                      const active = sel?.filterKey === f.key;
                      return (
                        <button
                          type="button"
                          key={f.key}
                          title={f.key}
                          onClick={() => onSelect({ tabKey: t.key, filterKey: f.key })}
                          className={cn(
                            'flex w-full items-center justify-between gap-2 py-0.5 pl-8 pr-3 text-left text-[11px]',
                            active
                              ? 'bg-primary/10 text-primary'
                              : n === 0
                                ? 'text-muted-foreground/70 hover:bg-accent hover:text-accent-foreground'
                                : 'text-foreground hover:bg-accent hover:text-accent-foreground',
                          )}
                        >
                          <span className="flex min-w-0 items-center gap-1.5">
                            <span
                              className={cn(
                                'h-1.5 w-1.5 shrink-0 rounded-full',
                                f.severity === 'issue' && 'bg-severity-issue',
                                f.severity === 'warning' && 'bg-severity-warning',
                                f.severity === 'opportunity' && 'bg-severity-opportunity',
                                (!f.severity ||
                                  !['issue', 'warning', 'opportunity'].includes(f.severity)) &&
                                  'bg-severity-stat',
                              )}
                            />
                            <span className="truncate">{f.display_name}</span>
                          </span>
                          <Badge
                            variant={n > 0 ? severityVariant(f.severity) : 'outline'}
                            className="tabular-nums"
                          >
                            {n}
                          </Badge>
                        </button>
                      );
                    })}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      </ScrollArea>
    </aside>
  );
}
