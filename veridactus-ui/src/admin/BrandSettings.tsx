// 品牌白标定制 — Logo上传 + 完整 CSS Variables 品牌注入
import { useState, useEffect, useRef, useCallback } from 'react';
import { motion } from 'framer-motion';
import { Upload, CheckCircle, Palette, Image, Loader2, X } from 'lucide-react';
import { toast } from '../components/ui/Toast';

const STORAGE_KEY = 'veridactus_brand';
const API = (import.meta as any)?.env?.VITE_API_URL || '';

export default function BrandSettings() {
  const [name, setName] = useState('');
  const [primaryColor, setPrimaryColor] = useState('#6c5ce7');
  const [logoUrl, setLogoUrl] = useState('');
  const [logoPreview, setLogoPreview] = useState('');
  const [saved, setSaved] = useState(false);
  const [saving, setSaving] = useState(false);
  const [uploading, setUploading] = useState(false);
  const fileRef = useRef<HTMLInputElement>(null);

  // 加载已有品牌设置
  useEffect(() => {
    const cached = localStorage.getItem(STORAGE_KEY);
    if (cached) {
      try {
        const b = JSON.parse(cached);
        if (b.name) setName(b.name);
        if (b.primary_color) setPrimaryColor(b.primary_color);
        if (b.logo_url) { setLogoUrl(b.logo_url); setLogoPreview(b.logo_url); }
      } catch {}
    }
    fetch(`${API}/api/v1/brand`)
      .then(r => r.json())
      .then(d => {
        if (d.name) setName(d.name);
        if (d.primary_color) setPrimaryColor(d.primary_color);
        if (d.logo_url) { setLogoUrl(d.logo_url); setLogoPreview(d.logo_url); }
      })
      .catch(() => {});
  }, []);

  // 品牌 CSS 变量注入
  const injectBrandVariables = useCallback((color: string) => {
    const root = document.documentElement;
    root.style.setProperty('--brand-primary', color);
    root.style.setProperty('--brand-gradient', `linear-gradient(135deg, ${color}, ${lightenColor(color, 0.3)})`);
    root.style.setProperty('--brand-glow', `${color}40`);
    root.style.setProperty('--brand-border', `${color}30`);
    root.style.setProperty('--brand-bg', `${color}12`);
    root.style.setProperty('--brand-text', color);
  }, []);

  // 处理 Logo 文件上传
  const handleLogoUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    if (file.size > 2 * 1024 * 1024) { toast.warning('Logo 文件不能超过 2MB'); return; }

    setUploading(true);
    try {
      // 转换为 base64 data URL
      const reader = new FileReader();
      const dataUrl = await new Promise<string>((resolve, reject) => {
        reader.onload = () => resolve(reader.result as string);
        reader.onerror = reject;
        reader.readAsDataURL(file);
      });
      setLogoPreview(dataUrl);
      setLogoUrl(dataUrl);
    } catch {
      toast.error('Logo 上传失败');
    } finally {
      setUploading(false);
    }
  };

  const handleRemoveLogo = () => {
    setLogoUrl('');
    setLogoPreview('');
    if (fileRef.current) fileRef.current.value = '';
  };

  // 保存品牌设置
  const handleSave = async () => {
    setSaving(true);
    try {
      // 注入 CSS 变量
      injectBrandVariables(primaryColor);

      // 保存到后端
      await fetch(`${API}/api/v1/brand`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          primary_color: primaryColor,
          name,
          logo_url: logoUrl,
        }),
      });

      // 缓存到 localStorage
      localStorage.setItem(STORAGE_KEY, JSON.stringify({
        name, primary_color: primaryColor, logo_url: logoUrl,
      }));

      setSaved(true);
      setTimeout(() => setSaved(false), 2500);
    } catch { toast.error('保存失败'); }
    finally { setSaving(false); }
  };

  return (
    <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} style={{ maxWidth: 640 }}>
      <h1 style={{ fontSize: 22, fontWeight: 700, color: '#fff', marginBottom: 8 }}>
        <Palette size={22} style={{ verticalAlign: -3, marginRight: 8, color: '#6c5ce7' }} />
        品牌定制
      </h1>
      <p style={{ color: '#8892b0', fontSize: 13, marginBottom: 28 }}>
        自定义 Logo、品牌色和名称，Chat UI 将自动适配企业品牌
      </p>

      {/* 组织名称 */}
      <div style={{
        background: 'rgba(255,255,255,0.03)', border: '1px solid rgba(255,255,255,0.06)',
        borderRadius: 16, padding: 24, marginBottom: 20,
      }}>
        <span style={{ fontSize: 14, fontWeight: 600, color: '#fff', display: 'block', marginBottom: 12 }}>组织名称</span>
        <input value={name} onChange={e => setName(e.target.value)} placeholder="VERIDACTUS"
          style={{
            width: '100%', padding: '10px 14px', borderRadius: 10, fontSize: 14, color: '#fff',
            background: 'rgba(255,255,255,0.05)', border: '1px solid rgba(255,255,255,0.1)', outline: 'none',
          }} />
      </div>

      {/* Logo 上传（真实功能） */}
      <div style={{
        background: 'rgba(255,255,255,0.03)', border: '1px solid rgba(255,255,255,0.06)',
        borderRadius: 16, padding: 24, marginBottom: 20,
      }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 16 }}>
          <Image size={18} color="#6c5ce7" />
          <span style={{ fontSize: 14, fontWeight: 600, color: '#fff' }}>企业 Logo</span>
        </div>

        <div style={{ display: 'flex', gap: 16, alignItems: 'flex-start' }}>
          {/* 上传区域 */}
          {logoPreview ? (
            <div style={{ position: 'relative' }}>
              <img src={logoPreview} alt="Logo" style={{
                width: 120, height: 120, borderRadius: 16, objectFit: 'contain',
                border: `2px solid ${primaryColor}40`, background: 'rgba(255,255,255,0.05)',
              }} />
              <button onClick={handleRemoveLogo} style={{
                position: 'absolute', top: -6, right: -6,
                width: 24, height: 24, borderRadius: '50%',
                background: 'rgba(255,107,107,0.9)', border: 'none',
                display: 'flex', alignItems: 'center', justifyContent: 'center',
                cursor: 'pointer',
              }}>
                <X size={12} color="#fff" />
              </button>
            </div>
          ) : (
            <div onClick={() => fileRef.current?.click()}
              style={{
                width: 120, height: 120, borderRadius: 16,
                background: 'rgba(108,92,231,0.1)', border: '2px dashed rgba(108,92,231,0.3)',
                display: 'flex', alignItems: 'center', justifyContent: 'center',
                cursor: 'pointer', transition: 'all 0.2s',
              }}>
              <div style={{ textAlign: 'center', color: '#8892b0' }}>
                {uploading ? <Loader2 size={24} style={{ animation: 'spin 1s linear infinite', marginBottom: 8 }} /> : <Upload size={24} style={{ marginBottom: 8 }} />}
                <span style={{ fontSize: 11 }}>{uploading ? '处理中...' : '上传 Logo'}</span>
              </div>
            </div>
          )}

          <input ref={fileRef} type="file" accept="image/*" onChange={handleLogoUpload} style={{ display: 'none' }} />

          <div style={{ fontSize: 11, color: '#5a6a8a', lineHeight: 1.8 }}>
            支持 PNG / JPG / SVG<br />
            建议尺寸: 120×120px<br />
            最大: 2MB
          </div>
        </div>
      </div>

      {/* 主题色 */}
      <div style={{
        background: 'rgba(255,255,255,0.03)', border: '1px solid rgba(255,255,255,0.06)',
        borderRadius: 16, padding: 24, marginBottom: 20,
      }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 16 }}>
          <Palette size={18} color="#6c5ce7" />
          <span style={{ fontSize: 14, fontWeight: 600, color: '#fff' }}>主题色</span>
        </div>
        <div style={{ display: 'flex', gap: 12, alignItems: 'center', flexWrap: 'wrap' }}>
          <input type="color" value={primaryColor} onChange={e => { setPrimaryColor(e.target.value); injectBrandVariables(e.target.value); }}
            style={{ width: 48, height: 48, borderRadius: 12, border: '2px solid rgba(255,255,255,0.1)', cursor: 'pointer', padding: 2, background: 'transparent' }} />
          <input value={primaryColor} onChange={e => setPrimaryColor(e.target.value)}
            style={{ background: 'rgba(255,255,255,0.05)', border: '1px solid rgba(255,255,255,0.1)', borderRadius: 10, padding: '10px 14px', color: '#fff', fontSize: 14, fontFamily: 'monospace', width: 140, outline: 'none' }} />
          <div style={{ width: 48, height: 48, borderRadius: 12, background: primaryColor, boxShadow: `0 0 20px ${primaryColor}40` }} />
        </div>
        {/* 预设色板 */}
        <div style={{ marginTop: 16, display: 'flex', gap: 8, flexWrap: 'wrap' }}>
          {['#6c5ce7', '#00d4aa', '#ff6b6b', '#74b9ff', '#fdcb6e', '#e17055', '#0984e3', '#00cec9'].map(c => (
            <div key={c} onClick={() => { setPrimaryColor(c); injectBrandVariables(c); }}
              style={{ width: 28, height: 28, borderRadius: 8, background: c, cursor: 'pointer', border: primaryColor === c ? '2px solid #fff' : '2px solid transparent', transition: 'all 0.15s' }} />
          ))}
        </div>
      </div>

      {/* 预览 */}
      <div style={{
        background: 'rgba(255,255,255,0.03)', border: `1px solid ${primaryColor}30`,
        borderRadius: 16, padding: 24, marginBottom: 24,
      }}>
        <div style={{ fontSize: 12, color: '#8892b0', marginBottom: 12 }}>实时预览 — 你的 Chat UI 将显示为：</div>
        <div style={{ padding: 16, borderRadius: 12, background: `${primaryColor}15`, border: `1px solid ${primaryColor}30` }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
            {logoPreview ? (
              <img src={logoPreview} alt="" style={{ width: 32, height: 32, borderRadius: 8, objectFit: 'contain' }} />
            ) : (
              <div style={{ width: 32, height: 32, borderRadius: 8, background: `linear-gradient(135deg, ${primaryColor}, ${lightenColor(primaryColor, 0.3)})` }} />
            )}
            <span style={{ color: '#fff', fontWeight: 700, fontSize: 15 }}>{name || 'VERIDACTUS'}</span>
            <span style={{ fontSize: 10, padding: '2px 8px', borderRadius: 10, background: `${primaryColor}20`, color: primaryColor, fontWeight: 600 }}>ENTERPRISE</span>
          </div>
          <div style={{ marginTop: 14, display: 'flex', gap: 8 }}>
            <div style={{ padding: '8px 18px', borderRadius: 8, fontSize: 12, fontWeight: 600, background: primaryColor, color: '#fff' }}>发送消息</div>
            <div style={{ padding: '8px 18px', borderRadius: 8, fontSize: 12, fontWeight: 600, background: `${primaryColor}20`, border: `1px solid ${primaryColor}40`, color: primaryColor }}>设置</div>
          </div>
          <div style={{ marginTop: 12, display: 'flex', gap: 6, flexWrap: 'wrap' }}>
            {['--brand-primary', '--brand-gradient', '--brand-glow', '--brand-border', '--brand-bg', '--brand-text'].map(v => (
              <code key={v} style={{ fontSize: 10, padding: '2px 8px', borderRadius: 6, background: 'rgba(255,255,255,0.05)', color: '#8892b0' }}>{v}</code>
            ))}
          </div>
        </div>
      </div>

      {/* 保存 */}
      <div style={{ display: 'flex', gap: 12, alignItems: 'center' }}>
        <motion.button whileHover={{ scale: 1.02 }} whileTap={{ scale: 0.98 }}
          onClick={handleSave} disabled={saving}
          style={{ padding: '12px 28px', borderRadius: 12, background: 'linear-gradient(135deg, #6c5ce7, #00d4aa)', border: 'none', color: '#fff', fontSize: 14, fontWeight: 700, cursor: saving ? 'not-allowed' : 'pointer', opacity: saving ? 0.7 : 1 }}>
          {saving ? '保存中...' : '保存品牌设置'}
        </motion.button>
        {saved && (
          <motion.span initial={{ opacity: 0, x: -8 }} animate={{ opacity: 1, x: 0 }}
            style={{ display: 'flex', alignItems: 'center', gap: 6, color: '#00d4aa', fontSize: 13 }}>
            <CheckCircle size={16} /> 品牌设置已保存，CSS 变量已注入
          </motion.span>
        )}
      </div>
    </motion.div>
  );
}

// 颜色变亮辅助函数
function lightenColor(hex: string, amount: number): string {
  const num = parseInt(hex.replace('#', ''), 16);
  const r = Math.min(255, (num >> 16) + Math.round(255 * amount));
  const g = Math.min(255, ((num >> 8) & 0x00FF) + Math.round(255 * amount));
  const b = Math.min(255, (num & 0x0000FF) + Math.round(255 * amount));
  return '#' + ((r << 16) | (g << 8) | b).toString(16).padStart(6, '0');
}
