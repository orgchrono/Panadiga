import { ShieldAlert } from 'lucide-react';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui';

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
