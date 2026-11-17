import * as React from 'react';
import { cva, type VariantProps } from 'class-variance-authority';
import { cn } from '@/lib/utils';

const alertVariants = cva(
  'relative w-full rounded-xl border p-3.5 text-xs font-mono flex items-start gap-3',
  {
    variants: {
      variant: {
        default: 'bg-slate-950/80 border-slate-800 text-slate-200',
        info: 'bg-cyan-500/10 border-cyan-500/30 text-cyan-200',
        success: 'bg-emerald-500/10 border-emerald-500/30 text-emerald-200',
        warning: 'bg-amber-500/10 border-amber-500/30 text-amber-200',
        destructive: 'bg-rose-500/10 border-rose-500/40 text-rose-200',
      },
    },
    defaultVariants: {
      variant: 'default',
    },
  },
);

const Alert = React.forwardRef<
  HTMLDivElement,
  React.HTMLAttributes<HTMLDivElement> & VariantProps<typeof alertVariants>
>(function Alert({ className, variant, ...props }, ref) {
  return <div ref={ref} role="alert" className={cn(alertVariants({ variant }), className)} {...props} />;
});

const AlertTitle = React.forwardRef<HTMLHeadingElement, React.HTMLAttributes<HTMLHeadingElement>>(
  function AlertTitle({ className, children, ...props }, ref) {
    return (
      <h5 ref={ref} className={cn('font-bold leading-tight tracking-tight', className)} {...props}>
        {children}
      </h5>
    );
  },
);

const AlertDescription = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  function AlertDescription({ className, ...props }, ref) {
    return (
      <div
        ref={ref}
        className={cn('text-[11px] opacity-90 leading-relaxed font-sans mt-0.5', className)}
        {...props}
      />
    );
  },
);

export { Alert, AlertTitle, AlertDescription };
