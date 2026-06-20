import { useEffect, useRef } from 'react';

interface CircularProgressProps {
  score: number;
  color: string;
}

export default function CircularProgress({ score, color }: CircularProgressProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    const size = 180;
    canvas.width = size * dpr;
    canvas.height = size * dpr;
    canvas.style.width = size + 'px';
    canvas.style.height = size + 'px';
    ctx.scale(dpr, dpr);

    const cx = size / 2, cy = size / 2, r = 72, lineWidth = 8;
    const startAngle = -Math.PI / 2;
    const endAngle = startAngle + (Math.PI * 2 * score) / 100;

    ctx.clearRect(0, 0, size, size);

    // Trail
    ctx.beginPath();
    ctx.arc(cx, cy, r, 0, Math.PI * 2);
    ctx.strokeStyle = 'rgba(255,255,255,0.06)';
    ctx.lineWidth = lineWidth;
    ctx.stroke();

    // Active
    ctx.beginPath();
    ctx.arc(cx, cy, r, startAngle, endAngle);
    ctx.strokeStyle = color;
    ctx.lineWidth = lineWidth;
    ctx.lineCap = 'round';
    ctx.stroke();

    // Glow effect
    ctx.shadowColor = color;
    ctx.shadowBlur = 20;
    ctx.beginPath();
    ctx.arc(cx, cy, r, startAngle, endAngle);
    ctx.strokeStyle = color + '40';
    ctx.lineWidth = lineWidth + 4;
    ctx.stroke();
    ctx.shadowBlur = 0;
  }, [score, color]);

  return <canvas ref={canvasRef} style={{ display: 'block' }} />;
}
