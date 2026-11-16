import { ShieldAlert } from 'lucide-react';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui';

export interface MatshitaAsyncSlice<T> {
  loading: boolean;
  data: T | null;
  error: string | null;
}

export const MATSHITA_SELECT_CLS =
  'flex h-9 w-full rounded-xl border border-slate-800 bg-slate-950 px-3 py-2 text-xs font-mono text-slate-100 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-cyan-500';

export function MatshitaErrorBanner({ message }: { message: string | null }) {
  if (!message) return null;
  return (
    <Alert variant="destructive" className="py-2">
      <ShieldAlert className="h-4 w-4" />
      <AlertTitle className="text-xs font-mono">E05 / Validation</AlertTitle>
      <AlertDescription className="text-xs font-mono break-all">{message}</AlertDescription>
    </Alert>
  );
}
