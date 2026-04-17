import { Info, ExternalLink } from 'lucide-react';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { sfUserGuide } from '@/docs/sf-user-guide';

interface InfoTipProps {
  field: string;
  className?: string;
}

/**
 * Small info icon that renders a tooltip with verbatim text pulled from the
 * Screaming Frog user guide. Gracefully renders nothing if the field key is
 * not (yet) in the docs dictionary — later batches expand coverage.
 */
export function InfoTip({ field, className }: InfoTipProps) {
  const doc = sfUserGuide[field];
  if (!doc) return null;
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <span
          className={`inline-flex cursor-help items-center text-muted-foreground hover:text-foreground ${className ?? ''}`}
          aria-label={`Help: ${doc.title}`}
        >
          <Info className="h-3.5 w-3.5" />
        </span>
      </TooltipTrigger>
      <TooltipContent side="bottom" align="start">
        <div className="space-y-1.5">
          <div className="text-[11px] font-semibold uppercase tracking-wide text-foreground">
            {doc.title}
          </div>
          <p className="text-xs leading-relaxed text-muted-foreground">{doc.body}</p>
          <a
            href={doc.href}
            target="_blank"
            rel="noreferrer"
            className="inline-flex items-center gap-1 text-[11px] font-medium text-primary hover:underline"
          >
            User guide <ExternalLink className="h-3 w-3" />
          </a>
        </div>
      </TooltipContent>
    </Tooltip>
  );
}
