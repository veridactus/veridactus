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

interface ReportResult {
  report_id: string;
  regulation: string;
  trace_count: number;
  merkle_root: string;
  signature: string;
  pdf_path?: string;
  all_satisfied?: boolean;
}

export default function ComplianceReport() {
  const [regulation, setRegulation] = useState('EU_AI_ACT');
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<ReportResult | null>(null);
  const [error, setError] = useState('');
  const token = getStoredToken();

  const handleGenerate = async () => {
    if (!token) { setError('请先登录'); return; }
    setLoading(true); setError(''); setResult(null);

    try {
      // 从数据面获取 traces
      const tracesRes = await fetch('/v1/traces?limit=50', {
        headers: { Authorization: `Bearer ${token}` },
      });
      const tracesData = await tracesRes.json();
      const traces = tracesData?.traces || tracesData || [];

      // 调用合规报告生成
      const res = await fetch(`${API}/api/v1/compliance/reports`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify({
          regulation,
          trace_ids: traces.map((t: any) => t.trace_id || t.id).slice(0, 50),
          traces_data: traces.slice(0, 50),
        }),
      });

      const data = await res.json();
      if (!res.ok) { setError(data.message || data.error || '生成失败'); return; }
      setResult(data);
    } catch (e: any) {
      setError(e.message || '网络错误');
    } finally {
      setLoading(false);
    }
  };

  const handleDownloadPDF = async () => {
    if (!result?.report_id || !token) return;
    try {
      const res = await fetch(`${API}/api/v1/compliance/reports?job_id=${result.report_id}`, {
        headers: { Authorization: `Bearer ${token}` },
      });
      const data = await res.json();
      if (data.pdf_url) {
        window.open(data.pdf_url, '_blank');
      } else {
        // 如果后端返回 JSON 元数据，触发浏览器下载
        const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url; a.download = `${result.report_id}.json`; a.click();
        URL.revokeObjectURL(url);
      }
    } catch {
      setError('下载失败');
    }
  };

  return (
    <div style={{
      minHeight: '100%', padding: 32,
      background: 'linear-gradient(180deg, #0B0F19 0%, #131633 100%)',
      fontFamily: 'system-ui, sans-serif', color: '#fff',
    }}>
      <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }}
        style={{ maxWidth: 700, margin: '0 auto' }}>

        <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 8 }}>
          <Shield size={28} color="#6c5ce7" />
          <h1 style={{ fontSize: 22, fontWeight: 700 }}>合规报告生成</h1>
        </div>
        <p style={{ color: '#8892b0', fontSize: 13, marginBottom: 32 }}>
          生成带密码学签名的合规审计报告（Merkle Root + 数字签名），支持离线验证
        </p>

        {/* 法规选择 */}
        <div style={{ marginBottom: 24 }}>
          <label style={{ fontSize: 13, fontWeight: 600, color: '#8892b0', marginBottom: 8, display: 'block' }}>
            选择合规法规
          </label>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 10 }}>
            {REGULATIONS.map(r => (
              <motion.div key={r.id}
                whileHover={{ scale: 1.02 }} whileTap={{ scale: 0.98 }}
                onClick={() => setRegulation(r.id)}
                style={{
                  padding: '14px 12px', borderRadius: 12, cursor: 'pointer', textAlign: 'center',
                  background: regulation === r.id
                    ? 'linear-gradient(135deg, rgba(108,92,231,0.2), rgba(108,92,231,0.1))'
                    : 'rgba(255,255,255,0.03)',
                  border: regulation === r.id
                    ? '1px solid rgba(108,92,231,0.4)'
                    : '1px solid rgba(255,255,255,0.06)',
                  transition: 'all 0.2s',
                }}>
                <div style={{ fontWeight: 700, fontSize: 14, color: regulation === r.id ? '#6c5ce7' : '#fff' }}>
                  {r.label}
                </div>
                <div style={{ fontSize: 11, color: '#5a6a8a', marginTop: 4 }}>{r.desc}</div>
              </motion.div>
            ))}
          </div>
        </div>

        {/* 生成按钮 */}
        <motion.button
          whileHover={{ scale: 1.02 }} whileTap={{ scale: 0.98 }}
          onClick={handleGenerate} disabled={loading}
          style={{
            width: '100%', padding: '14px', borderRadius: 12,
            background: loading ? 'rgba(255,255,255,0.05)' : 'linear-gradient(135deg, #6c5ce7, #00d4aa)',
            border: 'none', color: '#fff', fontSize: 15, fontWeight: 700,
            cursor: loading ? 'not-allowed' : 'pointer',
            display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 8,
          }}>
          {loading ? <><Loader2 size={18} style={{ animation: 'spin 1s linear infinite' }} /> 生成中...</> :
            <><FileText size={18} /> 生成合规报告</>}
        </motion.button>

        {error && (
          <div style={{
            marginTop: 16, padding: '12px 16px', borderRadius: 10,
            background: 'rgba(255,107,107,0.1)', border: '1px solid rgba(255,107,107,0.3)',
            display: 'flex', alignItems: 'center', gap: 8, color: '#ff6b6b', fontSize: 13,
          }}>
            <AlertTriangle size={16} /> {error}
          </div>
        )}

        {/* 结果展示 */}
        {result && (
          <motion.div initial={{ opacity: 0, y: 10 }} animate={{ opacity: 1, y: 0 }}
            style={{
              marginTop: 24, padding: 24, borderRadius: 16,
              background: 'rgba(255,255,255,0.03)', border: '1px solid rgba(255,255,255,0.08)',
            }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 16 }}>
              <div style={{
                width: 40, height: 40, borderRadius: 12,
                background: result.all_satisfied ? 'rgba(0,212,170,0.15)' : 'rgba(255,107,107,0.15)',
                display: 'flex', alignItems: 'center', justifyContent: 'center',
              }}>
                <CheckCircle size={22} color={result.all_satisfied ? '#00d4aa' : '#ff6b6b'} />
              </div>
              <div>
                <div style={{ fontWeight: 700, fontSize: 15 }}>
                  {result.all_satisfied ? '所有合规条款已满足' : '部分条款未满足'}
                </div>
                <div style={{ fontSize: 12, color: '#8892b0' }}>Report ID: {result.report_id}</div>
              </div>
            </div>

            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12, marginBottom: 20 }}>
              {[
                ['法规', result.regulation],
                ['Trace 数', result.trace_count],
                ['Merkle Root', (result.merkle_root || '').slice(0, 16) + '...'],
                ['签名', (result.signature || '').slice(0, 16) + '...'],
              ].map(([k, v]) => (
                <div key={k} style={{ fontSize: 12 }}>
                  <span style={{ color: '#5a6a8a' }}>{k}: </span>
                  <span style={{ color: '#e0e6f0', fontFamily: 'monospace' }}>{v}</span>
                </div>
              ))}
            </div>

            <motion.button
              whileHover={{ scale: 1.02 }} whileTap={{ scale: 0.98 }}
              onClick={handleDownloadPDF}
              style={{
                width: '100%', padding: '12px', borderRadius: 10,
                background: 'rgba(0,212,170,0.12)', border: '1px solid rgba(0,212,170,0.3)',
                color: '#00d4aa', fontSize: 14, fontWeight: 600, cursor: 'pointer',
                display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 8,
              }}>
              <Download size={16} /> 下载报告与离线验证脚本
            </motion.button>
          </motion.div>
        )}
      </motion.div>
    </div>
  );
}
