// Changed by Hookwars: new file (explorer v2). Screenshots of the explorer and launch pages, desktop and
// mobile, in demo mode (MOCK_DATA=1, port 9966) and against the API (port 9968), each checked for
// horizontal overflow, page errors, monospace text, em dashes, the "Demo data" label in demo mode and
// the lowercase wordmark. Run from a directory where `playwright` resolves: node shots.mjs <out-dir>.
import { chromium } from 'playwright';
const D = 'http://127.0.0.1:9966', R = 'http://127.0.0.1:9968';
const sig = {
  buy: 'xQs4JE593e3HaeUQ3cRDxHDLvyyP7ds1HF4fhT3Shhwcrn4MW1QEJoKCDWtLNMW3UvPZk2BNSqYbHAV95teNuD4',
  sell: '32p87r9JxXL4dyouWEtGiHkmafg1AATaFDvA3iUGDJZpoySDXYNnjWVc86PUAKSea5qzFA59KAboSuaRhGs7FfAV',
  settle: '2Y3Pj5n18rm1xotcWtmdU2pLnv7QDUxa73ZKy94HhMGM3Ei9Ja5JzGBjkiqrfpJodVM9xRAurZWB6PqxTgChKtaY',
  raid: '5R2PZ8P3EWGsMbKDcoBcU5UYF6VAE4cC1o3J8bwDdpdhr7N7YKyhu7FCLnB5f1Jyd4xabQfXdEbsHksip8icvQ9b',
  refused: '3ZGEFBSQ64QSUgGf1XTZeAi7UDFivJgKR9YtJ8rafYcgamVGZqKighqzUzH5CRRbSUmW3QX3yv69iAaEusLTakJa',
  cut: '3xFRXuJKNedy99zG6AE7pMsZbtaedxjUtey6LTugmBGwf42qE1AmV61omYLgRfapbscH6qcR7xQwh4EZd3fDfG4m',
};
const pages = [
  ['demo_explorer', D, '/explorer'],
  ['demo_tx_slot_buy', D, `/tx/${sig.buy}`],
  ['demo_tx_slot_sell', D, `/tx/${sig.sell}`],
  ['demo_tx_settle', D, `/tx/${sig.settle}`],
  ['demo_tx_raid_delivery', D, `/tx/${sig.raid}`],
  ['demo_tx_transfer_cut', D, `/tx/${sig.cut}`],
  ['demo_tx_refused', D, `/tx/${sig.refused}`],
  ['demo_address_mint', D, '/address/NpiYgJq1EQbN6q5Dx17KFsv7BgD5wooT9BPNS8HPFxe'],
  ['demo_program_items', D, '/program/8wMqHBAWhKxw2fNpczHfMohKjowbUM4hGqQkoYPf93Gv'],
  ['demo_launch', D, '/launch', 'launch'],
  ['api_explorer', R, '/explorer'],
  ['api_program_token', R, '/program/5yeVq5rEWWBRkBWiA49So9u4jpxeZQTeYsjQcFRwX618'],
  ['api_tx_unknown', R, `/tx/${sig.buy}`],
  ['api_address', R, '/address/So11111111111111111111111111111111111111112'],
  ['api_launch', R, '/launch', 'launch'],
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
  const wantDemo = base === D;
  const ok = status < 500 && over <= 0 && mono === 0 && !dash && demo === wantDemo && !wordmark && errs.length === 0;
  if (!ok) bad++;
  console.log(vw, name, status, 'overflow', over, 'mono', mono, 'emdash', dash, 'demo', demo, 'wordmark', wordmark, errs.length ? 'ERR ' + errs.slice(0, 2).join(' | ') : '', ok ? 'ok' : 'BAD');
}
for (const [vw, size] of [['desktop', { width: 1440, height: 900 }], ['mobile', { width: 390, height: 844 }]]) {
  const ctx = await b.newContext({ viewport: size });
  for (const [name, base, path, flow] of pages) {
    const p = await ctx.newPage(); const errs = [];
    p.on('pageerror', (e) => errs.push(e.message)); p.on('console', (m) => { if (m.type() === 'error' && !/Failed to load resource/.test(m.text())) errs.push(m.text()); });
    const r = await p.goto(base + path, { waitUntil: 'networkidle' });
    await check(p, name, base, vw, r.status(), errs);
    await p.screenshot({ path: `${out}/${vw}_${name}.png`, fullPage: true });
    if (flow === 'launch') {
      // Walk the draft: token, rules with the kit, two slots with templates, then the review.
      await p.fill('input[placeholder^="Shown on the board"]', 'Ashfall');
      await p.fill('input[placeholder^="1 to 10"]', 'ASH');
      await p.fill('input[placeholder*="sets the opening price"]', '30');
      await p.getByRole('button', { name: 'Next' }).click();
      await p.getByLabel(/Kit rules/).check();
      await p.screenshot({ path: `${out}/${vw}_${name}_rules.png`, fullPage: true });
      await p.getByRole('button', { name: 'Next' }).click();
      await p.getByRole('button', { name: 'Add a slot' }).click();
      await p.getByRole('radio', { name: /^Raid/ }).first().click();
      await p.getByRole('button', { name: 'Add a slot' }).click();
      await p.locator('select').nth(2).selectOption('fee');
      await p.getByRole('radio', { name: /^Half-Life/ }).first().click();
      await p.getByRole('button', { name: 'Build a composite item for this slot' }).first().click();
      await p.getByRole('button', { name: 'Add a module' }).click();
      await p.getByRole('button', { name: 'Add a module' }).click();
      await check(p, `${name}_slots`, base, vw, 200, errs);
      await p.screenshot({ path: `${out}/${vw}_${name}_slots.png`, fullPage: true });
      await p.getByRole('button', { name: 'Close' }).click();
      await p.getByRole('button', { name: 'Next' }).click();
      await check(p, `${name}_review`, base, vw, 200, errs);
      await p.screenshot({ path: `${out}/${vw}_${name}_review.png`, fullPage: true });
      await p.getByRole('button', { name: 'Next' }).click();
      await check(p, `${name}_walk`, base, vw, 200, errs);
      await p.screenshot({ path: `${out}/${vw}_${name}_walk.png`, fullPage: true });
      await p.evaluate(() => localStorage.clear());
    }
    await p.close();
  }
  await ctx.close();
}
await b.close(); console.log('bad', bad);
