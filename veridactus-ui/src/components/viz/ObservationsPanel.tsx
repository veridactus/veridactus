import { motion } from 'framer-motion';
import { Activity, Clock, Zap, DollarSign, Shield, TrendingUp, AlertTriangle, CheckCircle } from 'lucide-react';
import AnimatedMetric from './AnimatedMetric';
import type { Observations } from '../../types';

interface ObservationsPanelProps {
  observations: Observations | undefined;
}

export default function ObservationsPanel({ observations }: ObservationsPanelProps) {
  const metrics = [
    {
      key: 'token_count',
      label: 'Total Tokens',
      icon: Activity,
      value: observations?.token_count || 0,
      suffix: ' tokens',
      color: '#00d4aa',
      bgColor: 'rgba(0, 212, 170, 0.15)',
    },
    {
      key: 'prompt_tokens',
      label: 'Prompt Tokens',
      icon: Zap,
      value: observations?.prompt_tokens || 0,
      suffix: '',
      color: '#74b9ff',
      bgColor: 'rgba(116, 185, 255, 0.15)',
    },
    {
      key: 'completion_tokens',
      label: 'Completion Tokens',
      icon: Zap,
      value: observations?.completion_tokens || 0,
      suffix: '',
      color: '#fdcb6e',
      bgColor: 'rgba(253, 203, 110, 0.15)',
    },
    {
      key: 'latency_ms',
      label: 'Latency',
      icon: Clock,
      value: observations?.latency_ms || 0,
      suffix: ' ms',
      color: '#a29bfe',
      bgColor: 'rgba(162, 155, 254, 0.15)',
    },
    {
      key: 'cost_usd',
      label: 'Cost',
      icon: DollarSign,
      value: observations?.cost_usd || 0,
      prefix: '$',
      decimals: 4,
      color: '#fdcb6e',
      bgColor: 'rgba(253, 203, 110, 0.15)',
    },
    {
      key: 'budget_used',
      label: 'Budget Used',
      icon: Shield,
      value: observations?.budget_used || 0,
      suffix: ` / ${observations?.budget_limit || 100}%`,
      decimals: 1,
      color: observations?.budget_used && observations?.budget_used > 80 ? '#ff7675' : '#00d4aa',
      bgColor: 'rgba(0, 212, 170, 0.15)',
    },
  ];

  const riskScore = observations?.risk_score || 0;
  const driftScore = observations?.drift_score || 0;

  return (
    <motion.div
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      className="glass-card"
      style={{ padding: 20 }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 16 }}>
        <div style={{ width: 3, height: 16, background: 'linear-gradient(180deg, #fdcb6e 0%, #f39c12 100%)', borderRadius: 2 }} />
        <h3 style={{ fontSize: 13, fontWeight: 600, color: 'var(--text-secondary)', display: 'flex', alignItems: 'center', gap: 6 }}>
          <Activity size={14} /> Observations
        </h3>
        <span style={{ marginLeft: 'auto', fontSize: 10, color: 'var(--text-tertiary)' }}>
          {observations?.events?.length || 0} events tracked
        </span>
      </div>

      {/* 主要指标网格 */}
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 12 }}>
        {metrics.map((metric, index) => (
          <motion.div
            key={metric.key}
            initial={{ opacity: 0, scale: 0.95 }}
            animate={{ opacity: 1, scale: 1 }}
            transition={{ delay: index * 0.05 }}
            style={{
              padding: 14,
              borderRadius: 12,
              background: metric.bgColor,
              border: `1px solid ${metric.color}20`,
              transition: 'all 0.25s ease',
            }}
            whileHover={{ transform: 'translateY(-2px)', boxShadow: `0 8px 20px ${metric.color}15` }}
          >
            <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 8 }}>
              <metric.icon size={14} style={{ color: metric.color }} />
              <span style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>{metric.label}</span>
            </div>
            <div style={{ fontSize: 20, fontWeight: 700, color: metric.color, fontFamily: "'JetBrains Mono', monospace" }}>
              <AnimatedMetric
                value={metric.value}
                prefix={metric.prefix}
                suffix={metric.suffix}
                decimals={metric.decimals || 0}
              />
            </div>
          </motion.div>
        ))}
      </div>

      {/* 风险和漂移分数 */}
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12, marginTop: 16 }}>
        <motion.div
          initial={{ opacity: 0, x: -10 }}
          animate={{ opacity: 1, x: 0 }}
          transition={{ delay: 0.3 }}
          style={{ padding: 14, borderRadius: 12, background: 'rgba(255, 118, 117, 0.1)', border: '1px solid rgba(255, 118, 117, 0.2)' }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 8 }}>
            <AlertTriangle size={14} style={{ color: '#ff7675' }} />
            <span style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>Risk Score</span>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <span style={{ fontSize: 28, fontWeight: 700, color: riskScore > 70 ? '#ff7675' : riskScore > 40 ? '#fdcb6e' : '#00d4aa' }}>
              {riskScore}
            </span>
            <div style={{ flex: 1, height: 6, background: 'rgba(255,255,255,0.1)', borderRadius: 3, overflow: 'hidden' }}>
              <motion.div
                initial={{ width: 0 }}
                animate={{ width: `${riskScore}%` }}
                transition={{ duration: 0.8, delay: 0.4 }}
                style={{
                  height: '100%',
                  background: riskScore > 70 ? '#ff7675' : riskScore > 40 ? '#fdcb6e' : '#00d4aa',
                  borderRadius: 3,
                }}
              />
            </div>
          </div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, x: 10 }}
          animate={{ opacity: 1, x: 0 }}
          transition={{ delay: 0.35 }}
          style={{ padding: 14, borderRadius: 12, background: 'rgba(116, 185, 255, 0.1)', border: '1px solid rgba(116, 185, 255, 0.2)' }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 8 }}>
            <TrendingUp size={14} style={{ color: '#74b9ff' }} />
            <span style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>Drift Score</span>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <span style={{ fontSize: 28, fontWeight: 700, color: driftScore > 50 ? '#fdcb6e' : '#74b9ff' }}>
              {driftScore}
            </span>
            <div style={{ flex: 1, height: 6, background: 'rgba(255,255,255,0.1)', borderRadius: 3, overflow: 'hidden' }}>
              <motion.div
                initial={{ width: 0 }}
                animate={{ width: `${driftScore}%` }}
                transition={{ duration: 0.8, delay: 0.45 }}
                style={{
                  height: '100%',
                  background: driftScore > 50 ? '#fdcb6e' : '#74b9ff',
                  borderRadius: 3,
                }}
              />
            </div>
          </div>
        </motion.div>
      </div>

      {/* 事件时间线 */}
      {observations?.events && observations.events.length > 0 && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ delay: 0.5 }}
          style={{ marginTop: 16, paddingTop: 16, borderTop: '1px solid rgba(255,255,255,0.06)' }}
        >
          <div style={{ fontSize: 11, color: 'var(--text-tertiary)', marginBottom: 10, fontWeight: 600 }}>Event Timeline</div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            {observations.events.map((event, i) => (
              <motion.div
                key={i}
                initial={{ opacity: 0, x: -10 }}
                animate={{ opacity: 1, x: 0 }}
                transition={{ delay: 0.5 + i * 0.1 }}
                style={{ display: 'flex', alignItems: 'center', gap: 10 }}
              >
                <div style={{
                  width: 10,
                  height: 10,
                  borderRadius: '50%',
                  background: event.event_type.includes('success') ? '#00d4aa' : event.event_type.includes('error') ? '#ff7675' : '#fdcb6e',
                  boxShadow: event.event_type.includes('success') ? '0 0 8px rgba(0,212,170,0.5)' : 'none',
                }} />
                <div style={{ flex: 1 }}>
                  <span style={{ fontSize: 12, color: 'var(--text-primary)' }}>{event.event_type.replace('_', ' ')}</span>
                  <span style={{ fontSize: 10, color: 'var(--text-tertiary)', marginLeft: 8 }}>
                    {new Date(event.timestamp).toLocaleTimeString()}
                  </span>
                </div>
                {event.event_type.includes('success') && <CheckCircle size={12} style={{ color: '#00d4aa' }} />}
              </motion.div>
            ))}
          </div>
        </motion.div>
      )}
    </motion.div>
  );
}
