// Shared fixtures: a page watcher (console errors, page errors, failed requests), a stand-in
// wallet injected as window.solana, and helpers that answer the site's prepare and submit routes
// with a real, unsigned transaction so a form runs end to end without a cluster.
import { test as base, expect, type Page, type Request, type Route } from '@playwright/test';
import { Keypair, PublicKey, SystemProgram, TransactionMessage, VersionedTransaction } from '@solana/web3.js';

/** The stand-in wallet's key: fixed, so request bodies can be checked. Holds nothing anywhere. */
export const WALLET = Keypair.fromSeed(new Uint8Array(32).fill(7)).publicKey.toBase58();
export const SIGNATURE = '5'.repeat(64).replace(/^5/, '4');

/** Base64 of an unsigned v0 transaction paid by the stand-in wallet (a 0-lamport self transfer). */
export function unsignedTx(payer = WALLET): string {
  const key = new PublicKey(payer);
  const msg = new TransactionMessage({
    payerKey: key,
    recentBlockhash: new PublicKey(new Uint8Array(32).fill(1)).toBase58(),
    instructions: [SystemProgram.transfer({ fromPubkey: key, toPubkey: key, lamports: 0 })],
  }).compileToV0Message();
  return Buffer.from(new VersionedTransaction(msg).serialize()).toString('base64');
}

export function preparedTx(label = 'e2e', stage = 0) {
  return { transaction: unsignedTx(), version: 'v0' as const, stage, label, extraSigners: [] as string[] };
}

/** Injects window.solana before any page script: no key until connect(), signs by returning the
 * transactions unchanged (the submit route is answered by the test, nothing reaches a cluster). */
export async function installWallet(page: Page, key = WALLET): Promise<void> {
  await page.addInitScript((k: string) => {
    const pk = { toBase58: () => k, toString: () => k };
    const w: Record<string, unknown> = {
      isPhantom: true,
      publicKey: undefined,
      async connect() { w.publicKey = pk; return { publicKey: pk }; },
      async disconnect() { w.publicKey = undefined; },
      async signAllTransactions<T>(txs: T[]) { return txs; },
      async signTransaction<T>(tx: T) { return tx; },
      on() {}, off() {},
    };
    Object.defineProperty(window, 'solana', { value: w, configurable: true });
  }, key);
}

export type Captured = { route: string; body: Record<string, unknown> };

/** Answers POST /api/v1/<route>/prepare with one prepared transaction and records the body, and
 * answers /api/v1/submit with a signature. */
export async function answerPrepare(page: Page, route: string, out: Captured[]): Promise<void> {
  await page.route(`**/api/v1/${route}/prepare`, async (r: Route) => {
    out.push({ route, body: r.request().postDataJSON() as Record<string, unknown> });
    await r.fulfill({ json: { transactions: [preparedTx(route)] } });
  });
  await page.route('**/api/v1/submit', (r) => r.fulfill({ json: { signature: SIGNATURE } }));
}

export type Problem = { kind: 'console' | 'pageerror' | 'response' | 'requestfailed'; text: string };

/** Collects what a page reports as broken. Same-origin responses only: a third-party image is the
 * page's choice, a failing /api route is ours. */
export function watch(page: Page, origin: string): Problem[] {
  const out: Problem[] = [];
  page.on('console', (m) => { if (m.type() === 'error') out.push({ kind: 'console', text: m.text() }); });
  page.on('pageerror', (e) => out.push({ kind: 'pageerror', text: e.message }));
  page.on('response', (r) => { if (r.url().startsWith(origin) && r.status() >= 400) out.push({ kind: 'response', text: `${r.status()} ${r.request().method()} ${r.url()}` }); });
  page.on('requestfailed', (r: Request) => {
    const f = r.failure()?.errorText ?? '';
    // A navigation away cancels in-flight reads; that is not a failure of the page.
    if (r.url().startsWith(origin) && !/ERR_ABORTED|NS_BINDING_ABORTED/.test(f)) out.push({ kind: 'requestfailed', text: `${f} ${r.url()}` });
  });
  return out;
}

type Fixtures = { problems: Problem[]; wallet: string };

export const test = base.extend<Fixtures>({
  problems: async ({ page, baseURL }, use) => { await use(watch(page, new URL(baseURL!).origin)); },
  wallet: async ({ page }, use) => { await installWallet(page); await use(WALLET); },
});
export { expect };

/** The checks every page must pass, run in the page. */
export async function pageFacts(page: Page): Promise<{ emDash: string[]; mono: string[]; overflow: number }> {
  return page.evaluate(() => {
    const MONO = /(^|,)\s*["']?(monospace|ui-monospace|menlo|monaco|consolas|courier|courier new|sf mono|jetbrains mono|fira code|fira mono|source code pro|ibm plex mono|roboto mono|geist mono|dejavu sans mono|liberation mono)["']?\s*(,|$)/i;
    const emDash: string[] = [];
    const mono: string[] = [];
    const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
    const seen = new Set<Element>();
    for (let n = walker.nextNode(); n; n = walker.nextNode()) {
      const text = n.textContent ?? '';
      if (!text.trim()) continue;
      const el = n.parentElement;
      if (!el || el.closest('script, style, noscript, template')) continue;
      const box = el.getBoundingClientRect();
      const visible = box.width > 0 && box.height > 0 && getComputedStyle(el).visibility !== 'hidden';
      if (!visible) continue;
      if (text.includes(String.fromCharCode(0x2014))) emDash.push(text.trim().slice(0, 80));
      // Code is allowed in a monospace face; UI text is not.
      if (el.closest('pre, code, kbd, samp')) continue;
      if (seen.has(el)) continue;
      seen.add(el);
      const family = getComputedStyle(el).fontFamily.split(',')[0] ?? '';
      if (MONO.test(family)) mono.push(`${el.tagName.toLowerCase()}${el.className && typeof el.className === 'string' ? '.' + el.className.split(' ')[0] : ''}: ${family} "${text.trim().slice(0, 40)}"`);
    }
    const overflow = document.documentElement.scrollWidth - document.documentElement.clientWidth;
    return { emDash: [...new Set(emDash)].slice(0, 10), mono: [...new Set(mono)].slice(0, 10), overflow };
  });
}

/** The action card a page renders for one prepare route, found by its title. */
export function action(page: Page, title: string) {
  return page.locator('.action').filter({ has: page.locator('.action-head b', { hasText: new RegExp(`^${title.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}$`) }) }).first();
}
