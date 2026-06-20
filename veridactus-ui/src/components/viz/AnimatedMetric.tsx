import { motion, useAnimation } from 'framer-motion';
import { useEffect, useRef } from 'react';

interface AnimatedMetricProps {
  value: number;
  prefix?: string;
  suffix?: string;
  decimals?: number;
  threshold?: { value: number; color: string };
}

export default function AnimatedMetric({ value, prefix = '', suffix = '', decimals = 2, threshold }: AnimatedMetricProps) {
  const controls = useAnimation();
  const prev = useRef(value);

  useEffect(() => {
    if (value !== prev.current) {
      controls.start({
        scale: [1, 1.05, 1],
        color: threshold && value > threshold.value ? [undefined, threshold.color, undefined] : undefined,
        transition: { duration: 0.3 },
      });
      prev.current = value;
    }
  }, [value, controls, threshold]);

  const formatted = new Intl.NumberFormat('en-US', { minimumFractionDigits: decimals, maximumFractionDigits: decimals }).format(value);

  return (
    <motion.span
      animate={controls}
      style={{ fontFamily: "'JetBrains Mono', monospace", color: '#00d4aa', display: 'inline-flex', alignItems: 'baseline', gap: 2, fontVariantNumeric: 'tabular-nums' }}
    >
      {prefix && <span style={{ opacity: 0.7, fontSize: '0.9em' }}>{prefix}</span>}
      <span style={{ fontWeight: 600 }}>{formatted}</span>
      {suffix && <span style={{ opacity: 0.7, fontSize: '0.9em' }}>{suffix}</span>}
    </motion.span>
  );
}
