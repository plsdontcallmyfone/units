'use client';
import { useMemo, useState } from 'react';

/** One series at a time, one axis: price or chest balance as a line, raid volume as bars. A hover
 * crosshair names the point. The figures come from the market read; nothing here is computed. */
export type MarketSeries = { ts: number; price: number; chest: number; raids: number }[];
type Tab = 'price' | 'chest' | 'raids';
const TABS: [Tab, string][] = [['price', 'Price'], ['chest', 'Chest'], ['raids', 'Raids']];
const W = 720, H = 300, PAD = { l: 48, r: 12, t: 12, b: 24 };

function fmt(tab: Tab, v: number): string {
  if (tab === 'price') return `${v.toPrecision(4)} SOL`;
  return `${v.toLocaleString('en-US', { maximumFractionDigits: 2 })} SOL`;
}
function hhmm(ts: number): string { const d = new Date(ts * 1000); return `${String(d.getUTCHours()).padStart(2, '0')}:${String(d.getUTCMinutes()).padStart(2, '0')}`; }

export function TokenChart({ series, symbol }: { series: MarketSeries; symbol: string }) {
  const [tab, setTab] = useState<Tab>('price');
  const [hover, setHover] = useState<number | null>(null);
  const values = useMemo(() => series.map((p) => p[tab]), [series, tab]);
  const n = values.length;
  const max = Math.max(...values, 0), min = tab === 'raids' ? 0 : Math.min(...values);
  const span = max - min || 1;
  const x = (i: number) => PAD.l + (i / Math.max(1, n - 1)) * (W - PAD.l - PAD.r);
  const y = (v: number) => PAD.t + (1 - (v - min) / span) * (H - PAD.t - PAD.b);
  const path = values.map((v, i) => `${i ? 'L' : 'M'}${x(i).toFixed(1)} ${y(v).toFixed(1)}`).join(' ');
  const ticks = [0, 0.5, 1].map((f) => min + f * span);
  const first = values[0] ?? 0, last = values[n - 1] ?? 0;
  const delta = first ? ((last - first) / first) * 100 : 0;
  const barW = Math.max(2, ((W - PAD.l - PAD.r) / Math.max(1, n)) - 2);
  return (
    <>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '0 4px' }}>
        <span className="k">{symbol} · 24h</span>
        <span style={{ font: '600 13px var(--mono)', color: tab === 'raids' ? 'var(--ink)' : delta >= 0 ? 'var(--good)' : 'var(--bad)' }}>
          {tab === 'raids' ? `${values.reduce((a, b) => a + b, 0).toLocaleString('en-US', { maximumFractionDigits: 1 })} SOL raided` : `${delta >= 0 ? '+' : ''}${delta.toFixed(1)}%`}
        </span>
        <div className="chart-tabs" role="tablist">{TABS.map(([t, label]) => <button key={t} role="tab" aria-selected={tab === t} className={tab === t ? 'on' : ''} type="button" onClick={() => { setTab(t); setHover(null); }}>{label}</button>)}</div>
      </div>
      <div className="chart">
        <svg viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none" role="img" aria-label={`${TABS.find(([t]) => t === tab)?.[1]} over 24 hours`}
          onMouseMove={(e) => { const r = e.currentTarget.getBoundingClientRect(); const px = ((e.clientX - r.left) / r.width) * W; setHover(Math.max(0, Math.min(n - 1, Math.round(((px - PAD.l) / (W - PAD.l - PAD.r)) * (n - 1))))); }}
          onMouseLeave={() => setHover(null)}>
          {ticks.map((t) => <g key={t}><line className="grid-line" x1={PAD.l} x2={W - PAD.r} y1={y(t)} y2={y(t)} /><text className="axis-label" x={PAD.l - 6} y={y(t) + 3} textAnchor="end">{tab === 'price' ? t.toPrecision(3) : t.toLocaleString('en-US', { maximumFractionDigits: 1 })}</text></g>)}
          {tab === 'raids'
            ? values.map((v, i) => <rect key={i} className="bar raid" x={x(i) - barW / 2} y={y(v)} width={barW} height={Math.max(0, y(0) - y(v))} rx={2} opacity={hover === null || hover === i ? 1 : 0.5} />)
            : <><path className="area" d={`${path} L${x(n - 1).toFixed(1)} ${y(min)} L${x(0).toFixed(1)} ${y(min)} Z`} /><path className="series" d={path} /></>}
          {hover !== null && <><line className="crosshair" x1={x(hover)} x2={x(hover)} y1={PAD.t} y2={H - PAD.b} />{tab !== 'raids' && <circle className="marker" cx={x(hover)} cy={y(values[hover]!)} r={4} />}</>}
        </svg>
        {hover !== null && series[hover] && (
          <div className="chart-tip" style={{ left: `${(x(hover) / W) * 100}%`, top: 8, transform: x(hover) > W * 0.7 ? 'translateX(calc(-100% - 8px))' : 'translateX(8px)' }}>
            <span>{hhmm(series[hover]!.ts)} UTC</span>{fmt(tab, values[hover]!)}
          </div>
        )}
      </div>
      <div className="chart-foot"><span>{series[0] ? hhmm(series[0].ts) : ''}</span><span>{series[n - 1] ? hhmm(series[n - 1]!.ts) : ''} UTC</span></div>
    </>
  );
}
