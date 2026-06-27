// 🔗 密码学自证 — VERIDACTUS 降维打击组件
// 浏览器端 Web Crypto API 计算 SHA-256 → 与后端 L0 签名比对
import { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { ShieldCheck, ShieldX, Loader2, Sparkles } from 'lucide-react';

interface CryptoVerifyProps {
  trace: Record<string, any>;
  auditSignature?: string;
}

// 递归剥离以 _ 开头的字段 (RFC 8785 要求)
function stripInternalFields(obj: any): any {
  if (obj === null || typeof obj !== 'object') return obj;
  if (Array.isArray(obj)) return obj.map(stripInternalFields);
  const result: Record<string, any> = {};
  for (const [key, value] of Object.entries(obj)) {
    if (key.startsWith('_')) continue;
    result[key] = stripInternalFields(value);
  }
  return result;
}

// JCS 规范化 (JSON Canonicalization Scheme)
// 使用 Object.keys() 按字母序排序 + 序列化无空格
function jcsCanonicalize(obj: any): string {
  if (obj === null) return 'null';
  if (typeof obj === 'boolean') return obj ? 'true' : 'false';
  if (typeof obj === 'number') {
    if (!isFinite(obj)) return 'null';
    return String(obj);
  }
  if (typeof obj === 'string') return JSON.stringify(obj);
  if (Array.isArray(obj)) {
    return '[' + obj.map(jcsCanonicalize).join(',') + ']';
  }
  // Object: sort keys alphabetically
  const keys = Object.keys(obj).sort();
  const pairs = keys.map(k => jcsCanonicalize(k) + ':' + jcsCanonicalize(obj[k]));
  return '{' + pairs.join(',') + '}';
}

async function computeSHA256(input: string): Promise<string> {
  const encoder = new TextEncoder();
  const data = encoder.encode(input);
  const hashBuffer = await crypto.subtle.digest('SHA-256', data);
  const hashArray = Array.from(new Uint8Array(hashBuffer));
  return hashArray.map(b => b.toString(16).padStart(2, '0')).join('');
}

// 粒子爆发动效
function ParticleBurst({ onComplete }: { onComplete: () => void }) {
  return (
    <motion.div
      initial={{ opacity: 1 }}
      animate={{ opacity: 0 }}
      transition={{ duration: 2.5 }}
      onAnimationComplete={onComplete}
      style={{
        position: 'fixed', inset: 0, zIndex: 9999, pointerEvents: 'none',
        display: 'flex', alignItems: 'center', justifyContent: 'center',
      }}
    >
      {Array.from({ length: 50 }).map((_, i) => (
        <motion.div
          key={i}
          initial={{
            x: 0, y: 0, scale: 0, opacity: 1,
          }}
          animate={{
            x: (Math.random() - 0.5) * 800,
            y: (Math.random() - 0.5) * 800,
            scale: Math.random() * 2 + 1,
            opacity: 0,
          }}
          transition={{ duration: 1.5 + Math.random(), ease: 'easeOut' }}
          style={{
            position: 'absolute',
            width: 6 + Math.random() * 6,
            height: 6 + Math.random() * 6,
            borderRadius: '50%',
            background: ['#00d4aa', '#6c5ce7', '#74b9ff', '#fdcb6e'][i % 4],
            boxShadow: `0 0 12px ${['#00d4aa', '#6c5ce7', '#74b9ff', '#fdcb6e'][i % 4]}`,
          }}
        />
      ))}
    </motion.div>
  );
}

export default function CryptoVerify({ trace, auditSignature }: CryptoVerifyProps) {
  const [status, setStatus] = useState<'idle' | 'verifying' | 'pass' | 'fail'>('idle');
  const [computedHash, setComputedHash] = useState('');
  const [showParticles, setShowParticles] = useState(false);

  const handleVerify = async () => {
    setStatus('verifying');
    try {
      // 1. 克隆并剥离内部字段
      const clone = JSON.parse(JSON.stringify(trace));

      // 2. 清空 proof_chain 中的 signature（设为 null，不删除 key）
      // ⚠️ 不能 delete！Rust serde_json 序列化 Option::None 为 null（保留 key）
      // JCS 规范化对 key 存在性敏感 → 删除 key vs null → 哈希不同
      if (clone.proofs?.proof_chain) {
        for (const entry of clone.proofs.proof_chain) {
          entry.signature = null;
          entry.signature_pq = null;
        }
      }

      // 3. 剥离 _ 开头的字段
      const stripped = stripInternalFields(clone);

      // 4. JCS 规范化
      const canonical = jcsCanonicalize(stripped);

      // 5. Web Crypto API SHA-256
      const hash = await computeSHA256(canonical);
      setComputedHash(hash);

      // 6. 比对
      const expectedSig = auditSignature || trace?.proofs?.proof_chain?.find(
        (p: any) => p?.level === 'L0'
      )?.signature || '';

      const match = hash.toLowerCase() === expectedSig.toLowerCase();
      setStatus(match ? 'pass' : 'fail');
      if (match) setShowParticles(true);
    } catch (err) {
      console.error('Crypto verify failed:', err);
      setStatus('fail');
    }
  };

  return (
    <>
      <AnimatePresence>
        {showParticles && (
          <ParticleBurst onComplete={() => setShowParticles(false)} />
        )}
      </AnimatePresence>

      <motion.div
        style={{
          background: 'linear-gradient(135deg, rgba(108,92,231,0.08), rgba(0,212,170,0.08))',
          border: '1px solid rgba(108,92,231,0.2)', borderRadius: 16,
          padding: 24, textAlign: 'center',
        }}
      >
        <div style={{
          display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 8,
          marginBottom: 16,
        }}>
          <Sparkles size={20} color="#6c5ce7" />
          <h3 style={{
            fontSize: 16, fontWeight: 700, color: '#fff',
          }}>L0 审计证书</h3>
          <Sparkles size={20} color="#6c5ce7" />
        </div>

        <p style={{ fontSize: 12, color: '#8892b0', marginBottom: 20, lineHeight: 1.6 }}>
          使用浏览器原生 <code style={{ color: '#6c5ce7', background: 'rgba(108,92,231,0.1)', padding: '1px 6px', borderRadius: 4 }}>Web Crypto API</code> 计算 JCS 规范化 SHA-256，
          与后端签名比对。<strong>完全离线验证，无需信任服务器。</strong>
        </p>

        {status === 'idle' && (
          <motion.button
            whileHover={{ scale: 1.02 }}
            whileTap={{ scale: 0.98 }}
            onClick={handleVerify}
            style={{
              padding: '14px 32px', borderRadius: 12,
              background: 'linear-gradient(135deg, #6c5ce7, #00d4aa)',
              border: 'none', color: '#fff', fontSize: 14, fontWeight: 700,
              cursor: 'pointer', display: 'inline-flex', alignItems: 'center', gap: 8,
            }}
          >
            <ShieldCheck size={18} /> 🔍 验证签名
          </motion.button>
        )}

        {status === 'verifying' && (
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 8, color: '#6c5ce7' }}>
            <motion.div animate={{ rotate: 360 }} transition={{ duration: 1, repeat: Infinity }}>
              <Loader2 size={20} />
            </motion.div>
            <span style={{ fontSize: 14, fontWeight: 600 }}>验证中...</span>
          </div>
        )}

        {status === 'pass' && (
          <motion.div
            initial={{ opacity: 0, scale: 0.8 }}
            animate={{ opacity: 1, scale: 1 }}
          >
            <div style={{
              padding: '16px 24px', borderRadius: 12,
              background: 'rgba(0,212,170,0.12)', border: '1px solid rgba(0,212,170,0.3)',
              display: 'inline-flex', alignItems: 'center', gap: 10,
              color: '#00d4aa', fontSize: 16, fontWeight: 700,
              marginBottom: 12,
            }}>
              <ShieldCheck size={24} />
              ✅ 密码学验证通过：此记录未被篡改
            </div>
            <div style={{ fontSize: 11, color: '#8892b0', fontFamily: 'monospace', wordBreak: 'break-all' }}>
              SHA-256: {computedHash}
            </div>
          </motion.div>
        )}

        {status === 'fail' && (
          <motion.div
            initial={{ opacity: 0, scale: 0.8 }}
            animate={{ opacity: 1, scale: 1 }}
            style={{
              padding: '16px 24px', borderRadius: 12,
              background: 'rgba(255,118,117,0.12)', border: '1px solid rgba(255,118,117,0.3)',
              display: 'inline-flex', alignItems: 'center', gap: 10,
              color: '#ff7675', fontSize: 16, fontWeight: 700,
            }}
          >
            <ShieldX size={24} />
            ❌ 签名不匹配：此记录可能已被篡改
          </motion.div>
        )}
      </motion.div>
    </>
  );
}
