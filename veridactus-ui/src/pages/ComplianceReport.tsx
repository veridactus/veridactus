/// <reference types="vite/client" />
// VERIDACTUS 合规报告生成页面
import { useState } from 'react';
import { motion } from 'framer-motion';
import { Shield, Download, FileText, Loader2, CheckCircle, AlertTriangle } from 'lucide-react';
import { getStoredToken } from '../auth/useAuth';

const API = (import.meta as any)?.env?.VITE_API_URL || '';
const REGULATIONS = [
  { id: 'EU_AI_ACT', label: 'EU AI Act', desc: '欧盟人工智能法案' },
  { id: 'GDPR', label: 'GDPR', desc: '通用数据保护条例' },
  { id: 'NIST_AI_600', label: 'NIST AI 600', desc: '美国 NIST AI 风险管理框架' },
];

interface ReportResult { report_id: string; regulation: string; trace_count: number; merkle_root: string; signature: string; pdf_path?: string; all_satisfied?: boolean; }

export default function ComplianceReport() {
  const [regulation, setRegulation] = useState('EU_AI_ACT');
  const [loading, setLoading] = useState(false); const [result, setResult] = useState<ReportResult | null>(null); const [error, setError] = useState('');
  const token = getStoredToken();

  const handleGenerate = async () => {
    if (!token) { setError('请先登录'); return; }
    setLoading(true); setError(''); setResult(null);
    try {
      const tr = await fetch('/v1/traces?limit=50', { headers: { Authorization: `Bearer ${token}` } }).then(r => r.json());
      const traces = tr?.traces || tr || [];
      const res = await fetch(`${API}/api/v1/compliance/reports`, { method: 'POST', headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${token}` }, body: JSON.stringify({ regulation, trace_ids: traces.map((t: any) => t.trace_id || t.id).slice(0, 50), traces_data: traces.slice(0, 50) }) });
      const data = await res.json();
      if (!res.ok) { setError(data.message || data.error || '生成失败'); return; }
      setResult(data);
    } catch (e: any) { setError(e.message || '网络错误'); } finally { setLoading(false); }
  };

  const handleDownloadPDF = async () => {
    if (!result?.report_id || !token) return;
    try {
      const res = await fetch(`${API}/api/v1/compliance/reports?job_id=${result.report_id}`, { headers: { Authorization: `Bearer ${token}` } });
      const data = await res.json();
      if (data.pdf_url) { window.open(data.pdf_url, '_blank'); }
      else { const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' }); const a = document.createElement('a'); a.href = URL.createObjectURL(blob); a.download = `${result.report_id}.json`; a.click(); }
    } catch { setError('下载失败'); }
  };

  return (
    <div className="min-h-full p-8 font-sans text-white" style={{ background: 'linear-gradient(180deg, #0B0F19 0%, #131633 100%)' }}>
      <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="max-w-[700px] mx-auto">
        <div className="flex items-center gap-3 mb-2"><Shield size={28} color="#6c5ce7" /><h1 className="text-xl font-bold">合规报告生成</h1></div>
        <p className="text-sm text-[#8892b0] mb-8">生成带密码学签名的合规审计报告（Merkle Root + 数字签名），支持离线验证</p>

        <div className="mb-6">
          <label className="text-sm font-semibold text-[#8892b0] mb-2 block">选择合规法规</label>
          <div className="grid grid-cols-3 gap-2.5">
            {REGULATIONS.map(r => (
              <motion.div key={r.id} whileHover={{ scale: 1.02 }} whileTap={{ scale: 0.98 }} onClick={() => setRegulation(r.id)}
                className="py-3.5 px-3 rounded-xl cursor-pointer text-center transition-all" style={{
                  background: regulation === r.id ? 'linear-gradient(135deg, rgba(108,92,231,0.2), rgba(108,92,231,0.1))' : 'rgba(255,255,255,0.03)',
                  border: regulation === r.id ? '1px solid rgba(108,92,231,0.4)' : '1px solid rgba(255,255,255,0.06)',
                }}>
                <div className="font-bold text-sm" style={{ color: regulation === r.id ? '#6c5ce7' : '#fff' }}>{r.label}</div>
                <div className="text-[11px] text-[#5a6a8a] mt-1">{r.desc}</div>
              </motion.div>
            ))}
          </div>
        </div>

        <motion.button whileHover={{ scale: 1.02 }} whileTap={{ scale: 0.98 }} onClick={handleGenerate} disabled={loading}
          className="w-full py-3.5 rounded-xl border-none text-white text-[15px] font-bold flex items-center justify-center gap-2 disabled:cursor-not-allowed" style={{
            background: loading ? 'rgba(255,255,255,0.05)' : 'linear-gradient(135deg, #6c5ce7, #00d4aa)',
          }}>
          {loading ? <><Loader2 size={18} className="animate-spin" /> 生成中...</> : <><FileText size={18} /> 生成合规报告</>}
        </motion.button>

        {error && <div className="mt-4 py-3 px-4 rounded-btn flex items-center gap-2 text-sm text-[#ff6b6b]" style={{ background: 'rgba(255,107,107,0.1)', border: '1px solid rgba(255,107,107,0.3)' }}><AlertTriangle size={16} /> {error}</div>}

        {result && (
          <motion.div initial={{ opacity: 0, y: 10 }} animate={{ opacity: 1, y: 0 }} className="mt-6 p-6 rounded-card" style={{ background: 'rgba(255,255,255,0.03)', border: '1px solid rgba(255,255,255,0.08)' }}>
            <div className="flex items-center gap-2.5 mb-4">
              <div className="w-10 h-10 rounded-xl flex items-center justify-center" style={{ background: result.all_satisfied ? 'rgba(0,212,170,0.15)' : 'rgba(255,107,107,0.15)' }}><CheckCircle size={22} color={result.all_satisfied ? '#00d4aa' : '#ff6b6b'} /></div>
              <div><div className="font-bold text-[15px]">{result.all_satisfied ? '所有合规条款已满足' : '部分条款未满足'}</div><div className="text-xs text-[#8892b0]">Report ID: {result.report_id}</div></div>
            </div>
            <div className="grid grid-cols-2 gap-3 mb-5 text-xs">
              {[['法规', result.regulation], ['Trace 数', result.trace_count], ['Merkle Root', (result.merkle_root || '').slice(0, 16) + '...'], ['签名', (result.signature || '').slice(0, 16) + '...']].map(([k, v]) => (
                <div key={k}><span className="text-[#5a6a8a]">{k}: </span><span className="text-[#e0e6f0] font-mono">{v}</span></div>
              ))}
            </div>
            <motion.button whileHover={{ scale: 1.02 }} whileTap={{ scale: 0.98 }} onClick={handleDownloadPDF}
              className="w-full py-3 rounded-btn border text-sm font-semibold flex items-center justify-center gap-2 cursor-pointer" style={{ background: 'rgba(0,212,170,0.12)', borderColor: 'rgba(0,212,170,0.3)', color: '#00d4aa' }}>
              <Download size={16} /> 下载报告与离线验证脚本
            </motion.button>
          </motion.div>
        )}
      </motion.div>
    </div>
  );
}