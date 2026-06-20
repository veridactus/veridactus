/**
 * VERIDACTUS 桌面端色彩系统：深空主题 + 动态高光
 * AI-front.md §2.1
 */
export const colors = {
  background: {
    primary: '#0a0e27',
    secondary: '#131633',
    tertiary: '#1a1e42',
    overlay: 'rgba(10, 14, 39, 0.85)',
    glass: 'rgba(255, 255, 255, 0.08)',
    glassBorder: 'rgba(255, 255, 255, 0.12)',
  },
  brand: {
    primary: { start: '#6c5ce7', end: '#00d4aa', glow: 'rgba(108, 92, 231, 0.4)' },
    secondary: { start: '#00cec9', end: '#74b9ff' },
    gradient: 'linear-gradient(135deg, #6c5ce7 0%, #00d4aa 100%)',
    gradientHover: 'linear-gradient(135deg, #5a4fcf 0%, #00b894 100%)',
  },
  semantic: {
    success: { bg: 'rgba(0, 212, 170, 0.12)', border: 'rgba(0, 212, 170, 0.4)', text: '#00d4aa', glow: '0 0 20px rgba(0, 212, 170, 0.3)' },
    warning: { bg: 'rgba(253, 203, 110, 0.12)', border: 'rgba(253, 203, 110, 0.4)', text: '#fdcb6e', glow: '0 0 20px rgba(253, 203, 110, 0.3)' },
    error: { bg: 'rgba(255, 118, 117, 0.12)', border: 'rgba(255, 118, 117, 0.4)', text: '#ff7675', glow: '0 0 20px rgba(255, 118, 117, 0.3)' },
    proof: {
      l0: { text: '#74b9ff', bg: 'rgba(116, 185, 255, 0.1)', glow: '0 0 12px rgba(116, 185, 255, 0.4)' },
      l1: { text: '#fdcb6e', bg: 'rgba(253, 203, 110, 0.1)', glow: '0 0 12px rgba(253, 203, 110, 0.4)' },
      l2a: { text: '#a29bfe', bg: 'rgba(162, 155, 254, 0.1)', glow: '0 0 12px rgba(162, 155, 254, 0.4)' },
      l2b: { text: '#00d4aa', bg: 'rgba(0, 212, 170, 0.1)', glow: '0 0 16px rgba(0, 212, 170, 0.6)' },
    },
  },
  text: {
    primary: 'rgba(255, 255, 255, 0.92)',
    secondary: 'rgba(255, 255, 255, 0.65)',
    tertiary: 'rgba(255, 255, 255, 0.4)',
    disabled: 'rgba(255, 255, 255, 0.25)',
    metric: { font: "'JetBrains Mono', monospace", color: '#00d4aa', highlight: 'rgba(0, 212, 170, 0.2)' },
  },
  border: {
    default: 'rgba(255, 255, 255, 0.12)',
    hover: 'rgba(255, 255, 255, 0.24)',
    focus: 'rgba(108, 92, 231, 0.6)',
    glass: 'inset 0 0 0 1px rgba(255, 255, 255, 0.1), 0 0 0 1px rgba(255, 255, 255, 0.05)',
  },
  shadow: {
    card: '0 4px 24px rgba(0, 0, 0, 0.4), 0 0 0 1px rgba(255, 255, 255, 0.05)',
    cardHover: '0 8px 40px rgba(0, 0, 0, 0.5), 0 0 0 1px rgba(108, 92, 231, 0.3)',
    glow: { small: '0 0 12px rgba(108, 92, 231, 0.4)', medium: '0 0 24px rgba(108, 92, 231, 0.6)', large: '0 0 40px rgba(0, 212, 170, 0.8)' },
  },
} as const;
