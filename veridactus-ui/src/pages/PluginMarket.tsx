// 插件市场 — 独立页面，浏览和启用治理插件
import { useState, useEffect } from 'react';
import { motion } from 'framer-motion';
import { Package, Search, Shield, Eye, Activity, Zap, ToggleLeft, ToggleRight } from 'lucide-react';

interface Plugin { id: string; name: string; type: string; version: string; description: string; enabled: boolean; config: any }

const DEFAULT_PLUGINS: Plugin[] = [
  { id: 'budget-guard', name: 'Budget Guard', type: 'native', version: '1.0.0', description: '微美元预算守卫，每10 token 触发 Redis Lua 原子扣减，超限熔断', enabled: true, config: { limit_usd: 10.0 } },
  { id: 'pii-detector', name: 'PII Detector', type: 'native', version: '0.2.1', description: 'PII 检测与脱敏：email/phone/id_card/credit_card/api_key', enabled: true, config: { enabled: true } },
  { id: 'input-sanitizer', name: 'Input Sanitizer', type: 'native', version: '1.0.0', description: '输入净化：注入攻击检测（SQL/XSS/命令注入）', enabled: true, config: {} },
  { id: 'response-validator', name: 'Response Validator', type: 'native', version: '1.0.0', description: '响应验证：安全评分 + 幻觉检测', enabled: true, config: {} },
  { id: 'content-safety-scorer', name: 'Content Safety Scorer', type: 'sidecar', version: '0.2.1', description: 'C-SafeGen 内容安全评分（Python Worker）', enabled: true, config: {} },
  { id: 'toxicity-classifier', name: 'Toxicity Classifier', type: 'sidecar', version: '0.2.1', description: '毒性分类器（可接入 detoxify 模型）', enabled: false, config: {} },
  { id: 'bias-detector', name: 'Bias Detector', type: 'sidecar', version: '0.2.1', description: '偏见检测器（可接入公平性检查模型）', enabled: false, config: {} },
];

export default function PluginMarket() {
  const [plugins, setPlugins] = useState<Plugin[]>([]);
  const [search, setSearch] = useState('');
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetch('/api/v1/plugins')
      .then(r => r.json())
      .then(d => {
        const list = d?.plugins || d || [];
        if (list.length > 0) {
          setPlugins(list);
        } else {
          setPlugins(DEFAULT_PLUGINS);
        }
      })
      .catch(() => setPlugins(DEFAULT_PLUGINS))
      .finally(() => setLoading(false));
  }, []);

  const togglePlugin = async (id: string) => {
    setPlugins(prev => prev.map(p => p.id === id ? { ...p, enabled: !p.enabled } : p));
    // 异步保存到后端
    try {
      await fetch(`/api/v1/plugins/${id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ enabled: !plugins.find(p => p.id === id)?.enabled }),
      });
    } catch {}
  };

  const filtered = plugins.filter(p =>
    !search || p.name.toLowerCase().includes(search.toLowerCase()) || p.description.toLowerCase().includes(search.toLowerCase())
  );

  const typeIcons: Record<string, any> = { native: Zap, sidecar: Activity };

  return (
    <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }}
      style={{ padding: 32, maxWidth: 900, minHeight: '100%',
        background: 'linear-gradient(180deg, #0B0F19 0%, #131633 100%)',
        fontFamily: 'system-ui, sans-serif' }}>
      
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 24 }}>
        <div>
          <h1 style={{ fontSize: 22, fontWeight: 700, color: '#fff', margin: 0 }}>
            <Package size={22} style={{ verticalAlign: -3, marginRight: 8, color: '#6c5ce7' }} />
            插件市场
          </h1>
          <p style={{ color: '#8892b0', fontSize: 13, marginTop: 4 }}>管理 VERIDACTUS 治理插件，一键启用/禁用</p>
        </div>
        
        <div style={{ position: 'relative' }}>
          <Search size={16} style={{ position: 'absolute', left: 12, top: '50%', transform: 'translateY(-50%)', color: '#5a6a8a' }} />
          <input value={search} onChange={e => setSearch(e.target.value)} placeholder="搜索插件..."
            style={{
              padding: '10px 14px 10px 36px', borderRadius: 12, fontSize: 13, color: '#fff',
              background: 'rgba(255,255,255,0.05)', border: '1px solid rgba(255,255,255,0.1)',
              outline: 'none', width: 200,
            }} />
        </div>
      </div>

      {loading ? (
        <div style={{ textAlign: 'center', padding: 48, color: '#8892b0' }}>加载中...</div>
      ) : (
        <div style={{ display: 'grid', gap: 12 }}>
          {filtered.map((p, i) => {
            const Icon = typeIcons[p.type] || Package;
            return (
              <motion.div key={p.id}
                initial={{ opacity: 0, y: 10 }} animate={{ opacity: 1, y: 0 }} transition={{ delay: i * 0.03 }}
                style={{
                  padding: '18px 20px', borderRadius: 14,
                  background: p.enabled ? 'rgba(108,92,231,0.06)' : 'rgba(255,255,255,0.02)',
                  border: p.enabled ? '1px solid rgba(108,92,231,0.2)' : '1px solid rgba(255,255,255,0.05)',
                  display: 'flex', alignItems: 'center', gap: 14,
                  transition: 'all 0.2s',
                }}>
                <div style={{
                  width: 44, height: 44, borderRadius: 12,
                  background: p.enabled ? 'rgba(108,92,231,0.15)' : 'rgba(255,255,255,0.05)',
                  display: 'flex', alignItems: 'center', justifyContent: 'center', flexShrink: 0,
                }}>
                  <Icon size={20} color={p.enabled ? '#6c5ce7' : '#5a6a8a'} />
                </div>
                <div style={{ flex: 1 }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
                    <span style={{ fontWeight: 700, fontSize: 14, color: '#fff' }}>{p.name}</span>
                    <span style={{
                      fontSize: 10, padding: '1px 8px', borderRadius: 6,
                      background: p.type === 'native' ? 'rgba(0,212,170,0.15)' : 'rgba(116,185,255,0.15)',
                      color: p.type === 'native' ? '#00d4aa' : '#74b9ff',
                    }}>{p.type}</span>
                    <span style={{ fontSize: 10, color: '#5a6a8a' }}>v{p.version}</span>
                  </div>
                  <div style={{ fontSize: 12, color: '#8892b0', lineHeight: 1.5 }}>{p.description}</div>
                </div>
                <button onClick={() => togglePlugin(p.id)}
                  style={{ background: 'none', border: 'none', cursor: 'pointer', padding: 4 }}>
                  {p.enabled ? <ToggleRight size={28} color="#00d4aa" /> : <ToggleLeft size={28} color="#5a6a8a" />}
                </button>
              </motion.div>
            );
          })}
        </div>
      )}
    </motion.div>
  );
}
