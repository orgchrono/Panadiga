import { useState, useEffect, useCallback } from 'react';
import type { SystemOverview } from '@/models';
import { fetchSystemOverview } from '@/services/systemService';
import { logger } from '@/utils/logger';

export function useDashboardViewModel() {
  const [overview, setOverview] = useState<SystemOverview | null>(null);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);

  const fetchOverview = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await fetchSystemOverview();
      setOverview(data);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setError(msg);
      logger.error('DashboardViewModel', 'Error fetching system overview', err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void Promise.resolve().then(() => fetchOverview());
  }, [fetchOverview]);

  return {
    overview,
    loading,
    error,
    refresh: fetchOverview,
  };
}
