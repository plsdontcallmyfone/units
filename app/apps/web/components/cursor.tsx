'use client';
import { useEffect, useRef } from 'react';

/** The reference's drawn cursor (public/plnty/homepage/cursors): pointer, hover over links and
 * buttons, click while pressed, grab while dragging, text over inputs. Fine pointers only; the
 * landing frame runs the reference's own script, so both show the same cursor. */
const LAYERS = [
  ['pointer', '/plnty/homepage/cursors/pointer.svg', 80, 64, '--hx:-5px;--hy:-5px'],
  ['hover', '/plnty/homepage/cursors/hover.svg', 89, 75, '--hx:-20.58px;--hy:-1px;--px:19.75px;--py:38.3px'],
  ['click', '/plnty/homepage/cursors/click.svg', 89, 75, '--hx:-15.34px;--hy:-1px'],
  ['grab', '/plnty/homepage/cursors/grab.svg', 89, 69, '--hx:-20.28px;--hy:-1px'],
  ['text', '/plnty/homepage/cursors/text.svg', 88, 75, '--hx:-18px;--hy:-23px'],
] as const;
const styleOf = (s: string) => Object.fromEntries(s.split(';').map((kv) => kv.split(':') as [string, string]));

export function Cursor() {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const el = ref.current; if (!el) return;
    if (!window.matchMedia('(hover: hover) and (pointer: fine)').matches) return;
    document.documentElement.setAttribute('data-hp-cursor-active', 'on');
    let x = 0, y = 0, tx = 0, ty = 0, down = false, over: Element | null = null, raf = 0, shown = false;
    const state = () => {
      const t = over as HTMLElement | null;
      const text = !!t?.closest('input:not([type=checkbox]):not([type=range]), textarea, [contenteditable]');
      const act = !!t?.closest('a, button, label, select, summary, [role=button], [role=tab], .card, .rail-item');
      const drag = !!t?.closest('[data-grab], .chart');
      el.dataset.hpState = text ? 'text' : down ? (drag ? 'grab' : 'click') : act ? 'hover' : 'pointer';
    };
    const tick = () => { tx += (x - tx) * 0.35; ty += (y - ty) * 0.35; el.style.transform = `translate(${tx.toFixed(1)}px, ${ty.toFixed(1)}px)`; raf = requestAnimationFrame(tick); };
    const move = (e: PointerEvent) => { x = e.clientX; y = e.clientY; over = e.target as Element; if (!shown) { shown = true; tx = x; ty = y; el.hidden = false; } state(); };
    const dn = () => { down = true; state(); }; const up = () => { down = false; state(); };
    const leave = () => { el.hidden = true; shown = false; }; const enter = () => { if (!el.hidden) return; };
    window.addEventListener('pointermove', move, { passive: true }); window.addEventListener('pointerdown', dn); window.addEventListener('pointerup', up);
    document.documentElement.addEventListener('mouseleave', leave); document.documentElement.addEventListener('mouseenter', enter);
    raf = requestAnimationFrame(tick);
    return () => { cancelAnimationFrame(raf); window.removeEventListener('pointermove', move); window.removeEventListener('pointerdown', dn); window.removeEventListener('pointerup', up); document.documentElement.removeEventListener('mouseleave', leave); document.documentElement.removeEventListener('mouseenter', enter); document.documentElement.removeAttribute('data-hp-cursor-active'); };
  }, []);
  return <div ref={ref} className="hp-cursor" data-hp-state="pointer" hidden aria-hidden>{LAYERS.map(([st, src, w, h, css]) => <img key={st} className="layer" data-state={st} src={src} width={w} height={h} style={styleOf(css) as React.CSSProperties} alt="" draggable={false} />)}</div>;
}
