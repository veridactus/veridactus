interface PluginIconProps {
  category: string;
  size?: number;
}

const icons: Record<string, string> = {
  budget: '💰', auth: '🔑', route: '🚦', guardrail: '🛡️', pii: '🔒',
  proof: '🔐', drift: '📊', guarantee: '✅', wasm: '⚡', grpc: '🔌',
};

export default function PluginIcon({ category, size = 16 }: PluginIconProps) {
  return <span style={{ fontSize: size }}>{icons[category] || '🧩'}</span>;
}
