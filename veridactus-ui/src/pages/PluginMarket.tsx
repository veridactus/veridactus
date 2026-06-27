// 插件市场 — 独立页面，浏览和启用治理插件
import { useState, useEffect } from 'react';
import { motion } from 'framer-motion';
import { Package, Search, Zap, Activity, ToggleLeft, ToggleRight } from 'lucide-react';

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

const typeIcons: Record<string, any> = { native: Zap, sidecar: Activity };

export default function PluginMarket() {
  const [plugins, setPlugins] = useState<Plugin[]>([]); const [search, setSearch] = useState(''); const [loading, setLoading] = useState(true);
  useEffect(() => { fetch('/api/v1/plugins').then(r => r.json()).then(d => { const list = d?.plugins || d || []; setPlugins(list.length > 0 ? list : DEFAULT_PLUGINS); }).catch(() => setPlugins(DEFAULT_PLUGINS)).finally(() => setLoading(false)); }, []);

  const togglePlugin = async (id: string) => {
    setPlugins(prev => prev.map(p => p.id === id ? { ...p, enabled: !p.enabled } : p));
    try { await fetch(`/api/v1/plugins/${id}`, { method: 'PUT', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ enabled: !plugins.find(p => p.id === id)?.enabled }) }); } catch {}
  };

  const filtered = plugins.filter(p => !search || p.name.toLowerCase().includes(search.toLowerCase()) || p.description.toLowerCase().includes(search.toLowerCase()));

  return (
    <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="p-8 max-w-[900px] min-h-full font-sans" style={{ background: 'linear-gradient(180deg, #0B0F19 0%, #131633 100%)' }}>
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-xl font-bold text-white"><Package size={22} className="inline align-sub mr-2 text-[#6c5ce7]" />插件市场</h1>
          <p className="text-sm text-[#8892b0] mt-1">管理 VERIDACTUS 治理插件，一键启用/禁用</p>
        </div>
        <div className="relative">
          <Search size={16} className="absolute left-3 top-1/2 -translate-y-1/2 text-[#5a6a8a]" />
          <input value={search} onChange={e => setSearch(e.target.value)} placeholder="搜索插件..."
            className="py-2.5 pl-9 pr-3.5 rounded-xl text-sm text-white border outline-none w-[200px]" style={{ background: 'rgba(255,255,255,0.05)', borderColor: 'rgba(255,255,255,0.1)' }} />
        </div>
      </div>

      {loading ? <div className="text-center py-12 text-[#8892b0]">加载中...</div> : (
        <div className="grid gap-3">
          {filtered.map((p, i) => {
            const Icon = typeIcons[p.type] || Package;
            return (
              <motion.div key={p.id} initial={{ opacity: 0, y: 10 }} animate={{ opacity: 1, y: 0 }} transition={{ delay: i * 0.03 }}
                className="py-[18px] px-5 rounded-2xl flex items-center gap-3.5 transition-all" style={{
                  background: p.enabled ? 'rgba(108,92,231,0.06)' : 'rgba(255,255,255,0.02)',
                  border: p.enabled ? '1px solid rgba(108,92,231,0.2)' : '1px solid rgba(255,255,255,0.05)',
                }}>
                <div className="w-11 h-11 rounded-xl flex items-center justify-center flex-shrink-0" style={{ background: p.enabled ? 'rgba(108,92,231,0.15)' : 'rgba(255,255,255,0.05)' }}>
                  <Icon size={20} color={p.enabled ? '#6c5ce7' : '#5a6a8a'} />
                </div>
                <div className="flex-1">
                  <div className="flex items-center gap-2 mb-1">
                    <span className="font-bold text-sm text-white">{p.name}</span>
                    <span className="text-[10px] py-px px-2 rounded-md" style={{ background: p.type === 'native' ? 'rgba(0,212,170,0.15)' : 'rgba(116,185,255,0.15)', color: p.type === 'native' ? '#00d4aa' : '#74b9ff' }}>{p.type}</span>
                    <span className="text-[10px] text-[#5a6a8a]">v{p.version}</span>
                  </div>
                  <div className="text-xs text-[#8892b0] leading-relaxed">{p.description}</div>
                </div>
                <button onClick={() => togglePlugin(p.id)} className="bg-transparent border-none cursor-pointer p-1">
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