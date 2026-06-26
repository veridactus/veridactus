// OWASP 一键模板 — 点击自动生成 Pipeline 节点并连线
import { useState } from 'react';
import { motion } from 'framer-motion';
import { Shield, Zap, Lock, FileCode, Check, ArrowRight } from 'lucide-react';

interface Template {
  id: string; name: string; icon: string; desc: string;
  stages: { name: string; plugin: string; order: number }[];
}

const TEMPLATES: Template[] = [
  {
    id: 'owasp-top10', name: 'OWASP Top 10 防护', icon: 'Shield',
    desc: '覆盖 OWASP Top 10 风险：注入攻击、XSS、敏感数据泄露等',
    stages: [
      { name: 'PII 检测', plugin: 'PiiDetector', order: 1 },
      { name: '注入攻击检测', plugin: 'InputSanitizer', order: 2 },
      { name: '预算守卫', plugin: 'BudgetGuard', order: 3 },
      { name: '响应验证', plugin: 'ResponseValidator', order: 4 },
    ],
  },
  {
    id: 'compliance-gdpr', name: 'GDPR 合规', icon: 'Lock',
    desc: 'GDPR 数据保护合规：PII 检测 + 数据脱敏 + 审计签名',
    stages: [
      { name: 'PII 检测', plugin: 'PiiDetector', order: 1 },
      { name: '数据脱敏', plugin: 'InputSanitizer', order: 2 },
      { name: '审计签名', plugin: 'AuthPlugin', order: 3 },
      { name: '响应验证', plugin: 'ResponseValidator', order: 4 },
    ],
  },
  {
    id: 'cost-control', name: '成本管控', icon: 'Zap',
    desc: '微美元预算控制：Token 计数 + 预算熔断 + 成本分析',
    stages: [
      { name: '预算守卫', plugin: 'BudgetGuard', order: 1 },
      { name: '响应验证', plugin: 'ResponseValidator', order: 2 },
    ],
  },
];

interface Props { onSelect: (template: Template) => void; onClose: () => void }

export default function TemplateSelector({ onSelect, onClose }: Props) {
  const [selected, setSelected] = useState<string | null>(null);
  const [step, setStep] = useState<'select' | 'confirm'>('select');

  const handleConfirm = () => {
    const tpl = TEMPLATES.find(t => t.id === selected);
    if (tpl) { onSelect(tpl); onClose(); }
  };

  return (
    <div style={{ padding: 24 }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 20 }}>
        <h2 style={{ fontSize: 18, fontWeight: 700, color: '#fff', margin: 0 }}>
          <Shield size={20} style={{ verticalAlign: -3, marginRight: 8, color: '#6c5ce7' }} />
          一键模板
        </h2>
        {step === 'confirm' && (
          <motion.button whileHover={{ scale: 1.02 }} whileTap={{ scale: 0.98 }}
            onClick={handleConfirm}
            style={{ padding: '8px 20px', borderRadius: 10, background: 'linear-gradient(135deg, #6c5ce7, #00d4aa)', border: 'none', color: '#fff', fontSize: 13, fontWeight: 700, cursor: 'pointer', display: 'flex', alignItems: 'center', gap: 6 }}>
            <Check size={14} /> 应用模板
          </motion.button>
        )}
      </div>

      <div style={{ display: 'grid', gap: 12 }}>
        {TEMPLATES.map(tpl => {
          const isSelected = selected === tpl.id;
          const IconComp = tpl.icon === 'Shield' ? Shield : tpl.icon === 'Lock' ? Lock : tpl.icon === 'Zap' ? Zap : FileCode;
          return (
            <motion.div key={tpl.id}
              whileHover={{ scale: 1.01 }} whileTap={{ scale: 0.99 }}
              onClick={() => { setSelected(tpl.id); setStep('confirm'); }}
              style={{
                padding: '18px 20px', borderRadius: 14, cursor: 'pointer',
                background: isSelected ? 'rgba(108,92,231,0.12)' : 'rgba(255,255,255,0.03)',
                border: isSelected ? '1px solid rgba(108,92,231,0.4)' : '1px solid rgba(255,255,255,0.06)',
                transition: 'all 0.2s',
                position: 'relative',
              }}>
              {isSelected && (
                <motion.div initial={{ scale: 0 }} animate={{ scale: 1 }}
                  style={{ position: 'absolute', top: 12, right: 12, width: 22, height: 22, borderRadius: '50%', background: '#6c5ce7', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                  <Check size={12} color="#fff" />
                </motion.div>
              )}
              <div style={{ display: 'flex', alignItems: 'flex-start', gap: 12 }}>
                <div style={{
                  width: 40, height: 40, borderRadius: 12,
                  background: isSelected ? 'rgba(108,92,231,0.2)' : 'rgba(255,255,255,0.05)',
                  display: 'flex', alignItems: 'center', justifyContent: 'center',
                }}>
                  <IconComp size={20} color={isSelected ? '#6c5ce7' : '#8892b0'} />
                </div>
                <div style={{ flex: 1 }}>
                  <div style={{ fontWeight: 700, fontSize: 14, color: '#fff', marginBottom: 4 }}>{tpl.name}</div>
                  <div style={{ fontSize: 12, color: '#5a6a8a', lineHeight: 1.5 }}>{tpl.desc}</div>
                  <div style={{ display: 'flex', gap: 6, marginTop: 10, flexWrap: 'wrap' }}>
                    {tpl.stages.map(s => (
                      <span key={s.order} style={{
                        fontSize: 10, padding: '2px 10px', borderRadius: 8,
                        background: isSelected ? 'rgba(108,92,231,0.15)' : 'rgba(255,255,255,0.05)',
                        color: isSelected ? '#6c5ce7' : '#8892b0',
                        display: 'flex', alignItems: 'center', gap: 4,
                      }}>
                        {s.order}. {s.name} <ArrowRight size={8} />
                      </span>
                    ))}
                  </div>
                </div>
              </div>
            </motion.div>
          );
        })}
      </div>
    </div>
  );
}
