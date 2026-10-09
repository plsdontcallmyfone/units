'use client';
import { useEffect, useRef, useState } from 'react';
import { AreaSeries, ColorType, CrosshairMode, HistogramSeries, LineStyle, createChart, type IChartApi, type UTCTimestamp } from 'lightweight-charts';

/** TradingView Lightweight Charts: one series at a time, one axis. Price and chest as an area,
 * raid volume as a histogram. The figures come from the market read; nothing here is computed. */
export type MarketSeries = { ts: number; price: number; chest: number; raids: number }[];
type Tab = 'price' | 'chest' | 'raids';
const TABS: [Tab, string][] = [['price', 'Price'], ['chest', 'Chest'], ['raids', 'Raids']];
const INK = '#ece9df', INK2 = '#bdb9ac', MUTED = '#7b786f', GOOD = '#c9f5a3', LINE = 'rgba(236,233,223,0.08)';

export function TokenChart({ series, symbol }: { series: MarketSeries; symbol: string }) {
  const [tab, setTab] = useState<Tab>('price');
  const box = useRef<HTMLDivElement>(null);
  const chart = useRef<IChartApi | null>(null);
  const values = series.map((p) => p[tab]);
  const first = values[0] ?? 0, last = values[values.length - 1] ?? 0;
  const delta = first ? ((last - first) / first) * 100 : 0;

  useEffect(() => {
    const el = box.current;
    if (!el) return;
    const c = createChart(el, {
      autoSize: true,
      layout: { background: { type: ColorType.Solid, color: 'transparent' }, textColor: MUTED, fontFamily: 'var(--font-mono), ui-monospace, monospace', fontSize: 10, attributionLogo: false },
      grid: { vertLines: { color: 'transparent' }, horzLines: { color: LINE } },
      rightPriceScale: { borderColor: LINE, scaleMargins: { top: 0.08, bottom: tab === 'raids' ? 0 : 0.08 } },
      timeScale: { borderColor: LINE, timeVisible: true, secondsVisible: false, fixLeftEdge: true, fixRightEdge: true, tickMarkFormatter: (t: number) => { const d = new Date(t * 1000); return `${String(d.getUTCHours()).padStart(2, '0')}:${String(d.getUTCMinutes()).padStart(2, '0')}`; } },
      crosshair: { mode: CrosshairMode.Magnet, vertLine: { color: INK2, width: 1, style: LineStyle.Dashed, labelBackgroundColor: '#050505' }, horzLine: { color: INK2, width: 1, style: LineStyle.Dashed, labelBackgroundColor: '#050505' } },
      handleScroll: false, handleScale: false,
      localization: { timeFormatter: (t: number) => { const d = new Date(t * 1000); return `${String(d.getUTCHours()).padStart(2, '0')}:${String(d.getUTCMinutes()).padStart(2, '0')} UTC`; }, priceFormatter: (v: number) => tab === 'price' ? v.toPrecision(3) : v.toLocaleString('en-US', { maximumFractionDigits: 1 }) },
    });
    chart.current = c;
    const data = series.map((p) => ({ time: p.ts as UTCTimestamp, value: p[tab] }));
    if (tab === 'raids') {
      c.addSeries(HistogramSeries, { color: GOOD, priceFormat: { type: 'custom', formatter: (v: number) => `${v.toFixed(2)} SOL` }, priceLineVisible: false, lastValueVisible: false }).setData(data);
    } else {
      c.addSeries(AreaSeries, { lineColor: INK, lineWidth: 2, topColor: 'rgba(236,233,223,0.10)', bottomColor: 'rgba(236,233,223,0)', priceLineColor: MUTED, priceLineStyle: LineStyle.Dotted, crosshairMarkerRadius: 4, crosshairMarkerBackgroundColor: '#050505', crosshairMarkerBorderColor: INK, crosshairMarkerBorderWidth: 2 }).setData(data);
    }
    c.timeScale().fitContent();
    return () => { c.remove(); chart.current = null; };
  }, [series, tab]);

  return (
    <>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '0 4px' }}>
        <span className="k">{symbol} · 24h</span>
        <span style={{ font: '600 13px var(--mono)', color: tab === 'raids' ? 'var(--ink)' : delta >= 0 ? 'var(--good)' : 'var(--bad)' }}>
          {tab === 'raids' ? `${values.reduce((a, b) => a + b, 0).toLocaleString('en-US', { maximumFractionDigits: 1 })} SOL raided` : `${delta >= 0 ? '+' : ''}${delta.toFixed(1)}%`}
        </span>
        <div className="chart-tabs" role="tablist">{TABS.map(([t, label]) => <button key={t} role="tab" aria-selected={tab === t} className={tab === t ? 'on' : ''} type="button" onClick={() => setTab(t)}>{label}</button>)}</div>
      </div>
      <div className="chart" ref={box} role="img" aria-label={`${TABS.find(([t]) => t === tab)?.[1]} over 24 hours`} />
    </>
  );
}
