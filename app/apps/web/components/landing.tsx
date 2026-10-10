'use client';
import { useEffect, useRef } from 'react';

/** Frames the archived site from this origin and hides its own ticker and header so ours is the
 * only one. Same origin, so the frame's document can be styled once it loads. */
export function Landing() {
  const ref = useRef<HTMLIFrameElement>(null);
  useEffect(() => {
    const f = ref.current; if (!f) return;
    const style = () => {
      const d = f.contentDocument; if (!d) return;
      if (d.getElementById('units-frame')) return;
      const s = d.createElement('style'); s.id = 'units-frame';
      s.textContent = ':root{--hp-ticker-height:0px!important;--hp-bar-height:0px!important}.ticker,.header{display:none!important}a[href="/"],a[href="/plnty/index.html"]{pointer-events:none}';
      d.head.appendChild(s);
    };
    f.addEventListener('load', style); style();
    return () => f.removeEventListener('load', style);
  }, []);
  return <iframe ref={ref} className="landing" title="units" src="/plnty/index.html" />;
}
