import { create } from 'zustand';

interface MetricsState {
  traceCount: number;
  pipelineCount: number;
  pluginCount: number;
  policyCount: number;
  services: { dataPlane: boolean; controlPlane: boolean; pythonWorker: boolean };
  setMetrics: (m: Partial<MetricsState>) => void;
}

export const useMetricsStore = create<MetricsState>((set) => ({
  traceCount: 0, pipelineCount: 0, pluginCount: 0, policyCount: 0,
  services: { dataPlane: false, controlPlane: false, pythonWorker: false },
  setMetrics: (m) => set(m),
}));

export type ThemeMode = 'dark' | 'light';

interface ThemeState {
  theme: ThemeMode;
  setTheme: (t: ThemeMode) => void;
  toggleTheme: () => void;
}

function getInitialTheme(): ThemeMode {
  try {
    const saved = localStorage.getItem('veridactus-theme');
    if (saved === 'light' || saved === 'dark') return saved;
  } catch {}
  return 'dark';
}

export const useThemeStore = create<ThemeState>((set) => ({
  theme: getInitialTheme(),
  setTheme: (t) => {
    document.documentElement.setAttribute('data-theme', t);
    try { localStorage.setItem('veridactus-theme', t); } catch {}
    set({ theme: t });
  },
  toggleTheme: () => set((state) => {
    const next = state.theme === 'dark' ? 'light' : 'dark';
    document.documentElement.setAttribute('data-theme', next);
    try { localStorage.setItem('veridactus-theme', next); } catch {}
    return { theme: next };
  }),
}));
