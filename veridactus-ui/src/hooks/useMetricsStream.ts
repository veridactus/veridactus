import { useEffect, useRef, useCallback } from 'react';
import { useMetricsStore } from '../store';
import { checkAllServices } from '../api';

export function useMetricsStream() {
  const { setMetrics } = useMetricsStore();
  const intervalRef = useRef<ReturnType<typeof setInterval>>();

  const fetchMetrics = useCallback(async () => {
    try {
      const services = await checkAllServices();
      const [tracesRes, pipelinesRes, pluginsRes, policiesRes] = await Promise.all([
        fetch('/v1/traces').then(r => r.json()).catch(() => ({ traces: [] })),
        fetch('/api/v1/pipelines').then(r => r.json()).catch(() => ({ pipelines: [] })),
        fetch('/api/v1/plugins').then(r => r.json()).catch(() => ({ plugins: [] })),
        fetch('/api/v1/policies').then(r => r.json()).catch(() => ({ policies: [] })),
      ]);
      setMetrics({
        services,
        traceCount: tracesRes.traces?.length || 0,
        pipelineCount: pipelinesRes.pipelines?.length || 0,
        pluginCount: pluginsRes.plugins?.length || 0,
        policyCount: policiesRes.policies?.length || 0,
      });
    } catch {}
  }, [setMetrics]);

  useEffect(() => {
    fetchMetrics();
    intervalRef.current = setInterval(fetchMetrics, 10000);
    return () => { if (intervalRef.current) clearInterval(intervalRef.current); };
  }, [fetchMetrics]);
}
