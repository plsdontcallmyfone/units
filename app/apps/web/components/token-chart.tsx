'use client';
import { useEffect, useRef, useState } from 'react';
import { AreaSeries, CandlestickSeries, ColorType, CrosshairMode, HistogramSeries, LineStyle, createChart, type IChartApi, type UTCTimestamp } from 'lightweight-charts';
import type { Candle } from '@/lib/mock';

/** TradingView Lightweight Charts. Price: candlesticks with a volume histogram on its own scale
 * at the bottom. Chest: an area. Raids: a histogram. Figures come from the market read as they are. */
type Tab = 'price' | 'chest' | 'raids';
const TABS: [Tab, string][] = [['price', 'Price'], ['chest', 'Chest'], ['raids', 'Raids']];
const INK = '#1a1a19', INK2 = '#55554f', MUTED = '#6c6962', GOOD = '#79a636', BAD = '#d0463b', BG = '#f8f8f8';
const GRID = 'rgba(26,26,25,0.07)', BORDER = 'rgba(26,26,25,0.14)';
const hhmm = (t: number) => { const d = new Date(t * 1000); return `${String(d.getUTCHours()).padStart(2, '0')}:${String(d.getUTCMinutes()).padStart(2, '0')}`; };

export function TokenChart({ series, symbol }: { series: Candle[]; symbol: string }) {
  const [tab, setTab] = useState<Tab>('price');
  const box = useRef<HTMLDivElement>(null);
  const chart = useRef<IChartApi | null>(null);
  const first = series[0], last = series[series.length - 1];
  const delta = first && last ? ((last.close - first.open) / first.open) * 100 : 0;
  const chestDelta = first && last ? last.chest - first.chest : 0;
  const raided = series.reduce((a, c) => a + c.raids, 0);

  useEffect(() => {
    const el = box.current;
    if (!el) return;
    const c = createChart(el, {
      autoSize: true,
      layout: { background: { type: ColorType.Solid, color: 'transparent' }, textColor: INK2, fontFamily: '"Suisse Intl", system-ui, sans-serif', fontSize: 11, attributionLogo: false },
      grid: { vertLines: { color: GRID, style: LineStyle.Solid }, horzLines: { color: GRID, style: LineStyle.Solid } },
      rightPriceScale: { borderColor: BORDER, scaleMargins: { top: 0.06, bottom: tab === 'price' ? 0.24 : tab === 'raids' ? 0 : 0.06 }, entireTextOnly: true },
      timeScale: { borderColor: BORDER, timeVisible: true, secondsVisible: false, fixLeftEdge: true, fixRightEdge: true, rightOffset: 2, barSpacing: 8, tickMarkFormatter: (t: number) => hhmm(t) },
      crosshair: { mode: CrosshairMode.Normal, vertLine: { color: INK2, width: 1, style: LineStyle.Dashed, labelBackgroundColor: INK }, horzLine: { color: INK2, width: 1, style: LineStyle.Dashed, labelBackgroundColor: INK } },
      handleScroll: { mouseWheel: false, pressedMouseMove: true, horzTouchDrag: true, vertTouchDrag: false }, handleScale: { mouseWheel: true, pinch: true, axisPressedMouseMove: true, axisDoubleClickReset: true },
      localization: { timeFormatter: (t: number) => `${hhmm(t)} UTC`, priceFormatter: (v: number) => tab === 'price' ? v.toPrecision(4) : v.toLocaleString('en-US', { maximumFractionDigits: 2 }) },
    });
    chart.current = c;
    const time = (p: Candle) => p.ts as UTCTimestamp;
    if (tab === 'price') {
      c.addSeries(CandlestickSeries, { upColor: GOOD, downColor: BAD, borderUpColor: GOOD, borderDownColor: BAD, wickUpColor: GOOD, wickDownColor: BAD, priceLineColor: MUTED, priceLineStyle: LineStyle.Dotted, priceFormat: { type: 'price', precision: 6, minMove: 0.000001 } })
        .setData(series.map((p) => ({ time: time(p), open: p.open, high: p.high, low: p.low, close: p.close })));
      const vol = c.addSeries(HistogramSeries, { priceScaleId: 'volume', priceFormat: { type: 'volume' }, priceLineVisible: false, lastValueVisible: false });
      vol.priceScale().applyOptions({ scaleMargins: { top: 0.8, bottom: 0 } });
      vol.setData(series.map((p) => ({ time: time(p), value: p.volume, color: p.close >= p.open ? 'rgba(121,166,54,0.45)' : 'rgba(208,70,59,0.45)' })));
    } else if (tab === 'chest') {
      c.addSeries(AreaSeries, { lineColor: INK, lineWidth: 2, topColor: 'rgba(26,26,25,0.10)', bottomColor: 'rgba(26,26,25,0)', priceLineColor: MUTED, priceLineStyle: LineStyle.Dotted, crosshairMarkerRadius: 4, crosshairMarkerBackgroundColor: BG, crosshairMarkerBorderColor: INK, crosshairMarkerBorderWidth: 2, priceFormat: { type: 'custom', formatter: (v: number) => `${v.toFixed(2)} SOL` } })
        .setData(series.map((p) => ({ time: time(p), value: p.chest })));
    } else {
      c.addSeries(HistogramSeries, { color: GOOD, priceLineVisible: false, lastValueVisible: false, priceFormat: { type: 'custom', formatter: (v: number) => `${v.toFixed(2)} SOL` } })
        .setData(series.map((p) => ({ time: time(p), value: p.raids })));
    }
    c.timeScale().fitContent();
    return () => { c.remove(); chart.current = null; };
  }, [series, tab]);

  return (
    <>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '0 4px' }}>
        <span className="k">{symbol} · 24h · 15m</span>
        <span style={{ font: '600 13px var(--sans)', color: tab === 'raids' ? 'var(--ink)' : (tab === 'price' ? delta : chestDelta) >= 0 ? 'var(--good)' : 'var(--bad)' }}>
          {tab === 'price' ? `${delta >= 0 ? '+' : ''}${delta.toFixed(1)}%` : tab === 'chest' ? `${chestDelta >= 0 ? '+' : ''}${chestDelta.toFixed(2)} SOL` : `${raided.toLocaleString('en-US', { maximumFractionDigits: 1 })} SOL raided`}
        </span>
        {last ? <span className="k" style={{ marginLeft: 4 }}>{tab === 'price' ? `O ${last.open.toPrecision(3)} H ${last.high.toPrecision(3)} L ${last.low.toPrecision(3)} C ${last.close.toPrecision(3)}` : ''}</span> : null}
        <div className="chart-tabs" role="tablist">{TABS.map(([t, label]) => <button key={t} role="tab" aria-selected={tab === t} className={tab === t ? 'on' : ''} type="button" onClick={() => setTab(t)}>{label}</button>)}</div>
      </div>
      <div className="chart" ref={box} role="img" aria-label={`${TABS.find(([t]) => t === tab)?.[1]} over 24 hours`} />
    </>
  );
}
