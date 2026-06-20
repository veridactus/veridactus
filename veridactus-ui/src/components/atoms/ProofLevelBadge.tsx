interface ProofLevelBadgeProps {
  level: string;
  size?: 'small' | 'medium' | 'large';
}

const levelColors: Record<string, { cls: string; label: string }> = {
  L0: { cls: 'proof-l0', label: 'L0 HashChain' },
  L1: { cls: 'proof-l1', label: 'L1 TEE' },
  L2A: { cls: 'proof-l2a', label: 'L2A Transcript' },
  L2B: { cls: 'proof-l2b', label: 'L2B ZK' },
};

export default function ProofLevelBadge({ level, size = 'medium' }: ProofLevelBadgeProps) {
  const info = levelColors[level] || levelColors.L0;
  const fontSize = size === 'small' ? 10 : size === 'large' ? 13 : 11;
  return (
    <span className={`badge ${info.cls}`} style={{ fontSize, padding: size === 'small' ? '2px 8px' : '3px 12px' }}>
      {info.label}
    </span>
  );
}
