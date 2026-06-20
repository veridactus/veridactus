import { motion } from 'framer-motion';
import { ArrowRight, CheckCircle } from 'lucide-react';
import type { TraceDetail } from '../../types';

interface ExecutionContractProps {
  trace: TraceDetail;
}

const tupleConfig = [
  { key: 'input', label: 'Input', emoji: '📥', color: '#00d4aa', bgColor: 'rgba(0, 212, 170, 0.15)' },
  { key: 'constraints', label: 'Constraints', emoji: '⚙️', color: '#74b9ff', bgColor: 'rgba(116, 185, 255, 0.15)' },
  { key: 'observations', label: 'Observations', emoji: '📈', color: '#fdcb6e', bgColor: 'rgba(253, 203, 110, 0.15)' },
  { key: 'proofs', label: 'Proofs', emoji: '🔐', color: '#a29bfe', bgColor: 'rgba(162, 155, 254, 0.15)' },
];

export default function ExecutionContract({ trace }: ExecutionContractProps) {
  const hasInput = !!trace.input;
  const hasConstraints = !!trace.constraints_applied && typeof trace.constraints_applied === 'object' && Object.keys(trace.constraints_applied).length > 0;
  const hasObservations = !!trace.observations;
  const hasProofs = !!trace.proofs?.proof_chain && trace.proofs.proof_chain.length > 0;
  
  const statuses = [hasInput, hasConstraints, hasObservations, hasProofs];

  return (
    <motion.div
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      className="glass-card"
      style={{ padding: 20, background: 'linear-gradient(135deg, rgba(108,92,231,0.08) 0%, rgba(0,212,170,0.04) 100%)' }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 16 }}>
        <div style={{ width: 3, height: 16, background: 'linear-gradient(180deg, #6c5ce7 0%, #00d4aa 100%)', borderRadius: 2 }} />
        <h3 style={{ fontSize: 12, fontWeight: 700, color: '#6c5ce7', letterSpacing: '0.05em', textTransform: 'uppercase' }}>
          Execution Contract
        </h3>
        <div style={{ marginLeft: 'auto', display: 'flex', alignItems: 'center', gap: 4 }}>
          <CheckCircle size={14} style={{ color: '#00d4aa' }} />
          <span style={{ fontSize: 11, color: '#00d4aa', fontWeight: 600 }}>Complete</span>
        </div>
      </div>

      <div style={{ display: 'flex', alignItems: 'center', gap: 4, position: 'relative' }}>
        {tupleConfig.map((item, index) => (
          <motion.div
            key={item.key}
            initial={{ opacity: 0, scale: 0.9 }}
            animate={{ opacity: 1, scale: 1 }}
            transition={{ delay: index * 0.1 }}
            style={{
              flex: 1,
              display: 'flex',
              flexDirection: 'column',
              alignItems: 'center',
              padding: 12,
              borderRadius: 12,
              background: statuses[index] ? item.bgColor : 'rgba(255,255,255,0.03)',
              border: `1px solid ${statuses[index] ? `${item.color}30` : 'rgba(255,255,255,0.08)'}`,
              transition: 'all 0.3s ease',
            }}
            whileHover={{ transform: 'translateY(-4px)', boxShadow: `0 8px 24px ${item.color}20` }}
          >
            <div style={{
              width: 56,
              height: 56,
              borderRadius: 14,
              background: statuses[index] ? item.bgColor : 'rgba(255,255,255,0.05)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              marginBottom: 8,
              border: `2px solid ${statuses[index] ? `${item.color}50` : 'rgba(255,255,255,0.1)'}`,
            }}>
              <span style={{ fontSize: 24 }}>{item.emoji}</span>
            </div>
            <span style={{ fontSize: 11, fontWeight: 600, color: statuses[index] ? item.color : 'var(--text-tertiary)' }}>
              {item.label}
            </span>
            <span style={{ fontSize: 9, color: 'var(--text-tertiary)', marginTop: 2 }}>
              {statuses[index] ? '✓ Captured' : '-'}
            </span>
          </motion.div>
        ))}

        {/* 连接线 */}
        {tupleConfig.slice(0, -1).map((_, index) => (
          <motion.div
            key={`line-${index}`}
            initial={{ scaleX: 0 }}
            animate={{ scaleX: 1 }}
            transition={{ delay: index * 0.1 + 0.2, duration: 0.3 }}
            style={{
              position: 'absolute',
              top: '50%',
              left: `${(index + 1) * 25}%`,
              width: '8%',
              height: 2,
              transform: 'translateY(-50%)',
              background: 'linear-gradient(90deg, #6c5ce7 0%, #00d4aa 100%)',
              zIndex: -1,
            }}
          >
            <ArrowRight
              size={14}
              style={{
                position: 'absolute',
                right: -7,
                top: '50%',
                transform: 'translateY(-50%)',
                color: '#00d4aa',
              }}
            />
          </motion.div>
        ))}

        {/* 闭环箭头 */}
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ delay: 0.6 }}
          style={{
            position: 'absolute',
            right: 8,
            bottom: -8,
            color: '#a29bfe',
            transform: 'rotate(45deg)',
            opacity: 0.5,
          }}
        >
          <ArrowRight size={20} />
        </motion.div>
      </div>

      <div style={{ marginTop: 20, paddingTop: 12, borderTop: '1px solid rgba(255,255,255,0.06)' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 10, color: 'var(--text-tertiary)' }}>
          <span>Trace: {trace.trace_id?.slice(0, 8)}...</span>
          <span>Model: {trace.model}</span>
          <span>State: {trace.execution_state || 'N/A'}</span>
        </div>
      </div>
    </motion.div>
  );
}
