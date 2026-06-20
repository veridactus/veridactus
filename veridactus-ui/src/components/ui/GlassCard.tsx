import { motion } from 'framer-motion';

interface GlassCardProps {
  children: React.ReactNode;
  className?: string;
  glow?: 'none' | 'small' | 'medium' | 'large';
  border?: 'default' | 'glass' | 'gradient';
  hoverEnhance?: boolean;
  onClick?: () => void;
  style?: React.CSSProperties;
}

export default function GlassCard({
  children, className = '', glow = 'none', border = 'glass',
  hoverEnhance = true, onClick, style,
}: GlassCardProps) {
  const glowMap = { none: '', small: '0 0 12px rgba(108,92,231,0.4)', medium: '0 0 24px rgba(108,92,231,0.6)', large: '0 0 40px rgba(0,212,170,0.8)' };
  const borderStyle = border === 'gradient'
    ? { border: '2px solid transparent', backgroundImage: `linear-gradient(var(--bg-secondary),var(--bg-secondary)) padding-box, var(--brand-gradient) border-box` }
    : {};
  return (
    <motion.div
      className={`glass-card ${className}`}
      style={{ boxShadow: glowMap[glow], cursor: onClick ? 'pointer' : undefined, ...borderStyle, ...style }}
      whileHover={hoverEnhance ? { y: -2, boxShadow: 'var(--shadow-card-hover)' } : undefined}
      onClick={onClick}
    >
      {children}
    </motion.div>
  );
}
