import { useMetricsStore } from '../../store';
import { useI18n } from '../../i18n';
import { Server, CheckCircle, XCircle, Activity, Clock } from 'lucide-react';

export default function BottomStatusBar() {
  const { t } = useI18n();
  const { services, traceCount } = useMetricsStore();
  const allOk = services.dataPlane && services.controlPlane;

  return (
    <div style={{
      position: 'fixed', bottom: 0, left: 260, right: 0, height: 40,
      background: 'var(--bg-secondary)', borderTop: '1px solid var(--border-default)',
      display: 'flex', alignItems: 'center', justifyContent: 'space-between',
      padding: '0 24px', fontSize: 12, color: 'var(--text-tertiary)', zIndex: 30,
    }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 20 }}>
        <span style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
          <Server size={12} /> {t('status.data_plane')}:{services.dataPlane ? <CheckCircle size={12} color="#00d4aa" /> : <XCircle size={12} color="#ff7675" />}
        </span>
        <span style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
          <Server size={12} /> {t('status.control_plane')}:{services.controlPlane ? <CheckCircle size={12} color="#00d4aa" /> : <XCircle size={12} color="#ff7675" />}
        </span>
        <span style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
          <Server size={12} /> {t('status.python_worker')}:{services.pythonWorker ? <CheckCircle size={12} color="#00d4aa" /> : <XCircle size={12} color="#ff7675" />}
        </span>
      </div>
      <div style={{ display: 'flex', alignItems: 'center', gap: 20 }}>
        <span style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
          <Activity size={12} /> {t('status.traces')}: {traceCount}
        </span>
        <span style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
          <Clock size={12} /> {t('status.system')}: {allOk ? t('status.healthy') : t('status.degraded')}
        </span>
      </div>
    </div>
  );
}
