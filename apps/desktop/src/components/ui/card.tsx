import * as React from 'react';
import { cn } from '@/lib/utils';

const Card = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  function Card({ className, ...props }, ref) {
    return (
      <div
        ref={ref}
        className={cn(
          'rounded-2xl border border-slate-800 bg-bench-900/90 text-slate-100 shadow-xl',
          className,
        )}
        {...props}
      />
    );
  },
);

const CardHeader = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  function CardHeader({ className, ...props }, ref) {
    return <div ref={ref} className={cn('flex flex-col space-y-1.5 p-5 pb-4', className)} {...props} />;
  },
);

const CardTitle = React.forwardRef<HTMLHeadingElement, React.HTMLAttributes<HTMLHeadingElement>>(
  function CardTitle({ className, children, ...props }, ref) {
    return (
      <h3
        ref={ref}
        className={cn('text-sm font-bold leading-none tracking-tight text-slate-100 font-mono', className)}
        {...props}
      >
        {children}
      </h3>
    );
  },
);

const CardDescription = React.forwardRef<
  HTMLParagraphElement,
  React.HTMLAttributes<HTMLParagraphElement>
>(function CardDescription({ className, ...props }, ref) {
  return <p ref={ref} className={cn('text-xs text-slate-400 font-sans', className)} {...props} />;
});

const CardContent = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  function CardContent({ className, ...props }, ref) {
    return <div ref={ref} className={cn('p-5 pt-0', className)} {...props} />;
  },
);

const CardFooter = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  function CardFooter({ className, ...props }, ref) {
    return (
      <div
        ref={ref}
        className={cn('flex items-center p-5 pt-0 border-t border-slate-800/80 mt-4', className)}
        {...props}
      />
    );
  },
);

export { Card, CardHeader, CardFooter, CardTitle, CardDescription, CardContent };
