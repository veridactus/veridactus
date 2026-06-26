// 品牌白标定制 — Logo上传 + 完整 CSS Variables 品牌注入
import { useState, useEffect, useRef, useCallback } from 'react';
import { motion } from 'framer-motion';
import { Upload, CheckCircle, Palette, Image, Loader2, X } from 'lucide-react';
import { toast } from '../components/ui/Toast';

const STORAGE_KEY = 'veridactus_brand';
const API = (import.meta as any)?.env?.VITE_API_URL || '';

const PRESET_COLORS = ['#6c5ce7', '#00d4aa', '#ff6b6b', '#74b9ff', '#fdcb6e', '#e17055', '#0984e3', '#00cec9'];

export default function BrandSettings() {
  const [name, setName] = useState('');
  const [primaryColor, setPrimaryColor] = useState('#6c5ce7');
  const [logoUrl, setLogoUrl] = useState('');
  const [logoPreview, setLogoPreview] = useState('');
  const [saved, setSaved] = useState(false);
  const [saving, setSaving] = useState(false);
  const [uploading, setUploading] = useState(false);
  const fileRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const cached = localStorage.getItem(STORAGE_KEY);
    if (cached) try {
      const b = JSON.parse(cached);
      if (b.name) setName(b.name);
      if (b.primary_color) setPrimaryColor(b.primary_color);
      if (b.logo_url) { setLogoUrl(b.logo_url); setLogoPreview(b.logo_url); }
    } catch {}
    fetch(`${API}/api/v1/brand`).then(r => r.json()).then(d => {
      if (d.name) setName(d.name);
      if (d.primary_color) setPrimaryColor(d.primary_color);
      if (d.logo_url) { setLogoUrl(d.logo_url); setLogoPreview(d.logo_url); }
    }).catch(() => {});
  }, []);

  const injectBrandVariables = useCallback((color: string) => {
    const root = document.documentElement;
    root.style.setProperty('--brand-primary', color);
    root.style.setProperty('--brand-gradient', `linear-gradient(135deg, ${color}, ${lightenColor(color, 0.3)})`);
    root.style.setProperty('--brand-glow', `${color}40`);
    root.style.setProperty('--brand-border', `${color}30`);
    root.style.setProperty('--brand-bg', `${color}12`);
    root.style.setProperty('--brand-text', color);
  }, []);

  const handleLogoUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    if (file.size > 2 * 1024 * 1024) { toast.warning('Logo 文件不能超过 2MB'); return; }
    setUploading(true);
    try {
      const reader = new FileReader();
      const dataUrl = await new Promise<string>((resolve, reject) => {
        reader.onload = () => resolve(reader.result as string);
        reader.onerror = reject;
        reader.readAsDataURL(file);
      });
      setLogoPreview(dataUrl); setLogoUrl(dataUrl);
    } catch { toast.error('Logo 上传失败'); }
    finally { setUploading(false); }
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      injectBrandVariables(primaryColor);
      await fetch(`${API}/api/v1/brand`, {
        method: 'PUT', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ primary_color: primaryColor, name, logo_url: logoUrl }),
      });
      localStorage.setItem(STORAGE_KEY, JSON.stringify({ name, primary_color: primaryColor, logo_url: logoUrl }));
      setSaved(true); setTimeout(() => setSaved(false), 2500);
    } catch { toast.error('保存失败'); }
    finally { setSaving(false); }
  };

  return (
    <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="max-w-[640px]">
      <h1 className="text-xl font-bold text-white mb-2 flex items-center gap-2">
        <Palette size={22} className="text-[#6c5ce7]" /> 品牌定制
      </h1>
      <p className="text-sm text-[#8892b0] mb-7">自定义 Logo、品牌色和名称，Chat UI 将自动适配企业品牌</p>

      {/* 组织名称 */}
      <div className="p-6 rounded-card mb-5" style={{ background: 'rgba(255,255,255,0.03)', border: '1px solid rgba(255,255,255,0.06)' }}>
        <span className="block text-sm font-semibold text-white mb-3">组织名称</span>
        <input value={name} onChange={e => setName(e.target.value)} placeholder="VERIDACTUS"
          className="w-full py-2.5 px-3.5 rounded-btn text-sm text-white border outline-none"
          style={{ background: 'rgba(255,255,255,0.05)', borderColor: 'rgba(255,255,255,0.1)' }} />
      </div>

      {/* Logo 上传 */}
      <div className="p-6 rounded-card mb-5" style={{ background: 'rgba(255,255,255,0.03)', border: '1px solid rgba(255,255,255,0.06)' }}>
        <div className="flex items-center gap-2 mb-4"><Image size={18} color="#6c5ce7" /><span className="text-sm font-semibold text-white">企业 Logo</span></div>
        <div className="flex gap-4 items-start">
          {logoPreview ? (
            <div className="relative">
              <img src={logoPreview} alt="Logo" className="w-[120px] h-[120px] rounded-card object-contain" style={{ border: `2px solid ${primaryColor}40`, background: 'rgba(255,255,255,0.05)' }} />
              <button onClick={() => { setLogoUrl(''); setLogoPreview(''); if (fileRef.current) fileRef.current.value = ''; }}
                className="absolute -top-1.5 -right-1.5 w-6 h-6 rounded-full flex items-center justify-center cursor-pointer" style={{ background: 'rgba(255,107,107,0.9)' }}>
                <X size={12} color="#fff" />
              </button>
            </div>
          ) : (
            <div onClick={() => fileRef.current?.click()}
              className="w-[120px] h-[120px] rounded-card flex items-center justify-center cursor-pointer transition-all border-2 border-dashed" style={{ background: 'rgba(108,92,231,0.1)', borderColor: 'rgba(108,92,231,0.3)' }}>
              <div className="text-center">
                {uploading ? <Loader2 size={24} className="animate-spin mb-2 mx-auto text-[#8892b0]" /> : <Upload size={24} className="mb-2 mx-auto text-[#8892b0]" />}
                <span className="text-[11px] text-[#8892b0]">{uploading ? '处理中...' : '上传 Logo'}</span>
              </div>
            </div>
          )}
          <input ref={fileRef} type="file" accept="image/*" onChange={handleLogoUpload} className="hidden" />
          <div className="text-[11px] text-[#5a6a8a] leading-relaxed">支持 PNG / JPG / SVG<br />建议尺寸: 120×120px<br />最大: 2MB</div>
        </div>
      </div>

      {/* 主题色 */}
      <div className="p-6 rounded-card mb-5" style={{ background: 'rgba(255,255,255,0.03)', border: '1px solid rgba(255,255,255,0.06)' }}>
        <div className="flex items-center gap-2 mb-4"><Palette size={18} color="#6c5ce7" /><span className="text-sm font-semibold text-white">主题色</span></div>
        <div className="flex gap-3 items-center flex-wrap">
          <input type="color" value={primaryColor} onChange={e => { setPrimaryColor(e.target.value); injectBrandVariables(e.target.value); }}
            className="w-12 h-12 rounded-xl cursor-pointer p-0.5" style={{ border: '2px solid rgba(255,255,255,0.1)', background: 'transparent' }} />
          <input value={primaryColor} onChange={e => setPrimaryColor(e.target.value)}
            className="py-2.5 px-3.5 rounded-btn text-sm text-white font-mono w-[140px] outline-none" style={{ background: 'rgba(255,255,255,0.05)', border: '1px solid rgba(255,255,255,0.1)' }} />
          <div className="w-12 h-12 rounded-xl" style={{ background: primaryColor, boxShadow: `0 0 20px ${primaryColor}40` }} />
        </div>
        <div className="mt-4 flex gap-2 flex-wrap">
          {PRESET_COLORS.map(c => (
            <div key={c} onClick={() => { setPrimaryColor(c); injectBrandVariables(c); }}
              className="w-7 h-7 rounded-lg cursor-pointer transition-all" style={{ background: c, border: primaryColor === c ? '2px solid #fff' : '2px solid transparent' }} />
          ))}
        </div>
      </div>

      {/* 预览 */}
      <div className="p-6 rounded-card mb-6" style={{ background: 'rgba(255,255,255,0.03)', border: `1px solid ${primaryColor}30` }}>
        <div className="text-xs text-[#8892b0] mb-3">实时预览 — 你的 Chat UI 将显示为：</div>
        <div className="p-4 rounded-xl" style={{ background: `${primaryColor}15`, border: `1px solid ${primaryColor}30` }}>
          <div className="flex items-center gap-2.5">
            {logoPreview ? <img src={logoPreview} alt="" className="w-8 h-8 rounded-lg object-contain" />
              : <div className="w-8 h-8 rounded-lg" style={{ background: `linear-gradient(135deg, ${primaryColor}, ${lightenColor(primaryColor, 0.3)})` }} />}
            <span className="text-white font-bold text-[15px]">{name || 'VERIDACTUS'}</span>
            <span className="text-[10px] py-0.5 px-2 rounded-btn font-semibold" style={{ background: `${primaryColor}20`, color: primaryColor }}>ENTERPRISE</span>
          </div>
          <div className="mt-3.5 flex gap-2">
            <div className="py-2 px-[18px] rounded-lg text-xs font-semibold text-white" style={{ background: primaryColor }}>发送消息</div>
            <div className="py-2 px-[18px] rounded-lg text-xs font-semibold" style={{ background: `${primaryColor}20`, border: `1px solid ${primaryColor}40`, color: primaryColor }}>设置</div>
          </div>
          <div className="mt-3 flex gap-1.5 flex-wrap">
            {['--brand-primary', '--brand-gradient', '--brand-glow', '--brand-border', '--brand-bg', '--brand-text'].map(v => (
              <code key={v} className="text-[10px] py-0.5 px-2 rounded-md text-[#8892b0]" style={{ background: 'rgba(255,255,255,0.05)' }}>{v}</code>
            ))}
          </div>
        </div>
      </div>

      {/* 保存 */}
      <div className="flex gap-3 items-center">
        <motion.button whileHover={{ scale: 1.02 }} whileTap={{ scale: 0.98 }} onClick={handleSave} disabled={saving}
          className="py-3 px-7 rounded-xl text-sm font-bold text-white cursor-pointer disabled:opacity-70 disabled:cursor-not-allowed"
          style={{ background: 'linear-gradient(135deg, #6c5ce7, #00d4aa)' }}>
          {saving ? '保存中...' : '保存品牌设置'}
        </motion.button>
        {saved && (
          <motion.span initial={{ opacity: 0, x: -8 }} animate={{ opacity: 1, x: 0 }} className="flex items-center gap-1.5 text-[#00d4aa] text-sm">
            <CheckCircle size={16} /> 品牌设置已保存，CSS 变量已注入
          </motion.span>
        )}
      </div>
    </motion.div>
  );
}

function lightenColor(hex: string, amount: number): string {
  const num = parseInt(hex.replace('#', ''), 16);
  const r = Math.min(255, (num >> 16) + Math.round(255 * amount));
  const g = Math.min(255, ((num >> 8) & 0x00FF) + Math.round(255 * amount));
  const b = Math.min(255, (num & 0x0000FF) + Math.round(255 * amount));
  return '#' + ((r << 16) | (g << 8) | b).toString(16).padStart(6, '0');
}
