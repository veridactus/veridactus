// VERIDACTUS — 审计指挥舱辅助组件
import { type LucideIcon, CheckCircle, XCircle, AlertCircle } from 'lucide-react';

interface MetricCardProps {
  label: string;
  value: string;
  icon: LucideIcon;
  color: string;
}

export function MetricCard({ label, value, icon: Icon, color }: MetricCardProps) {
  return (
    <div className="flex items-center gap-2">
      <div className="p-2 rounded-lg" style={{ background: `${color}20` }}>
        <Icon size={16} style={{ color }} />
      </div>
      <div>
        <p className="text-sm font-semibold text-[var(--text-primary)]">{value}</p>
        <p className="text-[10px] text-[var(--text-tertiary)]">{label}</p>
      </div>
    </div>
  );
}

export function VerificationBadge({ level, passed }: { level: string; passed?: boolean }) {
  return (
    <div className="text-center p-2.5 rounded-lg" style={{ background: 'rgba(0,0,0,0.2)' }}>
      {passed === true
        ? <CheckCircle size={20} style={{ color: '#00d4aa' }} />
        : passed === false
        ? <XCircle size={20} style={{ color: '#ff7675' }} />
        : <AlertCircle size={20} className="text-[var(--text-tertiary)]" />
      }
      <p className="text-xs font-semibold text-[var(--text-primary)] mt-1">{level}</p>
      <p className="text-[10px] text-[var(--text-tertiary)] mt-0.5">
        {passed === true ? 'Passed' : passed === false ? 'Failed' : 'Not Available'}
      </p>
    </div>
  );
}
