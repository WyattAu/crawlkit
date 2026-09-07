import { useEffect, useState } from 'react';

interface HealthScoreGaugeProps {
  score: number;
  size?: number;
  strokeWidth?: number;
}

function scoreColor(score: number): string {
  if (score >= 80) return '#22c55e';
  if (score >= 50) return '#eab308';
  return '#ef4444';
}

function scoreLabel(score: number): string {
  if (score >= 80) return 'Good';
  if (score >= 50) return 'Needs attention';
  return 'Poor';
}

export default function HealthScoreGauge({
  score,
  size = 200,
  strokeWidth = 16,
}: HealthScoreGaugeProps) {
  const [animated, setAnimated] = useState(0);

  useEffect(() => {
    const timer = window.setTimeout(() => setAnimated(score), 50);
    return () => window.clearTimeout(timer);
  }, [score]);

  const clamped = Math.max(0, Math.min(100, score));
  const radius = (size - strokeWidth) / 2;
  const circumference = 2 * Math.PI * radius;
  const arcSpan = 0.75 * circumference;
  const offset = circumference - (animated / 100) * arcSpan;
  const center = size / 2;
  const color = scoreColor(clamped);

  return (
    <div
      className="flex flex-col items-center"
      role="img"
      aria-label={`Health score ${clamped} out of 100`}
    >
      <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`}>
        <circle
          cx={center}
          cy={center}
          r={radius}
          fill="none"
          strokeWidth={strokeWidth}
          strokeDasharray={`${arcSpan} ${circumference}`}
          strokeLinecap="round"
          className="stroke-gray-200 dark:stroke-gray-700"
          transform={`rotate(135 ${center} ${center})`}
        />
        <circle
          cx={center}
          cy={center}
          r={radius}
          fill="none"
          stroke={color}
          strokeWidth={strokeWidth}
          strokeDasharray={`${circumference} ${circumference}`}
          strokeDashoffset={offset}
          strokeLinecap="round"
          transform={`rotate(135 ${center} ${center})`}
          style={{ transition: 'stroke-dashoffset 1s ease-in-out, stroke 0.5s ease-in-out' }}
        />
        <text
          x={center}
          y={center}
          textAnchor="middle"
          dominantBaseline="central"
          className="fill-gray-900 dark:fill-white"
          style={{ fontSize: size * 0.22, fontWeight: 700 }}
        >
          {clamped}
        </text>
        <text
          x={center}
          y={center + size * 0.16}
          textAnchor="middle"
          dominantBaseline="central"
          className="fill-gray-500 dark:fill-gray-400"
          style={{ fontSize: size * 0.07 }}
        >
          / 100
        </text>
      </svg>
      <p className="mt-2 text-sm font-medium" style={{ color }}>
        {scoreLabel(clamped)}
      </p>
    </div>
  );
}
