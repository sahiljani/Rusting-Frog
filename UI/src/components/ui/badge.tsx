import * as React from 'react';
import { cva, type VariantProps } from 'class-variance-authority';
import { cn } from '@/lib/utils';

const badgeVariants = cva(
  'inline-flex items-center rounded-md px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wide',
  {
    variants: {
      variant: {
        default: 'bg-primary/10 text-primary',
        outline: 'border border-border text-muted-foreground',
        issue: 'bg-severity-issue/15 text-severity-issue',
        warning: 'bg-severity-warning/15 text-severity-warning',
        opportunity: 'bg-severity-opportunity/15 text-severity-opportunity',
        stat: 'bg-muted text-muted-foreground',
      },
    },
    defaultVariants: { variant: 'default' },
  },
);

export interface BadgeProps
  extends React.HTMLAttributes<HTMLSpanElement>,
    VariantProps<typeof badgeVariants> {}

export function Badge({ className, variant, ...props }: BadgeProps) {
  return <span className={cn(badgeVariants({ variant }), className)} {...props} />;
}
