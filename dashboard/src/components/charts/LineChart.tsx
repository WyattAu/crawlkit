import {
  LineChart as RechartsLineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from 'recharts';

interface DataPoint {
  name: string;
  value: number;
}

interface LineChartProps {
  data: DataPoint[];
  title?: string;
  color?: string;
  xLabel?: string;
  yLabel?: string;
}

export default function LineChart({
  data,
  title,
  color = '#3b82f6',
  xLabel,
  yLabel,
}: LineChartProps) {
  return (
    <div role="img" aria-label={`Line chart showing ${title || 'data'}`} className="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm border border-gray-200 dark:border-gray-700">
      {title && (
        <h3 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-4">
          {title}
        </h3>
      )}
      <ResponsiveContainer width="100%" height={300}>
        <RechartsLineChart data={data}>
          <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
          <XAxis
            dataKey="name"
            tick={{ fontSize: 12 }}
            label={xLabel ? { value: xLabel, position: 'bottom', offset: 0 } : undefined}
          />
          <YAxis
            tick={{ fontSize: 12 }}
            label={
              yLabel
                ? { value: yLabel, angle: -90, position: 'insideLeft' }
                : undefined
            }
          />
          <Tooltip />
          <Line
            type="monotone"
            dataKey="value"
            stroke={color}
            strokeWidth={2}
            dot={{ fill: color, strokeWidth: 2 }}
          />
        </RechartsLineChart>
      </ResponsiveContainer>
    </div>
  );
}
