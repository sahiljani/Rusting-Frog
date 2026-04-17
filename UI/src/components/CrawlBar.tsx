import { Play, Square, Pause } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { InfoTip } from './InfoTip';

interface Props {
  seedUrl: string;
  setSeedUrl: (v: string) => void;
  busy: boolean;
  running: boolean;
  paused: boolean;
  onStart: () => void;
  onPause: () => void;
  onStop: () => void;
}

export function CrawlBar({
  seedUrl,
  setSeedUrl,
  busy,
  running,
  paused,
  onStart,
  onPause,
  onStop,
}: Props) {
  return (
    <div className="flex items-center gap-2 border-b border-border bg-background px-3 py-1.5">
      <label className="flex items-center gap-1 text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
        Seed URL
        <InfoTip field="crawl.seed_url" />
      </label>
      <Input
        type="url"
        inputMode="url"
        autoComplete="off"
        placeholder="https://example.com/"
        className="font-mono text-xs"
        value={seedUrl}
        onChange={(e) => setSeedUrl(e.target.value)}
        disabled={busy || running}
        onKeyDown={(e) => {
          if (e.key === 'Enter' && !busy && !running) onStart();
        }}
      />
      <div className="flex shrink-0 items-center gap-1.5">
        {running && !paused ? (
          <>
            <Button
              variant="outline"
              size="sm"
              onClick={onPause}
              title="Pause crawl"
            >
              <Pause className="h-3 w-3" />
              Pause
              <InfoTip field="crawl.pause" />
            </Button>
            <Button variant="destructive" size="sm" onClick={onStop} title="Stop crawl">
              <Square className="h-3 w-3" />
              Stop
              <InfoTip field="crawl.stop" />
            </Button>
          </>
        ) : (
          <>
            <Button
              variant="success"
              size="sm"
              onClick={onStart}
              disabled={busy}
              title="Start crawl"
            >
              <Play className="h-3 w-3" />
              {paused ? 'Resume' : 'Start'}
              <InfoTip field="crawl.start" />
            </Button>
            {paused && (
              <Button variant="destructive" size="sm" onClick={onStop} title="Stop crawl">
                <Square className="h-3 w-3" />
                Stop
              </Button>
            )}
          </>
        )}
      </div>
    </div>
  );
}
