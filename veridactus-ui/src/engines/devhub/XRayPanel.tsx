// X-Ray Panel — Developer Hub 全息调试面板
// 实时展示治理透视信息
import { motion } from 'framer-motion';
import { Eye, Shield, Activity, DollarSign, Zap, AlertTriangle } from 'lucide-react';

interface XRayProps {
  rawRequest?: string;
  sanitizedRequest?: string;
  tokensUsed?: number;
  costUsd?: number;
  budgetRemaining?: number;
  guardrailTriggers?: string[];
  safetyScore?: number;
}

export default function XRayPanel({
  rawRequest, sanitizedRequest,
  tokensUsed = 0, costUsd = 0, budgetRemaining = 0,
  guardrailTriggers = [], safetyScore = 100,
}: XRayProps) {
  return (
    <motion.div
      initial={{ opacity: 0, x: 20 }}
      animate={{ opacity: 1, x: 0 }}
      style={{
        display: 'flex', flexDirection: 'column', gap: 16, padding: 16,
        background: 'rgba(19,22,51,0.95)', borderLeft: '1px solid rgba(108,92,231,0.2)',
        height: '100%', overflow: 'auto',
      }}
    >
      {/* Header */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        <Eye size={18} color="#6c5ce7" />
        <span style={{ fontWeight: 700, fontSize: 14, color: '#fff' }}>X-Ray Panel</span>
      </div>

      {/* Token & Cost */}
      <div style={{
        display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 10,
      }}>
        <MetricCard
          icon={<Activity size={16} />}
          label="Tokens"
          value={tokensUsed.toLocaleString()}
          color="#74b9ff"
        />
        <MetricCard
          icon={<DollarSign size={16} />}
          label="Cost"
          value={`$${costUsd.toFixed(6)}`}
          color="#00d4aa"
        />
        <MetricCard
          icon={<Zap size={16} />}
          label="Budget Left"
          value={`$${budgetRemaining.toFixed(4)}`}
          color={budgetRemaining > 0.01 ? '#00d4aa' : '#ff7675'}
        />
        <MetricCard
          icon={<Shield size={16} />}
          label="Safety"
          value={`${safetyScore}%`}
          color={safetyScore > 80 ? '#00d4aa' : safetyScore > 50 ? '#fdcb6e' : '#ff7675'}
        />
      </div>

      {/* Guardrail Triggers */}
      {guardrailTriggers.length > 0 && (
        <div style={{
          background: 'rgba(255,118,117,0.08)', border: '1px solid rgba(255,118,117,0.2)',
          borderRadius: 12, padding: 14,
        }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 8 }}>
            <AlertTriangle size={14} color="#ff7675" />
            <span style={{ fontSize: 12, fontWeight: 600, color: '#ff7675' }}>Guardrail Triggers</span>
          </div>
          {guardrailTriggers.map((t, i) => (
            <div key={i} style={{
              fontSize: 11, color: '#ff7675', padding: '4px 8px',
              background: 'rgba(255,118,117,0.1)', borderRadius: 6, marginTop: 4,
              fontFamily: 'monospace',
            }}>{t}</div>
          ))}
        </div>
      )}

      {/* Request Diff */}
      {rawRequest && sanitizedRequest && (
        <div>
          <div style={{ fontSize: 11, fontWeight: 600, color: '#8892b0', marginBottom: 8 }}>
            Request Diff (red = redacted)
          </div>
          <DiffView raw={rawRequest} sanitized={sanitizedRequest} />
        </div>
      )}

      {/* Raw Request Preview */}
      {rawRequest && (
        <div>
          <div style={{ fontSize: 11, fontWeight: 600, color: '#8892b0', marginBottom: 4 }}>
            Raw Request
          </div>
          <pre style={{
            background: 'rgba(255,255,255,0.03)', borderRadius: 8, padding: 10,
            fontSize: 10, color: '#8892b0', overflow: 'auto', maxHeight: 200,
            fontFamily: 'monospace', whiteSpace: 'pre-wrap', wordBreak: 'break-all',
          }}>
            {rawRequest.slice(0, 1000)}
          </pre>
        </div>
      )}
    </motion.div>
  );
}

function MetricCard({ icon, label, value, color }: {
  icon: React.ReactNode; label: string; value: string; color: string;
}) {
  return (
    <div style={{
      background: 'rgba(255,255,255,0.03)', borderRadius: 10,
      border: `1px solid ${color}20`, padding: 12,
    }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 6, color, marginBottom: 4 }}>
        {icon}
        <span style={{ fontSize: 10, fontWeight: 600 }}>{label}</span>
      </div>
      <div style={{ fontSize: 16, fontWeight: 700, color: '#fff', fontFamily: 'monospace' }}>
        {value}
      </div>
    </div>
  );
}

function DiffView({ raw, sanitized }: { raw: string; sanitized: string }) {
  // 简单差异高亮: 比较原始内容和脱敏后内容
  const words = raw.split(/\s+/);
  const sanWords = sanitized.split(/\s+/);
  return (
    <div style={{
      fontSize: 10, fontFamily: 'monospace', lineHeight: 1.6,
      maxHeight: 150, overflow: 'auto',
    }}>
      {words.map((word, i) => (
        <span
          key={i}
          style={{
            background: sanWords[i] === '[REDACTED]' ? 'rgba(255,118,117,0.3)' : 'transparent',
            color: sanWords[i] === '[REDACTED]' ? '#ff7675' : '#8892b0',
            textDecoration: sanWords[i] === '[REDACTED]' ? 'line-through' : 'none',
            padding: '0 1px', borderRadius: 2,
          }}
        >{word} </span>
      ))}
    </div>
  );
}
