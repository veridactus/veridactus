// X-Ray Panel — Developer Hub 全息调试面板
import { motion } from 'framer-motion';
import { Eye, Shield, Activity, DollarSign, Zap, AlertTriangle } from 'lucide-react';

interface XRayProps { rawRequest?: string; sanitizedRequest?: string; tokensUsed?: number; costUsd?: number; budgetRemaining?: number; guardrailTriggers?: string[]; safetyScore?: number; }

export default function XRayPanel({ rawRequest, sanitizedRequest, tokensUsed = 0, costUsd = 0, budgetRemaining = 0, guardrailTriggers = [], safetyScore = 100 }: XRayProps) {
  return (
    <motion.div initial={{ opacity: 0, x: 20 }} animate={{ opacity: 1, x: 0 }}
      className="flex flex-col gap-4 p-4 h-full overflow-auto" style={{ background: 'rgba(19,22,51,0.95)', borderLeft: '1px solid rgba(108,92,231,0.2)' }}>
      <div className="flex items-center gap-2"><Eye size={18} color="#6c5ce7" /><span className="font-bold text-sm text-white">X-Ray Panel</span></div>

      <div className="grid grid-cols-2 gap-2.5">
        <MetricCard icon={<Activity size={16} />} label="Tokens" value={tokensUsed.toLocaleString()} color="#74b9ff" />
        <MetricCard icon={<DollarSign size={16} />} label="Cost" value={`$${costUsd.toFixed(6)}`} color="#00d4aa" />
        <MetricCard icon={<Zap size={16} />} label="Budget Left" value={`$${budgetRemaining.toFixed(4)}`} color={budgetRemaining > 0.01 ? '#00d4aa' : '#ff7675'} />
        <MetricCard icon={<Shield size={16} />} label="Safety" value={`${safetyScore}%`} color={safetyScore > 80 ? '#00d4aa' : safetyScore > 50 ? '#fdcb6e' : '#ff7675'} />
      </div>

      {guardrailTriggers.length > 0 && (
        <div className="p-3.5 rounded-xl" style={{ background: 'rgba(255,118,117,0.08)', border: '1px solid rgba(255,118,117,0.2)' }}>
          <div className="flex items-center gap-1.5 mb-2"><AlertTriangle size={14} color="#ff7675" /><span className="text-xs font-semibold text-[#ff7675]">Guardrail Triggers</span></div>
          {guardrailTriggers.map((t, i) => <div key={i} className="text-[11px] text-[#ff7675] py-1 px-2 rounded-md mt-1 font-mono" style={{ background: 'rgba(255,118,117,0.1)' }}>{t}</div>)}
        </div>
      )}

      {rawRequest && sanitizedRequest && (<div><div className="text-[11px] font-semibold text-[#8892b0] mb-2">Request Diff</div><DiffView raw={rawRequest} sanitized={sanitizedRequest} /></div>)}

      {rawRequest && (<div><div className="text-[11px] font-semibold text-[#8892b0] mb-1">Raw Request</div>
        <pre className="bg-[rgba(255,255,255,0.03)] rounded-lg p-2.5 text-[10px] text-[#8892b0] overflow-auto max-h-[200px] font-mono whitespace-pre-wrap break-all">{rawRequest.slice(0, 1000)}</pre></div>)}
    </motion.div>
  );
}

function MetricCard({ icon, label, value, color }: { icon: React.ReactNode; label: string; value: string; color: string }) {
  return (
    <div className="bg-[rgba(255,255,255,0.03)] rounded-btn p-3" style={{ border: `1px solid ${color}20` }}>
      <div className="flex items-center gap-1.5 mb-1" style={{ color }}>{icon}<span className="text-[10px] font-semibold">{label}</span></div>
      <div className="text-base font-bold text-white font-mono">{value}</div>
    </div>
  );
}

function DiffView({ raw, sanitized }: { raw: string; sanitized: string }) {
  const words = raw.split(/\s+/); const sanWords = sanitized.split(/\s+/);
  return <div className="text-[10px] font-mono leading-relaxed max-h-[150px] overflow-auto">{words.map((w, i) => (
    <span key={i} className="px-px rounded-sm" style={{ background: sanWords[i] === '[REDACTED]' ? 'rgba(255,118,117,0.3)' : 'transparent', color: sanWords[i] === '[REDACTED]' ? '#ff7675' : '#8892b0', textDecoration: sanWords[i] === '[REDACTED]' ? 'line-through' : 'none' }}>{w} </span>
  ))}</div>;
}