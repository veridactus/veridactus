/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  darkMode: ['selector', '[data-theme="dark"]'],
  theme: {
    extend: {
      colors: {
        veridactus: {
          primary: '#0a0e27',
          secondary: '#131633',
          tertiary: '#1a1e42',
          glass: 'rgba(255,255,255,0.06)',
          accent: '#6c5ce7',
          success: '#00d4aa',
          warning: '#fdcb6e',
          error: '#ff7675',
        },
        text: {
          primary: 'rgba(255,255,255,0.92)',
          secondary: 'rgba(255,255,255,0.70)',
          tertiary: 'rgba(255,255,255,0.45)',
        },
        border: {
          DEFAULT: 'rgba(255,255,255,0.12)',
          hover: 'rgba(255,255,255,0.24)',
        },
      },
      fontSize: {
        'xs':  ['11px', { lineHeight: '1.4' }],
        'sm':  ['13px', { lineHeight: '1.5' }],
        'base':['14px', { lineHeight: '1.6' }],
        'lg':  ['16px', { lineHeight: '1.5' }],
        'xl':  ['20px', { lineHeight: '1.3' }],
        '2xl': ['24px', { lineHeight: '1.2' }],
        '3xl': ['28px', { lineHeight: '1.1' }],
      },
      spacing: {
        '1': '4px',  '2': '8px',  '3': '12px', '4': '16px',
        '5': '20px', '6': '24px', '7': '28px', '8': '32px',
        '10':'40px', '12':'48px', '15':'60px',
        'sidebar': '260px',
      },
      borderRadius: {
        'card': '16px',
        'btn': '10px',
        'input': '10px',
        'badge': '20px',
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', 'sans-serif'],
        mono: ['JetBrains Mono', 'Fira Code', 'monospace'],
      },
      keyframes: {
        'fade-in': { '0%': { opacity: '0', transform: 'translateY(8px)' }, '100%': { opacity: '1', transform: 'translateY(0)' } },
        'pulse-glow': { '0%,100%': { opacity: '0.6' }, '50%': { opacity: '1' } },
        'spin-slow': { '0%': { transform: 'rotate(0deg)' }, '100%': { transform: 'rotate(360deg)' } },
      },
      animation: {
        'fade-in': 'fade-in 0.3s ease-out',
        'pulse-glow': 'pulse-glow 2s ease-in-out infinite',
        'spin-slow': 'spin-slow 3s linear infinite',
      },
      boxShadow: {
        'card': '0 4px 24px rgba(0,0,0,0.4), 0 0 0 1px rgba(255,255,255,0.05)',
        'card-hover': '0 8px 40px rgba(0,0,0,0.5), 0 0 0 1px rgba(108,92,231,0.3)',
        'glow': '0 0 20px rgba(108,92,231,0.4)',
      },
    },
  },
  plugins: [],
}
