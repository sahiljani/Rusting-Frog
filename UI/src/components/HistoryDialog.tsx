import { useEffect, useState } from 'react';
import { Trash2 } from 'lucide-react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  clearHistory,
  getHistory,
  removeHistoryEntry,
  type CrawlHistoryEntry,
} from '@/api';

interface Props {
  open: boolean;
  onOpenChange: (v: boolean) => void;
  onLoad: (id: string) => void;
}

export function HistoryDialog({ open, onOpenChange, onLoad }: Props) {
  const [entries, setEntries] = useState<CrawlHistoryEntry[]>([]);

  useEffect(() => {
    if (open) setEntries(getHistory());
  }, [open]);

  const removeOne = (id: string) => {
    removeHistoryEntry(id);
    setEntries(getHistory());
  };

  const wipe = () => {
    clearHistory();
    setEntries([]);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>Crawl History</DialogTitle>
          <DialogDescription>
            Past crawls from this browser. Only crawl IDs and seed URLs are
            stored locally — the audit data still lives in the server database.
          </DialogDescription>
        </DialogHeader>

        {entries.length === 0 ? (
          <div className="py-6 text-center text-sm text-muted-foreground">
            No crawls yet. Start one from the toolbar above.
          </div>
        ) : (
          <ScrollArea className="max-h-[55vh]">
            <ul className="divide-y divide-border">
              {entries.map((e) => (
                <li
                  key={e.crawl_id}
                  className="flex items-center gap-3 py-2 pr-1"
                >
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-sm font-medium">{e.name}</div>
                    <div className="truncate font-mono text-[11px] text-primary">
                      {e.seed_url}
                    </div>
                    <div className="flex items-center gap-3 text-[10px] text-muted-foreground">
                      <span className="font-mono">{e.crawl_id.slice(0, 8)}</span>
                      <span>{new Date(e.created_at).toLocaleString()}</span>
                    </div>
                  </div>
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={() => {
                      onLoad(e.crawl_id);
                      onOpenChange(false);
                    }}
                  >
                    Load
                  </Button>
                  <button
                    type="button"
                    title="Remove from history"
                    onClick={() => removeOne(e.crawl_id)}
                    className="flex h-7 w-7 items-center justify-center rounded text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
                  >
                    <Trash2 className="h-3.5 w-3.5" />
                  </button>
                </li>
              ))}
            </ul>
          </ScrollArea>
        )}

        {entries.length > 0 && (
          <div className="flex justify-end border-t border-border pt-3">
            <Button variant="ghost" size="sm" onClick={wipe}>
              Clear all
            </Button>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
