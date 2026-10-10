'use client';
// Changed by Hookwars: new file (economy panel). Column charts over time and a split bar, drawn
// from the API's figures only: a bucket nothing was recorded in draws nothing, never an estimate.
import { useState } from 'react';

export type Series = { key: string; label: string; color: string };
export type Bucket = { t: number; values: Record<string, number> };

const lamportsToSol = (v: number): string => `${(v / 1e9).toLocaleString('en-US', { maximumFractionDigits: v >= 1e9 ? 2 : 6 })} SOL`;

function tickLabel(t: number, bucketSecs: number): string {
  const d = new Date(t * 1000);
  if (bucketSecs < 86_400) return d.toLocaleTimeString('en-US', { hour: 'numeric', hour12: false, timeZone: 'UTC' }) + 'h';
  return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric', timeZone: 'UTC' });
}

/** A nice ceiling for the axis: 1, 2 or 5 times a power of ten. */
function niceMax(v: number): number {
  if (v <= 0) return 1;
  const p = 10 ** Math.floor(Math.log10(v));
  for (const m of [1, 2, 5, 10]) if (m * p >= v) return m * p;
  return 10 * p;
}

/**
 * Columns per time bucket, stacked (parts of one total) or grouped (two measures side by side).
 * Buckets run from `from` to `to` at `bucketSecs`, so an empty stretch reads as empty time. Values
 * are lamports; the hover shows each series and the bucket's total.
 */
export function Columns({ series, buckets, from, to, bucketSecs, mode = 'stack', label }: { series: Series[]; buckets: Bucket[]; from: number; to: number; bucketSecs: number; mode?: 'stack' | 'group'; label: string }) {
  const [hover, setHover] = useState<number | null>(null);
  const start = Math.floor(from / bucketSecs) * bucketSecs;
  const slots: Bucket[] = [];
  const byT = new Map(buckets.map((b) => [b.t, b]));
  // At most 120 columns: a longer window falls back to the buckets that hold data.
  if ((to - start) / bucketSecs <= 120) for (let t = start; t <= to; t += bucketSecs) slots.push(byT.get(t) ?? { t, values: {} });
  else slots.push(...buckets);
  const total = (b: Bucket) => series.reduce((a, s) => a + (b.values[s.key] ?? 0), 0);
  const peak = Math.max(0, ...slots.map((b) => (mode === 'stack' ? total(b) : Math.max(0, ...series.map((s) => b.values[s.key] ?? 0)))));
  const max = niceMax(peak);
  const W = 1000, H = 240, n = Math.max(1, slots.length), slotW = W / n, gap = Math.min(6, slotW * 0.25), colW = slotW - gap;
  const r = (v: number) => Math.round(v * 100) / 100;
  const ticks = [0, 0.5, 1].map((f) => f * max);
  const every = Math.max(1, Math.ceil(n / 6));
  const h = hover === null ? null : slots[hover];
  return (
    <figure className="econ-chart" aria-label={label}>
      <div className="econ-chart-plot">
        <div className="econ-chart-y" aria-hidden>{ticks.slice().reverse().map((t) => <span key={t}>{lamportsToSol(t)}</span>)}</div>
        <div className="econ-chart-area" onMouseLeave={() => setHover(null)}>
          <svg viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none" role="img" aria-label={label}>
            {ticks.map((t) => <line key={t} x1={0} x2={W} y1={r(H - (t / max) * H)} y2={r(H - (t / max) * H)} className={t === 0 ? 'base' : 'grid'} vectorEffect="non-scaling-stroke" />)}
            {slots.map((b, i) => {
              const x = i * slotW + gap / 2;
              if (mode === 'group') {
                const w = colW / series.length;
                return <g key={b.t}>{series.map((s, j) => { const v = b.values[s.key] ?? 0; const hh = (v / max) * H; return v > 0 ? <rect key={s.key} x={r(x + j * w)} y={r(H - hh)} width={r(Math.max(1, w - 2))} height={r(hh)} fill={s.color} opacity={hover === null || hover === i ? 1 : 0.45} /> : null; })}</g>;
              }
              let y = H;
              return <g key={b.t}>{series.map((s) => {
                const v = b.values[s.key] ?? 0; if (v <= 0) return null;
                const hh = (v / max) * H; y -= hh;
                return <rect key={s.key} x={r(x)} y={r(y)} width={r(colW)} height={r(Math.max(0, hh - 1))} fill={s.color} opacity={hover === null || hover === i ? 1 : 0.45} />;
              })}</g>;
            })}
            {slots.map((b, i) => <rect key={`hit${b.t}`} x={r(i * slotW)} y={0} width={r(slotW)} height={H} fill="transparent" onMouseEnter={() => setHover(i)} onFocus={() => setHover(i)} tabIndex={0} aria-label={`${tickLabel(b.t, bucketSecs)}: ${series.map((s) => `${s.label} ${lamportsToSol(b.values[s.key] ?? 0)}`).join(', ')}`} />)}
          </svg>
          {h ? (
            <div className="econ-tip" style={{ left: `${Math.min(78, Math.max(0, ((hover! + 0.5) / n) * 100 - 11))}%` }} role="tooltip">
              <div className="econ-tip-t">{tickLabel(h.t, bucketSecs)}{bucketSecs >= 86_400 ? '' : ' UTC'}</div>
              {series.map((s) => <div key={s.key} className="econ-tip-row"><i style={{ background: s.color }} /><span>{s.label}</span><b>{h.values[s.key] ? lamportsToSol(h.values[s.key]!) : '-'}</b></div>)}
              {mode === 'stack' && series.length > 1 ? <div className="econ-tip-row total"><span>Total</span><b>{lamportsToSol(total(h))}</b></div> : null}
            </div>
          ) : null}
        </div>
        <div className="econ-chart-x" aria-hidden>{slots.map((b, i) => <span key={b.t} style={{ left: `${((i + 0.5) / n) * 100}%` }}>{i % every === 0 ? tickLabel(b.t, bucketSecs) : ''}</span>)}</div>
      </div>
    </figure>
  );
}

/** One bar split into parts of a whole, with a legend that carries every value. */
export function SplitBar({ parts, label }: { parts: { key: string; label: string; what: string; color: string; value: number | null; href?: string }[]; label: string }) {
  const total = parts.reduce((a, p) => a + (p.value ?? 0), 0);
  return (
    <figure className="econ-split" aria-label={label}>
      <div className="econ-split-bar" role="img" aria-label={`${label}: ${parts.map((p) => `${p.label} ${p.value === null ? 'none' : lamportsToSol(p.value)}`).join(', ')}`}>
        {total > 0 ? parts.filter((p) => (p.value ?? 0) > 0).map((p) => <span key={p.key} style={{ flexGrow: p.value!, background: p.color }} title={`${p.label}: ${lamportsToSol(p.value!)}`} />) : <span className="none" />}
      </div>
      <table className="econ-legend">
        <tbody>{parts.map((p) => (
          <tr key={p.key}>
            <td><i style={{ background: p.color }} aria-hidden />{p.label}<div className="faint">{p.what}</div></td>
            <td className="num">{p.value === null ? '-' : lamportsToSol(p.value)}</td>
            <td className="num faint">{total > 0 && p.value !== null ? `${((p.value / total) * 100).toFixed(1)}%` : '-'}</td>
          </tr>
        ))}</tbody>
      </table>
    </figure>
  );
}
