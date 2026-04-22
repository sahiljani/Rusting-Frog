import { AlertCircle, ChevronDown, ChevronLeft, ChevronRight } from 'lucide-react';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Badge } from '@/components/ui/badge';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
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
  collapsed: boolean;
  onToggleCollapsed: () => void;
  issuesActive: boolean;
  onSelectIssues: () => void;
  issuesTotal: number;
}

function severityVariant(sev: string): 'issue' | 'warning' | 'opportunity' | 'stat' {
  if (sev === 'issue') return 'issue';
  if (sev === 'warning') return 'warning';
  if (sev === 'opportunity') return 'opportunity';
  return 'stat';
}

function tabInitial(display: string): string {
  const first = display.trim().charAt(0);
  return first ? first.toUpperCase() : '?';
}

export function Sidebar({
  tabs,
  overview,
  expanded,
  setExpanded,
  sel,
  onSelect,
  tabTotals,
  collapsed,
  onToggleCollapsed,
  issuesActive,
  onSelectIssues,
  issuesTotal,
}: Props) {
  if (collapsed) {
    return (
      <aside className="flex h-full w-10 shrink-0 flex-col border-r border-border bg-background">
        <button
          type="button"
          onClick={onToggleCollapsed}
          title="Expand sidebar"
          className="flex h-8 items-center justify-center border-b border-border text-muted-foreground hover:bg-accent hover:text-accent-foreground"
        >
          <ChevronRight className="h-4 w-4" />
        </button>
        <ScrollArea className="flex-1">
          <div className="flex flex-col items-center gap-0.5 py-2">
            <Tooltip>
              <TooltipTrigger asChild>
                <button
                  type="button"
                  onClick={onSelectIssues}
                  className={cn(
                    'relative flex h-7 w-7 items-center justify-center rounded',
                    issuesActive
                      ? 'bg-emerald-500 text-white'
                      : 'text-muted-foreground hover:bg-accent hover:text-accent-foreground',
                  )}
                >
                  <AlertCircle className="h-4 w-4" />
                  {issuesTotal > 0 && (
                    <span className="absolute -right-0.5 -top-0.5 inline-flex h-3 min-w-3 items-center justify-center rounded-full bg-severity-issue px-[3px] text-[8px] font-medium leading-none text-white">
                      {issuesTotal > 99 ? '99+' : issuesTotal}
                    </span>
                  )}
                </button>
              </TooltipTrigger>
              <TooltipContent side="right">
                Issues{issuesTotal > 0 && <span className="ml-1 opacity-70">({issuesTotal})</span>}
              </TooltipContent>
            </Tooltip>
            <div className="my-1 h-px w-6 bg-border" />
            {tabs.map((t) => {
              const total = tabTotals[t.key] ?? 0;
              const isActiveTab = t.key === sel?.tabKey;
              return (
                <Tooltip key={t.key}>
                  <TooltipTrigger asChild>
                    <button
                      type="button"
                      onClick={() => {
                        onToggleCollapsed();
                        setExpanded((x) => ({ ...x, [t.key]: true }));
                      }}
                      className={cn(
                        'relative flex h-7 w-7 items-center justify-center rounded text-[10px] font-semibold',
                        isActiveTab
                          ? 'bg-primary/10 text-primary'
                          : 'text-muted-foreground hover:bg-accent hover:text-accent-foreground',
                      )}
                    >
                      {tabInitial(t.display_name)}
                      {total > 0 && (
                        <span className="absolute -right-0.5 -top-0.5 inline-flex h-3 min-w-3 items-center justify-center rounded-full bg-severity-stat px-[3px] text-[8px] font-medium leading-none text-white">
                          {total > 99 ? '99+' : total}
                        </span>
                      )}
                    </button>
                  </TooltipTrigger>
                  <TooltipContent side="right">
                    {t.display_name}
                    {total > 0 && <span className="ml-1 text-muted-foreground">({total})</span>}
                  </TooltipContent>
                </Tooltip>
              );
            })}
          </div>
        </ScrollArea>
      </aside>
    );
  }

  return (
    <aside className="flex h-full w-[280px] shrink-0 flex-col border-r border-border bg-background">
      <div className="flex items-center justify-between border-b border-border px-3 py-1.5">
        <span className="text-[10px] font-semibold uppercase tracking-widest text-muted-foreground">
          Overview
        </span>
        <button
          type="button"
          onClick={onToggleCollapsed}
          title="Collapse sidebar"
          className="flex h-5 w-5 items-center justify-center rounded text-muted-foreground hover:bg-accent hover:text-accent-foreground"
        >
          <ChevronLeft className="h-3.5 w-3.5" />
        </button>
      </div>
      <div className="border-b border-border px-2 py-1.5">
        <button
          type="button"
          onClick={onSelectIssues}
          className={cn(
            'flex w-full items-center justify-between gap-2 rounded-md px-2 py-1.5 text-left text-[12px] font-semibold transition-colors',
            issuesActive
              ? 'bg-emerald-500 text-white shadow-sm hover:bg-emerald-500/90'
              : 'border border-emerald-500/30 bg-emerald-500/10 text-emerald-700 hover:bg-emerald-500/20',
          )}
        >
          <span className="flex items-center gap-2">
            <AlertCircle className="h-3.5 w-3.5" />
            Issues
          </span>
          <span
            className={cn(
              'inline-flex min-w-[22px] items-center justify-center rounded-full px-1.5 py-0.5 text-[10px] font-semibold tabular-nums',
              issuesActive ? 'bg-white/20 text-white' : 'bg-emerald-500/20 text-emerald-700',
            )}
          >
            {issuesTotal}
          </span>
        </button>
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
