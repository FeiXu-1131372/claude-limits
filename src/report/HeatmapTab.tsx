import { useMemo, useState } from 'react';
import { EmptyState } from '../components/ui/EmptyState';
import type { HeatmapCell } from '../lib/types';
import { formatTokens } from '../lib/format';
import { IconHeatmap } from '../lib/icons';

/* Generate placeholder — ~180 days (6 months) */
function generatePlaceholder(): HeatmapCell[] {
  const cells: HeatmapCell[] = [];
  for (let i = 180; i >= 0; i--) {
    const d = new Date();
    d.setDate(d.getDate() - i);
    const val = Math.random();
    const level = val < 0.25 ? 0 : val < 0.45 ? 1 : val < 0.65 ? 2 : val < 0.85 ? 3 : 4;
    cells.push({
      date: d.toISOString().slice(0, 10),
      value: Math.floor(val * 500_000),
      level: level as 0 | 1 | 2 | 3 | 4,
    });
  }
  return cells;
}

const PLACEHOLDER = generatePlaceholder();

const CELL_SIZE = 11;
const CELL_GAP = 3;
const CELL_STEP = CELL_SIZE + CELL_GAP;

const MONTH_LABELS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
const DAY_LABELS = ['Mon', '', 'Wed', '', 'Fri', '', ''];

function getMonthPositions(cells: HeatmapCell[]): { label: string; x: number }[] {
  const seen = new Set<string>();
  const months: { label: string; x: number }[] = [];
  const startDay = new Date(cells[0]?.date ?? '').getDay();

  for (const cell of cells) {
    const d = new Date(cell.date);
    const key = `${d.getFullYear()}-${d.getMonth()}`;
    if (!seen.has(key)) {
      seen.add(key);
      const idx = cells.indexOf(cell);
      const col = Math.floor((idx + startDay) / 7);
      months.push({ label: MONTH_LABELS[d.getMonth()], x: col * CELL_STEP });
    }
  }
  return months;
}

const levelColors: Record<number, string> = {
  0: 'var(--color-track)',
  1: 'var(--color-safe)',
  2: 'var(--color-accent)',
  3: 'var(--color-warn)',
  4: 'var(--color-danger)',
};

export function HeatmapTab() {
  const data = PLACEHOLDER;
  const [hovered, setHovered] = useState<string | null>(null);

  const startDay = useMemo(() => {
    const d = new Date(data[0]?.date ?? '');
    return (d.getDay() + 6) % 7; // Monday = 0
  }, [data]);

  const totalDays = data.length;
  const weeks = Math.ceil((totalDays + startDay) / 7);
  const svgWidth = weeks * CELL_STEP + 30; // 30px for day labels
  const svgHeight = 7 * CELL_STEP + 20; // 20px for month labels

  const monthPositions = useMemo(() => getMonthPositions(data), [data]);

  if (data.length === 0) {
    return (
      <EmptyState
        icon={<IconHeatmap size={32} />}
        title="No heatmap data"
        description="Usage activity will appear here over time."
      />
    );
  }

  const totalValue = data.reduce((s, c) => s + c.value, 0);

  return (
    <div className="flex flex-col gap-[var(--space-md)]">
      {/* Legend */}
      <div className="flex items-center justify-between px-[var(--space-2xs)]">
        <span className="text-[var(--text-label)] text-[var(--color-text-muted)]">
          Last 6 months
        </span>
        <div className="flex items-center gap-[var(--space-2xs)]">
          <span className="text-[var(--text-micro)] text-[var(--color-text-muted)]">Less</span>
          {[0, 1, 2, 3, 4].map((level) => (
            <div
              key={level}
              className="w-[10px] h-[10px] rounded-[2px]"
              style={{ background: levelColors[level] }}
            />
          ))}
          <span className="text-[var(--text-micro)] text-[var(--color-text-muted)]">More</span>
        </div>
      </div>

      {/* Heatmap grid */}
      <div className="overflow-x-auto">
        <svg width={svgWidth} height={svgHeight}>
          {/* Month labels */}
          {monthPositions.map((m, i) => (
            <text
              key={i}
              x={30 + m.x}
              y={10}
              className="mono"
              style={{ fontSize: 9, fill: 'var(--color-text-muted)' }}
            >
              {m.label}
            </text>
          ))}

          {/* Day labels */}
          {dayLabels}

          {/* Cells */}
          {data.map((cell, i) => {
            const col = Math.floor((i + startDay) / 7);
            const row = (i + startDay) % 7;
            const x = 30 + col * CELL_STEP;
            const y = 18 + row * CELL_STEP;

            return (
              <g key={cell.date}>
                <rect
                  x={x}
                  y={y}
                  width={CELL_SIZE}
                  height={CELL_SIZE}
                  rx={2}
                  fill={levelColors[cell.level]}
                  opacity={hovered === cell.date ? 1 : 0.75}
                  onMouseEnter={() => setHovered(cell.date)}
                  onMouseLeave={() => setHovered(null)}
                  className="cursor-pointer transition-opacity"
                />
                {hovered === cell.date && (
                  <g>
                    <rect
                      x={x - 2}
                      y={y - 2}
                      width={CELL_SIZE + 4}
                      height={CELL_SIZE + 4}
                      rx={3}
                      fill="none"
                      stroke="var(--color-text)"
                      strokeWidth={1.5}
                    />
                    <text
                      x={x + CELL_SIZE / 2}
                      y={y - 6}
                      textAnchor="middle"
                      className="mono"
                      style={{ fontSize: 9, fill: 'var(--color-text-secondary)' }}
                    >
                      {new Date(cell.date).toLocaleDateString('en-US', { month: 'short', day: 'numeric' })}
                    </text>
                  </g>
                )}
              </g>
            );
          })}
        </svg>
      </div>

      {/* Summary */}
      <div className="flex items-center gap-[var(--space-md)] px-[var(--space-2xs)]">
        <span className="mono text-[var(--text-label)] text-[var(--color-text-secondary)]">
          {data.filter((c) => c.level > 0).length} active days
        </span>
        <span className="mono text-[var(--text-label)] text-[var(--color-text-muted)]">
          {formatTokens(totalValue)} total
        </span>
      </div>
    </div>
  );
}

/* Day label elements for the SVG */
const dayLabels = dayLabelRows();

function dayLabelRows() {
  return DAY_LABELS.map((label, i) =>
    label ? (
      <text
        key={i}
        x={24}
        y={18 + i * CELL_STEP + CELL_SIZE / 2 + 3}
        textAnchor="end"
        className="mono"
        style={{ fontSize: 9, fill: 'var(--color-text-muted)' }}
      >
        {label}
      </text>
    ) : null,
  );
}
