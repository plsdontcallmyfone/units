// App pass v3 screenshots: /craft, /book, an item page, the launch form's companion option, desktop
// and mobile, in demo mode (MOCK_DATA=1, port 9966) and against the API (port 9968). Each page is
// checked for overflow, page errors, monospace text, em dashes, the Demo data label and the wordmark.
import { chromium } from 'playwright';
const D = 'http://127.0.0.1:9966', R = 'http://127.0.0.1:9968';
const item = process.argv[3];
const pages = [
  ['demo_craft', D, '/craft'], ['demo_book', D, '/book'], ['demo_item', D, `/armory/items/${item}`], ['demo_launch_companion', D, '/launch', 'companion'],
  ['api_craft', R, '/craft'], ['api_book', R, '/book'],
];
const out = process.argv[2];
const b = await chromium.launch();
let bad = 0;
async function check(p, name, base, vw, status, errs) {
  const over = await p.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  const mono = await p.evaluate(() => [...document.querySelectorAll('body *')].filter((e) => [...e.childNodes].some((n) => n.nodeType === 3 && n.textContent.trim()) && /mono/i.test(getComputedStyle(e).fontFamily) && !e.closest('pre,code')).length);
  const text = await p.evaluate(() => document.body.innerText);
  const dash = text.includes('—');
  const demo = text.includes('Demo data');
  const wordmark = /\bUnits\b|\bUNITS\b|Hookwars/.test(text.replace(/compute units/gi, ''));
  const ok = status < 500 && over <= 0 && mono === 0 && !dash && demo === (base === D) && !wordmark && errs.length === 0;
  if (!ok) bad++;
  console.log(vw, name, status, 'overflow', over, 'mono', mono, 'emdash', dash, 'demo', demo, 'wordmark', wordmark, errs.length ? 'ERR ' + errs.slice(0, 2).join(' | ') : '', ok ? 'ok' : 'BAD');
}
for (const [vw, size] of [['desktop', { width: 1440, height: 900 }], ['mobile', { width: 390, height: 844 }]]) {
  const ctx = await b.newContext({ viewport: size });
  for (const [name, base, path, flow] of pages) {
    const p = await ctx.newPage(); const errs = [];
    p.on('pageerror', (e) => errs.push(e.message)); p.on('console', (m) => { if (m.type() === 'error' && !/Failed to load resource/.test(m.text())) errs.push(m.text()); });
    const r = await p.goto(base + path, { waitUntil: 'networkidle' });
    if (flow === 'companion') { await p.locator("select:has(option[value=companion])").selectOption('companion'); }
    await check(p, name, base, vw, r.status(), errs);
    await p.screenshot({ path: `${out}/${vw}_${name}.png`, fullPage: true });
    if (flow) await p.evaluate(() => localStorage.clear());
    await p.close();
  }
  await ctx.close();
}
await b.close(); console.log('bad', bad);
