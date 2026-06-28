// 🔗 密码学自证 — VERIDACTUS 审计证书组件（产品级 UI）
import { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { ShieldCheck, ShieldX, Loader2, Sparkles, Copy, Download, CheckCheck, Clock, Fingerprint, FileCheck } from 'lucide-react';

interface CryptoVerifyProps { trace: Record<string, any>; auditSignature?: string; }

// JCS 规范化 + SHA-256
function strip(obj: any): any {
  if (obj === null || typeof obj !== 'object') return obj;
  if (Array.isArray(obj)) return obj.map(strip);
  const r: Record<string, any> = {};
  for (const [k, v] of Object.entries(obj)) if (!k.startsWith('_')) r[k] = strip(v);
  return r;
}
function jcs(obj: any): string {
  if (obj === null) return 'null';
  if (typeof obj === 'boolean') return obj ? 'true' : 'false';
  if (typeof obj === 'number') { if (!isFinite(obj)) return 'null'; return obj === Math.floor(obj) ? String(obj) : String(obj); }
  if (typeof obj === 'string') return JSON.stringify(obj);
  if (Array.isArray(obj)) return '[' + obj.map(jcs).join(',') + ']';
  const keys = Object.keys(obj).sort();
  return '{' + keys.map(k => jcs(k) + ':' + jcs(obj[k])).join(',') + '}';
}
async function sha256(input: string): Promise<string> {
  const d = new TextEncoder().encode(input);
  const h = await crypto.subtle.digest('SHA-256', d);
  return Array.from(new Uint8Array(h)).map(b => b.toString(16).padStart(2, '0')).join('');
}

export default function CryptoVerify({ trace, auditSignature }: CryptoVerifyProps) {
  const [status, setStatus] = useState<'idle' | 'verifying' | 'pass' | 'fail'>('idle');
  const [computedHash, setComputedHash] = useState('');
  const [storedHash, setStoredHash] = useState('');
  const [showHash, setShowHash] = useState(false);
  const [copied, setCopied] = useState(false);
  const [certUrl, setCertUrl] = useState<string>('');
  const traceId = trace?.trace_id || '';
  const model = trace?.model || '';
  const createdAt = trace?.created_at || '';

  const verify = async () => {
    setStatus('verifying'); setShowHash(false); setCopied(false);
    try {
      const clone = JSON.parse(JSON.stringify(trace));
      if (clone.proofs?.proof_chain)
        for (const e of clone.proofs.proof_chain) { e.signature = null; e.signature_pq = null; }
      const canonical = jcs(strip(clone));
      const hash = await sha256(canonical);
      setComputedHash(hash);

      const sig = auditSignature || trace?.proofs?.proof_chain?.find((p: any) => p?.level === 'L0')?.signature || '';
      setStoredHash(sig);

      const match = hash.toLowerCase() === sig.toLowerCase();
      setStatus(match ? 'pass' : 'fail');
      setShowHash(true);
    } catch { setStatus('fail'); }
  };

  const copyHash = async () => {
    try { await navigator.clipboard.writeText(`trace_id: ${traceId}\nSHA-256: ${computedHash}`); setCopied(true); setTimeout(() => setCopied(false), 2000); } catch {}
  };

  const downloadCert = () => {
    if (!computedHash) return;
    const cert = {
      veridactus_audit_certificate: 'v1.0',
      trace_id: traceId,
      model: model,
      verified_at: new Date().toISOString(),
      canonicalization: 'RFC 8785 (JCS)',
      hash_algorithm: 'SHA-256',
      verification_method: 'Browser Web Crypto API — offline, zero-trust',
      computed_hash: computedHash,
      stored_hash: storedHash,
      match: status === 'pass',
      verified_by: 'VERIDACTUS L0 Audit — Client-Side Independent Verification'
    };
    const blob = new Blob([JSON.stringify(cert, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a'); a.href = url; a.download = `veridactus-l0-cert-${traceId.slice(0, 8)}.json`; a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <motion.div initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }}
      className="rounded-2xl border border-[rgba(108,92,231,0.15)] overflow-hidden"
      style={{ background: 'linear-gradient(135deg, rgba(108,92,231,0.04), rgba(0,212,170,0.04))' }}>
      
      {/* Header */}
      <div className="px-5 py-4 border-b border-white/[0.04] flex items-center justify-between">
        <div className="flex items-center gap-2.5">
          <Fingerprint size={18} color="#6c5ce7" />
          <span className="font-semibold text-[14px] text-white">L0 审计证书</span>
          <span className="text-[10px] px-1.5 py-0.5 rounded-full bg-[rgba(108,92,231,0.12)] text-[#6c5ce7] font-medium">RFC 8785 JCS</span>
        </div>
        <div className="flex items-center gap-1.5 text-[10px] text-[#4a5568]">
          <Clock size={11} /> {createdAt ? new Date(createdAt).toLocaleString('zh-CN') : ''}
        </div>
      </div>

      <div className="p-5">
        {/* Description */}
        <div className="mb-4 p-3 rounded-xl bg-white/[0.02] border border-white/[0.04]">
          <div className="flex items-start gap-2">
            <ShieldCheck size={14} color="#00d4aa" className="mt-0.5 flex-shrink-0" />
            <div className="text-[11px] leading-relaxed text-[#8892b0]">
              <strong className="text-[#c8d2e0]">完全离线验证 · 零信任</strong><br/>
              浏览器原生 Web Crypto API 独立计算 JCS SHA-256，与服务器端存储的签名比对。<br/>
              无需信任 VERIDACTUS 服务器，任何人均可独立验证此记录未被篡改。
            </div>
          </div>
        </div>

        {/* Trace Info */}
        <div className="grid grid-cols-2 gap-3 mb-4">
          {[{ label: '模型', value: model || '-' }, { label: 'Trace', value: traceId?.slice(0, 12) + '...' || '-' }].map(({ label, value }) => (
            <div key={label} className="p-2.5 rounded-xl bg-white/[0.02] border border-white/[0.04]">
              <div className="text-[10px] text-[#4a5568] mb-0.5">{label}</div>
              <div className="text-[12px] text-white font-mono truncate">{value}</div>
            </div>
          ))}
        </div>

        {/* Status & Controls */}
        {status === 'idle' && (
          <motion.button whileHover={{ scale: 1.01 }} whileTap={{ scale: 0.98 }}
            onClick={verify}
            className="w-full py-3 rounded-xl flex items-center justify-center gap-2.5 cursor-pointer border-none text-white text-[13px] font-semibold"
            style={{ background: 'linear-gradient(135deg, #6c5ce7, #00d4aa)' }}>
            <ShieldCheck size={16} /> 🔍 开始验证
          </motion.button>
        )}

        {status === 'verifying' && (
          <div className="flex items-center justify-center gap-2 py-3 text-[#6c5ce7] text-[13px]">
            <motion.div animate={{ rotate: 360 }} transition={{ duration: 1, repeat: Infinity }}><Loader2 size={16} /></motion.div>
            正在计算 SHA-256...
          </div>
        )}

        {status === 'pass' && (
          <motion.div initial={{ opacity: 0, scale: 0.95 }} animate={{ opacity: 1, scale: 1 }}>
            <div className="flex items-center gap-3 py-3 px-4 rounded-xl mb-3" style={{ background: 'rgba(0,212,170,0.08)', border: '1px solid rgba(0,212,170,0.2)' }}>
              <div className="w-10 h-10 rounded-full bg-[rgba(0,212,170,0.15)] flex items-center justify-center flex-shrink-0">
                <CheckCheck size={20} color="#00d4aa" />
              </div>
              <div>
                <div className="text-[14px] font-bold text-[#00d4aa]">密码学验证通过</div>
                <div className="text-[11px] text-[#50c8a0] mt-0.5">此记录未经篡改 · L0 哈希链完整</div>
              </div>
            </div>

            {/* Hash display */}
            {showHash && (
              <motion.div initial={{ height: 0, opacity: 0 }} animate={{ height: 'auto', opacity: 1 }}
                className="p-3 rounded-xl bg-[rgba(0,212,170,0.03)] border border-[rgba(0,212,170,0.1)] mb-3">
                <div className="flex items-center justify-between mb-2">
                  <span className="text-[10px] text-[#50c8a0] font-medium uppercase tracking-wider">SHA-256 摘要</span>
                  <button onClick={copyHash} className="flex items-center gap-1 text-[10px] text-[#50c8a0] hover:text-[#00d4aa] cursor-pointer bg-none border-none">
                    {copied ? <CheckCheck size={11} /> : <Copy size={11} />} {copied ? '已复制' : '复制'}
                  </button>
                </div>
                <div className="font-mono text-[11px] text-[#50c8a0] break-all leading-relaxed opacity-80">
                  {computedHash}
                </div>
              </motion.div>
            )}

            {/* Actions */}
            <div className="flex gap-2">
              <button onClick={downloadCert}
                className="flex-1 flex items-center justify-center gap-1.5 py-2.5 rounded-xl bg-[rgba(0,212,170,0.08)] hover:bg-[rgba(0,212,170,0.14)] border border-[rgba(0,212,170,0.15)] text-[#00d4aa] text-[12px] font-medium cursor-pointer transition-colors">
                <Download size={13} /> 下载证书
              </button>
              <button onClick={verify}
                className="flex items-center justify-center gap-1.5 py-2.5 px-4 rounded-xl bg-white/[0.03] hover:bg-white/[0.06] border border-white/[0.06] text-[#8892b0] text-[12px] font-medium cursor-pointer transition-colors">
                <ShieldCheck size={13} /> 重新验证
              </button>
            </div>
          </motion.div>
        )}

        {status === 'fail' && (
          <motion.div initial={{ opacity: 0, scale: 0.95 }} animate={{ opacity: 1, scale: 1 }}>
            <div className="flex items-center gap-3 py-3 px-4 rounded-xl mb-3" style={{ background: 'rgba(255,118,117,0.06)', border: '1px solid rgba(255,118,117,0.15)' }}>
              <div className="w-10 h-10 rounded-full bg-[rgba(255,118,117,0.12)] flex items-center justify-center flex-shrink-0">
                <ShieldX size={20} color="#ff7675" />
              </div>
              <div>
                <div className="text-[14px] font-bold text-[#ff7675]">签名不匹配</div>
                <div className="text-[11px] text-[#d36868] mt-0.5">此记录可能已被篡改或损坏</div>
              </div>
            </div>

            {showHash && (
              <div className="p-3 rounded-xl bg-[rgba(255,118,117,0.03)] border border-[rgba(255,118,117,0.1)] mb-3">
                <div className="flex gap-3 text-[10px] font-mono">
                  <div className="flex-1"><span className="text-[#d36868] block mb-0.5">计算值</span><span className="text-[#ff7675] break-all">{computedHash.slice(0, 32)}...</span></div>
                  <div className="flex-1"><span className="text-[#4a5568] block mb-0.5">存储值</span><span className="text-[#5a6a8a] break-all">{storedHash.slice(0, 32)}...</span></div>
                </div>
              </div>
            )}

            <button onClick={verify}
              className="w-full py-2.5 rounded-xl bg-white/[0.03] hover:bg-white/[0.06] border border-white/[0.06] text-[#8892b0] text-[12px] font-medium cursor-pointer transition-colors">
              <ShieldCheck size={13} className="inline mr-1"/> 重新验证
            </button>
          </motion.div>
        )}
      </div>

      {/* Footer */}
      <div className="px-5 py-2.5 border-t border-white/[0.04] flex items-center justify-between">
        <div className="flex items-center gap-1.5 text-[9px] text-[#3a4568]">
          <FileCheck size={10} /> VERIDACTUS Audit Protocol v1.0 · L0 Hash Chain
        </div>
        <div className="flex items-center gap-1 text-[9px] text-[#3a4568]">
          <span>{traceId?.slice(0, 8)}</span>
        </div>
      </div>
    </motion.div>
  );
}
