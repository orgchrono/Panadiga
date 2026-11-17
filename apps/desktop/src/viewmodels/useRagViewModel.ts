import { useState, useCallback } from 'react';
import { ExecutionStatus, type RagQueryResult } from '@/models';
import { queryRagEngine } from '@/services/systemService';

export function useRagViewModel(initialQuery = 'How to recover error U99 on Spansion Flash?') {
  const [ragQuery, setRagQuery] = useState<string>(initialQuery);
  const [ragStatus, setRagStatus] = useState<ExecutionStatus>(ExecutionStatus.IDLE);
  const [ragResult, setRagResult] = useState<RagQueryResult | null>(null);
  const [ragError, setRagError] = useState<string | null>(null);

  const handleRagSubmit = useCallback(async () => {
    if (!ragQuery.trim()) return;
    setRagStatus(ExecutionStatus.RUNNING);
    setRagError(null);

    try {
      const res = await queryRagEngine(ragQuery);
      setRagResult(res);
      setRagStatus(ExecutionStatus.SUCCESS);
    } catch (err) {
      setRagError(err instanceof Error ? err.message : String(err));
      setRagStatus(ExecutionStatus.ERROR);
    }
  }, [ragQuery]);

  return {
    ragQuery,
    setRagQuery,
    ragStatus,
    ragResult,
    ragError,
    handleRagSubmit,
  };
}
