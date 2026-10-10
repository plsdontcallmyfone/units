'use client';
// Changed by Hookwars: the launch walked step by step (launch page polish, explorer v2 lane). The
// creator drafts the token, its rules and its slots with every program limit checked as they type;
// the API prepares the staged transactions (prepare_launch, equip_prepared per item, the token's
// lookup table, create_prepared_launch, refresh_pool_registry, init_war) and returns each with its
// instructions decoded from the bytes the wallet will sign, its size and its fee; each step is
// simulated right before it is signed. The draft and the progress survive a reload.
import { useEffect, useMemo, useState } from 'react';
import { Keypair, type VersionedTransaction } from '@solana/web3.js';
import {
  BURN_CHOICES_BPS, CREATOR_FEE_CHOICES_BPS, CREATOR_LOCK_CHOICES_DAYS, EARLY_LOCK_CHOICES_SECS, EARLY_WINDOW_CHOICES_SECS, HOLDER_FEE_CHOICES_BPS,
  MAX_VIRTUAL_QUOTE, MAX_WALLET_CHOICES_BPS, MIN_VIRTUAL_QUOTE, NO_RULES, RULE_BOUNDS, TEMPLATES, checkLaunchRules, itemSentence,
  type EquipRule, type LaunchRulesInput, type PreparedTx, type RuleBounds, type SlotKind,
} from '@hookwars/shared';
import { signAndSend } from '@/lib/sign';
import './launch.css';
import { budget, compositeProblems, MAX_MODULES, MAX_SLOTS, MEASURED, PACKET, slotBytes, answersTouch, mayBurn, type ModuleDraft, type SlotDraft } from './checks';

declare global { interface Window { solana?: { isPhantom?: boolean; publicKey?: { toBase58(): string }; connect(): Promise<{ publicKey: { toBase58(): string } }>; signAllTransactions?<T extends VersionedTransaction>(txs: T[]): Promise<T[]> } } }

const KINDS: SlotKind[] = ['fee', 'reward', 'defense', 'relation', 'pool', 'war'];
const RULES: EquipRule[] = ['locked', 'vote', 'performance'];
const RULE_TEXT: Record<EquipRule, string> = { locked: 'Locked: the launch item stays forever', vote: 'Vote: holders vote an item in or out', performance: 'Performance: reverts when its condition fails' };
const STEPS = ['Token', 'Rules', 'Slots', 'Review', 'Launch'];
const STORE = 'units.launch.v2';
const LAMPORTS = 1_000_000_000n;
const pct = (bps: number) => `${(bps / 100).toLocaleString('en-US', { maximumFractionDigits: 2 })}%`;
const solOf = (l: bigint | string | number) => { const n = BigInt(l); const f = (n % LAMPORTS).toString().padStart(9, '0').replace(/0+$/, ''); return `${n / LAMPORTS}${f ? `.${f}` : ''} SOL`; };
function toLamports(s: string): bigint | null {
  const m = /^(\d+)(?:\.(\d{1,9}))?$/.exec(s.trim());
  return m ? BigInt(m[1]!) * LAMPORTS + BigInt((m[2] ?? '').padEnd(9, '0')) : null;
}

interface PlanStep { stage: number; label: string; instructions: { program: string | null; name: string | null; args: Record<string, unknown> | null }[]; bytes: number; packet: number; lookupTables: number; signatures: number; feeLamports: number; computeLimit: number | null; simulatedNow: boolean }
interface Plan { phase: 'prepare' | 'launch'; transactions: PreparedTx[]; steps: PlanStep[]; launchFeeLamports: string | null }
interface Sim { ok: boolean; units: number | null; failure: { name: string | null; code: number | null; message: string; program: string | null } | null }
interface Config { launchFeeLamports: string; maxCreatorFeeBps: number; lpFeeBps: number; minVirtualQuote: string; maxVirtualQuote: string; paused: boolean; ruleBounds: Record<string, string | number> }
interface ItemView { templateId: number; params: number[]; manifest: Record<string, unknown>; royaltyBps: number; level: number }

interface Draft { name: string; symbol: string; uri: string; virtualSol: string; creatorFeeBps: number; kit: boolean; rules: LaunchRulesInput; slots: SlotDraft[] }
interface Run { mint: number[]; phase: 'prepare' | 'launch' | 'done'; sent: { label: string; sig: string }[] }
const EMPTY: Draft = { name: '', symbol: '', uri: '', virtualSol: '', creatorFeeBps: 0, kit: false, rules: { ...NO_RULES }, slots: [] };

function load(): { draft: Draft; run: Run | null } {
  try { const v = JSON.parse(localStorage.getItem(STORE) ?? 'null') as { draft: Draft; run: Run | null } | null; if (v?.draft) return v; } catch { /* a blocked or empty store starts fresh */ }
  return { draft: EMPTY, run: null };
}
function save(v: { draft: Draft; run: Run | null }) { try { localStorage.setItem(STORE, JSON.stringify(v)); } catch { /* not kept: the page still works */ } }

function argsText(a: Record<string, unknown> | null): string {
  if (!a) return '';
  return Object.entries(a).map(([k, v]) => `${k} ${typeof v === 'object' ? JSON.stringify(v) : String(v)}`).join('; ');
}

export function LaunchForm() {
  const [step, setStep] = useState(0);
  const [draft, setDraft] = useState<Draft>(EMPTY);
  const [run, setRun] = useState<Run | null>(null);
  const [loaded, setLoaded] = useState(false);
  const [cfg, setCfg] = useState<Config | null | 'unread'>('unread');
  const [plan, setPlan] = useState<Plan | null>(null);
  const [sims, setSims] = useState<Record<number, Sim>>({});
  const [items, setItems] = useState<Record<string, ItemView | 'none' | 'loading'>>({});
  const [wallet, setWallet] = useState<string | null>(null);
  const [hasWallet, setHasWallet] = useState<boolean | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [note, setNote] = useState<{ ok: boolean; text: string } | null>(null);
  const [composer, setComposer] = useState<{ slot: number; modules: ModuleDraft[]; royaltyBps: number } | null>(null);

  useEffect(() => {
    const v = load(); setDraft(v.draft); setRun(v.run); setLoaded(true);
    if (v.run && v.run.phase !== 'done') setStep(4);
    setHasWallet(Boolean(window.solana));
    if (window.solana?.publicKey) setWallet(window.solana.publicKey.toBase58());
    fetch('/api/v1/launch/config').then((r) => (r.ok ? r.json() : null)).then((c: Config | null) => setCfg(c)).catch(() => setCfg(null));
  }, []);
  useEffect(() => { if (loaded) save({ draft, run }); }, [draft, run, loaded]);

  const set = (p: Partial<Draft>) => setDraft((d) => ({ ...d, ...p }));
  const upd = (i: number, p: Partial<SlotDraft>) => set({ slots: draft.slots.map((s, j) => (j === i ? { ...s, ...p } : s)) });
  const b = useMemo(() => budget(draft.slots, draft.kit), [draft.slots, draft.kit]);
  const bounds: Readonly<RuleBounds> = useMemo(() => {
    if (!cfg || cfg === 'unread') return RULE_BOUNDS;
    return Object.fromEntries(Object.entries(cfg.ruleBounds).map(([k, v]) => [k, Number(v)])) as unknown as RuleBounds;
  }, [cfg]);
  const minQ = cfg && cfg !== 'unread' ? BigInt(cfg.minVirtualQuote) : MIN_VIRTUAL_QUOTE;
  const maxQ = cfg && cfg !== 'unread' ? BigInt(cfg.maxVirtualQuote) : MAX_VIRTUAL_QUOTE;
  const maxFee = cfg && cfg !== 'unread' ? cfg.maxCreatorFeeBps : 200;
  const virtualQuote = toLamports(draft.virtualSol);
  const quoteOk = virtualQuote !== null && virtualQuote >= minQ && virtualQuote <= maxQ;
  const tokenOk = draft.name.trim().length > 0 && draft.name.length <= 32 && /^[^\s]{1,10}$/.test(draft.symbol) && quoteOk && draft.creatorFeeBps <= maxFee;
  const rulesProblem = draft.kit ? checkLaunchRules(draft.rules, draft.creatorFeeBps, bounds) : null;
  const problems = [...b.problems, ...(rulesProblem ? [rulesProblem] : [])];
  const mint = run ? Keypair.fromSecretKey(Uint8Array.from(run.mint)) : null;

  // The item named in each slot, read from the chain, for its sentence and manifest.
  useEffect(() => {
    for (const s of draft.slots) {
      const k = s.launchItem.trim();
      if (!/^[1-9A-HJ-NP-Za-km-z]{32,44}$/.test(k) || items[k]) continue;
      setItems((x) => ({ ...x, [k]: 'loading' }));
      fetch(`/api/v1/explorer/address/${k}`).then((r) => r.json()).then((a: { decoded?: { type?: string; data?: Record<string, unknown> } }) => {
        const d = a.decoded?.type === 'Item' ? a.decoded.data : null;
        setItems((x) => ({ ...x, [k]: d ? { templateId: Number(d.templateId), params: (d.params as unknown[]).map(Number), manifest: d.manifest as Record<string, unknown>, royaltyBps: Number(d.royaltyBps), level: Number(d.level) } : 'none' }));
      }).catch(() => setItems((x) => ({ ...x, [k]: 'none' })));
    }
  }, [draft.slots, items]);

  async function connect() {
    if (!window.solana) return;
    try { const r = await window.solana.connect(); setWallet(r.publicKey.toBase58()); } catch (e) { setNote({ ok: false, text: e instanceof Error ? e.message : String(e) }); }
  }

  function body(phase: 'prepare' | 'launch', m: Keypair) {
    return {
      owner: wallet, mint: m.publicKey.toBase58(), name: draft.name, symbol: draft.symbol, uri: draft.uri, virtualQuote: virtualQuote?.toString(),
      creatorFeeBps: draft.creatorFeeBps, kit: draft.kit, rules: draft.kit ? draft.rules : undefined, phase,
      slots: draft.slots.map((s) => ({ kind: s.kind, rule: s.rule, maxCutBps: s.maxCutBps, noticeSecs: s.noticeSecs, templateId: s.templateId, launchItem: s.launchItem.trim(), targets: s.targets })),
    };
  }

  async function preparePhase(phase: 'prepare' | 'launch') {
    if (!wallet) return;
    setBusy(phase === 'prepare' ? 'Preparing the first transactions' : 'Preparing the launch transactions'); setNote(null); setSims({});
    try {
      // A fresh mint for a new launch; a resumed launch keeps its mint, and the steps already sent are skipped by label.
      const m = run ? mint! : Keypair.generate();
      const r = await fetch('/api/v1/launch/plan', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body(phase, m)) });
      const j = await r.json() as Partial<Plan> & { error?: string };
      if (!r.ok || !j.transactions || !j.steps) throw new Error(j.error ?? `HTTP ${r.status}`);
      setPlan({ phase, transactions: j.transactions, steps: j.steps, launchFeeLamports: j.launchFeeLamports ?? null });
      if (!run) setRun({ mint: Array.from(m.secretKey), phase: 'prepare', sent: [] });
    } catch (e) { setNote({ ok: false, text: e instanceof Error ? e.message : String(e) }); } finally { setBusy(null); }
  }

  const done = new Set(run?.sent.map((s) => s.label) ?? []);
  const next = plan ? plan.transactions.findIndex((t) => !done.has(`${plan.phase}:${t.label}`)) : -1;

  async function simulateAndSign(i: number) {
    const w = window.solana;
    if (!plan || !mint || !w?.signAllTransactions) { setNote({ ok: false, text: 'This wallet cannot sign transactions here.' }); return; }
    const tx = plan.transactions[i]!;
    setBusy(`Simulating "${tx.label}"`); setNote(null);
    try {
      const r = await fetch('/api/v1/simulate', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ transaction: tx.transaction }) });
      const sim = await r.json() as Sim & { error?: string };
      if (!r.ok) throw new Error(sim.error ?? `HTTP ${r.status}`);
      setSims((x) => ({ ...x, [i]: sim }));
      if (!sim.ok) { setNote({ ok: false, text: `"${tx.label}" would fail: ${sim.failure?.name ?? ''} ${sim.failure?.message ?? ''}`.trim() }); return; }
      setBusy(`Signing "${tx.label}"`);
      const wallet2 = { signAllTransactions: <T extends VersionedTransaction>(txs: T[]) => w.signAllTransactions!(txs) };
      const [sig] = await signAndSend(wallet2, [tx], { mint }, () => undefined);
      const sent = [...(run?.sent ?? []), { label: `${plan.phase}:${tx.label}`, sig: sig! }];
      const last = i === plan.transactions.length - 1;
      setRun({ mint: run!.mint, phase: last && plan.phase === 'launch' ? 'done' : last ? 'launch' : plan.phase, sent });
      if (last && plan.phase === 'prepare') setPlan(null);
    } catch (e) { setNote({ ok: false, text: e instanceof Error ? e.message : String(e) }); } finally { setBusy(null); }
  }

  async function createComposite() {
    const w = window.solana;
    if (!composer || !wallet || !w?.signAllTransactions) return;
    setBusy('Creating the composite item'); setNote(null);
    try {
      const r = await fetch('/api/v1/composite/prepare', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ owner: wallet, modules: composer.modules, royaltyBps: composer.royaltyBps }) });
      const j = await r.json() as { transactions?: PreparedTx[]; error?: string };
      if (!r.ok || !j.transactions) throw new Error(j.error ?? `HTTP ${r.status}`);
      const wallet2 = { signAllTransactions: <T extends VersionedTransaction>(txs: T[]) => w.signAllTransactions!(txs) };
      const [sig] = await signAndSend(wallet2, j.transactions, {}, () => undefined);
      // The new item's address, from the transaction's own ItemCreated event.
      const t = await fetch(`/api/v1/explorer/tx/${sig}`).then((x) => x.json()) as { events?: { name: string; data: Record<string, unknown> }[] };
      const item = t.events?.find((e) => e.name === 'ItemCreated')?.data.item;
      if (typeof item === 'string') upd(composer.slot, { launchItem: item, templateId: 41 });
      setNote({ ok: true, text: typeof item === 'string' ? `Composite item ${item} created and named as slot ${composer.slot + (draft.kit ? 1 : 0)}'s launch item.` : `Sent ${sig}; read the item address from the transaction page.` });
      setComposer(null);
    } catch (e) { setNote({ ok: false, text: e instanceof Error ? e.message : String(e) }); } finally { setBusy(null); }
  }

  function reset() { setRun(null); setPlan(null); setSims({}); setNote(null); setStep(0); setDraft(EMPTY); }

  const slotNo = (i: number) => i + (draft.kit ? 1 : 0);
  const networkFee = plan ? plan.steps.reduce((n, s) => n + s.feeLamports, 0) : null;
  const launchFee = plan?.launchFeeLamports ?? (cfg && cfg !== 'unread' ? cfg.launchFeeLamports : null);

  return (
    <div className="grid cols-main launch-grid">
      <section className="panel">
        <div className="panel-head">
          <h2>Step {step + 1} of {STEPS.length}: {STEPS[step]}</h2>
          <div className="launch-chips">{STEPS.map((s, i) => <button key={s} type="button" className={`chip ${i === step ? 'accent' : ''}`} onClick={() => (i < 4 || run) && setStep(i)} disabled={Boolean(run) && i < 4}>{s}</button>)}</div>
        </div>
        <div className="panel-body launch-body">
          {run && step < 4 ? <p className="reason">A launch is in progress for {mint?.publicKey.toBase58()}; the draft is fixed until it finishes or you start over.</p> : null}
          {step === 0 && (
            <>
              <label className="field">Name<input value={draft.name} maxLength={32} onChange={(e) => set({ name: e.target.value })} placeholder="Shown on the board and the token page" /></label>
              <label className="field">Ticker<input value={draft.symbol} maxLength={10} onChange={(e) => set({ symbol: e.target.value.replace(/\s/g, '') })} placeholder="1 to 10 characters, no spaces" /></label>
              <label className="field">Metadata URI<input value={draft.uri} maxLength={200} onChange={(e) => set({ uri: e.target.value })} placeholder="Optional: where the image and description live" /></label>
              <label className="field">Virtual SOL reserve<input value={draft.virtualSol} inputMode="decimal" onChange={(e) => set({ virtualSol: e.target.value })} placeholder={`${solOf(minQ)} to ${solOf(maxQ)}: sets the opening price`} /></label>
              <label className="field">Creator fee on trades
                <select value={draft.creatorFeeBps} onChange={(e) => set({ creatorFeeBps: Number(e.target.value) })}>{CREATOR_FEE_CHOICES_BPS.filter((c) => c <= maxFee).map((c) => <option key={c} value={c}>{pct(c)}</option>)}</select>
              </label>
              <p className="muted launch-note">The creator fee is taken out of each trade on the launch pool, never added on top of it; the trader sees it in the quote before signing.</p>
            </>
          )}
          {step === 1 && (
            <>
              <label className="launch-check"><input type="checkbox" checked={draft.kit} onChange={(e) => set({ kit: e.target.checked })} /> Kit rules: holder rewards, burn, max wallet, locks</label>
              <p className="muted launch-note">Any kit rule puts the kit in slot 0, locked, using {32} of the {64} bytes of holder memory. {draft.kit ? '32 bytes are left for items.' : 'Without it all 64 bytes are left for items.'}</p>
              {draft.kit ? (
                <div className="launch-rules">
                  {([['holderFeeBuyBps', 'Holder rewards on buys', HOLDER_FEE_CHOICES_BPS, pct], ['holderFeeSellBps', 'Holder rewards on sells', HOLDER_FEE_CHOICES_BPS, pct], ['burnBuyBps', 'Burn on buys', BURN_CHOICES_BPS, pct], ['burnSellBps', 'Burn on sells', BURN_CHOICES_BPS, pct],
                    ['maxWalletBps', 'Max wallet', MAX_WALLET_CHOICES_BPS, (v: number) => (v ? pct(v) : 'off')], ['creatorLockDays', 'Creator wallet lock', CREATOR_LOCK_CHOICES_DAYS, (v: number) => (v ? `${v} days` : 'off')],
                    ['earlyWindowSecs', 'Early-buyer window', EARLY_WINDOW_CHOICES_SECS, (v: number) => (v ? `${v} s` : 'off')], ['earlyLockSecs', 'Early buys unlock after', [0, ...EARLY_LOCK_CHOICES_SECS], (v: number) => (v ? `${v / 60} min` : 'off')]] as const).map(([k, label, choices, fmt]) => (
                    <label className="field" key={k}>{label}
                      <select value={draft.rules[k]} onChange={(e) => set({ rules: { ...draft.rules, [k]: Number(e.target.value) } })}>{(choices as readonly number[]).map((c) => <option key={c} value={c}>{fmt(c)}</option>)}</select>
                    </label>
                  ))}
                </div>
              ) : null}
              {rulesProblem ? <p className="reason" role="alert">{rulesProblem}</p> : null}
              <div className="sep" />
              <div className="launch-option">
                <div><strong>Companion launch</strong><div className="muted launch-note">A companion makes a program the creator: its fees are bought back and shared, and its war share (war_bps of every fee claim) funds the war chest. The API does not prepare companion slot launches yet, so this launch funds its chest from items, partners and donations.</div></div>
                <span className="chip">not offered here</span>
              </div>
            </>
          )}
          {step === 2 && (
            <>
              {draft.slots.length === 0 ? <p className="muted launch-note">No item slots yet. A token can launch with none and still trade; it just has nothing to swap in later.</p> : null}
              {draft.slots.map((s, i) => {
                const fits = TEMPLATES.filter((t) => t.kind === s.kind || t.id === 41);
                const t = TEMPLATES.find((x) => x.id === s.templateId);
                const iv = items[s.launchItem.trim()];
                return (
                  <div key={i} className="launch-slot">
                    <div className="launch-slot-head"><strong>Slot {slotNo(i)}</strong><span className="faint">{slotBytes(s)} bytes of holder memory</span><button className="btn sm" type="button" onClick={() => set({ slots: draft.slots.filter((_, j) => j !== i) })}>Remove</button></div>
                    <div className="launch-slot-grid">
                      <label className="field">Kind<select value={s.kind} onChange={(e) => upd(i, { kind: e.target.value as SlotKind, templateId: null })}>{KINDS.map((k) => <option key={k}>{k}</option>)}</select></label>
                      <label className="field">Equip rule<select value={s.rule} onChange={(e) => upd(i, { rule: e.target.value as EquipRule })}>{RULES.map((r) => <option key={r} value={r}>{r}</option>)}</select></label>
                      <label className="field">Max cut (bps)<input type="number" min={0} max={10000} value={s.maxCutBps} disabled={s.kind === 'pool' || s.kind === 'war'} onChange={(e) => upd(i, { maxCutBps: Number(e.target.value) })} /></label>
                      <label className="field">Notice before an equip (s)<input type="number" min={0} value={s.noticeSecs} onChange={(e) => upd(i, { noticeSecs: Number(e.target.value) })} /></label>
                    </div>
                    <div className="faint">{RULE_TEXT[s.rule]}. {s.kind === 'pool' || s.kind === 'war' ? 'Pool and War slots cut only on the pool side; the token-side bound is 0.' : `Bound: an item here may cut at most ${pct(s.maxCutBps)} of a transfer.`}</div>
                    <div className="launch-picker" role="radiogroup" aria-label={`Template for slot ${slotNo(i)}`}>
                      {fits.map((x) => (
                        <button type="button" key={x.id} role="radio" aria-checked={s.templateId === x.id} className={`launch-tpl ${s.templateId === x.id ? 'on' : ''}`} onClick={() => upd(i, { templateId: s.templateId === x.id ? null : x.id })}>
                          <span className="launch-tpl-name">{x.name}</span>
                          <span className="faint">{x.callbacks.join(', ')}</span>
                          <span className="faint">{x.dataBytes ? `${x.dataBytes + 1} bytes` : 'no holder memory'}{answersTouch(x.id) ? ', answers touches' : ''}{mayBurn(x.id) ? ', may burn' : ''}</span>
                        </button>
                      ))}
                    </div>
                    {t ? (
                      <dl className="kv launch-kv">
                        <dt>Targets</dt><dd>{t.targets}</dd>
                        <dt>Fields</dt><dd>{t.fields.length ? t.fields.map((f) => `${f.name} (${f.floor} to ${f.ceiling})`).join('; ') : 'none'}</dd>
                        <dt>Spec</dt><dd>{t.spec}</dd>
                      </dl>
                    ) : null}
                    <label className="field">Launch item (address)<input value={s.launchItem} onChange={(e) => upd(i, { launchItem: e.target.value })} placeholder="An item you hold, equipped at launch; empty to leave the slot open" /></label>
                    {iv === 'loading' ? <div className="faint">Reading the item</div> : iv === 'none' ? <div className="reason">No item at that address on this cluster.</div> : iv ? (
                      <div className="launch-item">
                        <div><strong>{TEMPLATES.find((x) => x.id === iv.templateId)?.name ?? `template ${iv.templateId}`}</strong> level {iv.level}, royalty {pct(iv.royaltyBps)}</div>
                        <div className="muted">{itemSentence(iv.templateId, iv.params)}</div>
                        <div className="faint">Manifest: {Object.entries(iv.manifest).map(([k, v]) => `${k} ${String(v)}`).join(', ')}</div>
                        {iv.templateId !== s.templateId && s.templateId !== null ? <div className="reason">The item is not the template chosen above.</div> : null}
                      </div>
                    ) : null}
                    {['fee', 'reward', 'defense', 'relation', 'pool'].includes(s.kind) ? <button className="btn sm" type="button" onClick={() => setComposer({ slot: i, modules: [], royaltyBps: 0 })}>Build a composite item for this slot</button> : null}
                  </div>
                );
              })}
              <div><button className="btn" type="button" onClick={() => set({ slots: [...draft.slots, { kind: 'pool', rule: 'vote', maxCutBps: 0, noticeSecs: 0, templateId: null, launchItem: '', targets: [] }] })} disabled={b.slots >= MAX_SLOTS}>Add a slot</button>{b.slots >= MAX_SLOTS ? <div className="reason">MAX_SLOTS is {MAX_SLOTS}, the kit&apos;s slot included.</div> : null}</div>
              {composer ? <Composer c={composer} kind={draft.slots[composer.slot]?.kind} onChange={setComposer} onCreate={createComposite} canSign={Boolean(wallet)} busy={busy !== null} /> : null}
            </>
          )}
          {step === 3 && (
            <>
              <dl className="kv launch-kv">
                <dt>Token</dt><dd>{draft.name || '-'} ({draft.symbol || '-'})</dd>
                <dt>Virtual reserve</dt><dd>{draft.virtualSol || '-'} SOL</dd>
                <dt>Creator fee</dt><dd>{pct(draft.creatorFeeBps)} of each launch-pool trade, inside the trade</dd>
                <dt>Kit</dt><dd>{draft.kit ? `slot 0, locked: rewards ${pct(draft.rules.holderFeeBuyBps)} / ${pct(draft.rules.holderFeeSellBps)}, burn ${pct(draft.rules.burnBuyBps)} / ${pct(draft.rules.burnSellBps)}, max wallet ${draft.rules.maxWalletBps ? pct(draft.rules.maxWalletBps) : 'off'}` : 'none'}</dd>
                <dt>Item slots</dt><dd>{draft.slots.length ? draft.slots.map((s, i) => <div key={i}>Slot {slotNo(i)}: {s.kind}, {s.rule}, {s.kind === 'pool' || s.kind === 'war' ? 'pool side only' : `up to ${pct(s.maxCutBps)}`}, notice {s.noticeSecs} s{s.templateId ? `, ${TEMPLATES.find((t) => t.id === s.templateId)?.name}` : ''}{s.launchItem ? `, item ${s.launchItem.slice(0, 4)}...${s.launchItem.slice(-4)}` : ''}</div>) : 'none'}</dd>
                <dt>Fixed at launch</dt><dd className="muted">Kinds, bounds, equip rules, notices and holder memory ranges. Nobody can change these later; only the items change, by each slot&apos;s rule.</dd>
              </dl>
              <p className="muted launch-note">Next, the API prepares the transactions and the Launch step lists each one decoded from the exact bytes your wallet will sign, so you can compare them with this summary before signing anything.</p>
            </>
          )}
          {step === 4 && (
            <Walk plan={plan} run={run} sims={sims} next={next} busy={busy} mint={mint?.publicKey.toBase58() ?? null}
              onPrepare={() => preparePhase(run && run.phase === 'launch' ? 'launch' : 'prepare')} onSign={simulateAndSign} onReset={reset} canSign={Boolean(wallet)} canPrepare={problems.length === 0 && tokenOk} />
          )}
          {problems.length && step < 4 ? <div className="reason" role="alert">{problems.join(' ')}</div> : null}
          <div className="actions">
            <button className="btn" type="button" disabled={step === 0} onClick={() => setStep(step - 1)}>Back</button>
            <span className="right">
              {!wallet ? <button className="btn" type="button" onClick={connect} disabled={hasWallet === false}>Connect a wallet</button> : <span className="faint">{wallet.slice(0, 4)}...{wallet.slice(-4)}</span>}
              {step < 4 ? <button className="btn primary" type="button" disabled={(step === 0 && !tokenOk) || (step >= 1 && problems.length > 0)} onClick={() => setStep(step + 1)}>Next</button> : null}
              {step === 0 && !tokenOk ? <div className="reason">Name 1 to 32 characters, ticker 1 to 10 without spaces, virtual reserve {solOf(minQ)} to {solOf(maxQ)}.</div> : null}
              {!wallet && hasWallet === false ? <div className="reason">No Solana wallet in this browser: install one, then reload.</div> : null}
            </span>
          </div>
          {busy ? <div className="launch-status" role="status">{busy}</div> : null}
          {note ? <div className={`launch-status ${note.ok ? '' : 'bad'}`} role="status">{note.text}</div> : null}
        </div>
      </section>
      <aside className="grid">
        <section className="panel">
          <div className="panel-head"><h2>Limits</h2><span className="meta">checked as you go</span></div>
          <div className="panel-body">
            <dl className="kv launch-kv">
              <dt>Slots</dt><dd>{b.slots} of {MAX_SLOTS}</dd>
              <dt>Holder memory</dt><dd>{b.bytesUsed} of {b.bytesFree} bytes</dd>
              <dt>Cutting slots</dt><dd>{b.cutting} of {b.cuttingMax}</dd>
              <dt>Packet</dt><dd>{PACKET.toLocaleString('en-US')} bytes per transaction</dd>
            </dl>
            <div className="sep" />
            <div className="faint">Measured with the token&apos;s lookup table (docs: Limits): {MEASURED.map((m) => `${m.path} ${m.bytes} bytes, ${m.cu.toLocaleString('en-US')} CU`).join('; ')}.</div>
          </div>
        </section>
        <section className="panel">
          <div className="panel-head"><h2>Fees before signing</h2></div>
          <div className="panel-body">
            <dl className="kv launch-kv">
              <dt>Launch fee</dt><dd>{launchFee ? `${solOf(launchFee)}, from the launchpad config` : cfg === 'unread' ? 'reading' : 'not readable: the launchpad config is not on this cluster'}</dd>
              <dt>Network fees</dt><dd>{networkFee !== null ? `${solOf(networkFee)} for ${plan!.steps.length} transactions (5,000 lamports per signature)` : 'shown once prepared'}</dd>
              <dt>Account rent</dt><dd>Deposits for the accounts made (slot mint, pool, war state); your wallet shows each before you approve.</dd>
              <dt>On trades</dt><dd>{pct(draft.creatorFeeBps)} creator fee{draft.kit ? `, ${pct(draft.rules.holderFeeBuyBps)} / ${pct(draft.rules.holderFeeSellBps)} holder rewards, ${pct(draft.rules.burnBuyBps)} / ${pct(draft.rules.burnSellBps)} burn` : ''}, all inside each trade{cfg && cfg !== 'unread' ? `; LP fee ${pct(cfg.lpFeeBps)}` : ''}.</dd>
            </dl>
          </div>
        </section>
        <section className="panel">
          <div className="panel-head"><h2>Near graduation</h2></div>
          <div className="panel-body muted">A buy larger than what the curve can still fill is cut to the exact remainder, so the buy that completes the curve goes through and graduation can follow. The buy form on the token page says when that happens.</div>
        </section>
      </aside>
    </div>
  );
}

function Walk({ plan, run, sims, next, busy, mint, onPrepare, onSign, onReset, canSign, canPrepare }: {
  plan: Plan | null; run: Run | null; sims: Record<number, Sim>; next: number; busy: string | null; mint: string | null;
  onPrepare: () => void; onSign: (i: number) => void; onReset: () => void; canSign: boolean; canPrepare: boolean;
}) {
  const sentOf = (label: string) => run?.sent.find((s) => s.label === label)?.sig;
  const outline = ['prepare_launch: the mint with its slots', 'equip_prepared: one per launch item', "The token's lookup table", 'create_prepared_launch: supply, pool, pool items registry', 'refresh_pool_registry: when a pool slot forwards to an item', 'init_war and init_raid_ledger: the war chest'];
  if (run?.phase === 'done') {
    return (
      <div className="launch-done">
        <p><strong>Launched.</strong> The token is {mint}.</p>
        <p><a className="btn primary" href={`/t/${mint}`}>Open the token page</a> <a className="btn" href={`/address/${mint}`}>Open it in the explorer</a></p>
        <ol className="launch-sent">{run.sent.map((s) => <li key={s.sig}>{s.label.replace(/^\w+:/, '')}: <a href={`/tx/${s.sig}`}>{s.sig.slice(0, 8)}...</a></li>)}</ol>
        <button className="btn" type="button" onClick={onReset}>Launch another</button>
      </div>
    );
  }
  return (
    <>
      {mint ? <p className="muted launch-note">Mint {mint}. Its key stays in this browser until the launch is done; reloading resumes here.</p> : null}
      {!plan ? (
        <>
          <div className="steps">{outline.map((t, i) => <div className="step" key={t}><span className="n">{i + 1}</span><span>{t}</span><span className="faint">{run?.phase === 'launch' && i < 2 ? 'sent' : 'waiting'}</span></div>)}</div>
          <div className="actions">
            <button className="btn primary" type="button" disabled={!canSign || !canPrepare || busy !== null} onClick={onPrepare}>{run?.phase === 'launch' ? 'Prepare the launch transactions' : 'Prepare the first transactions'}</button>
            {run ? <button className="btn" type="button" onClick={onReset}>Start over</button> : null}
          </div>
          {!canSign ? <p className="reason">Connect a wallet to prepare.</p> : null}
        </>
      ) : (
        <ol className="launch-walk">
          {plan.transactions.map((t, i) => {
            const st = plan.steps[i]!;
            const sig = sentOf(`${plan.phase}:${t.label}`);
            const sim = sims[i];
            return (
              <li key={t.label} className={`launch-tx ${sig ? 'sent' : i === next ? 'next' : ''}`}>
                <div className="launch-tx-head">
                  <strong>{i + 1}. {t.label}</strong>
                  {sig ? <a className="chip ok" href={`/tx/${sig}`}>sent {sig.slice(0, 8)}</a> : sim ? (sim.ok ? <span className="chip ok">simulated, {sim.units?.toLocaleString('en-US')} CU</span> : <span className="chip bad">would fail</span>) : <span className="chip">waiting</span>}
                </div>
                <div className="faint">{st.bytes.toLocaleString('en-US')} of {st.packet.toLocaleString('en-US')} bytes, {st.lookupTables} lookup table{st.lookupTables === 1 ? '' : 's'}, {st.signatures} signature{st.signatures === 1 ? '' : 's'}, {st.computeLimit ? `compute limit ${st.computeLimit.toLocaleString('en-US')}${st.simulatedNow ? ' (from a simulation)' : ''}` : ''}{t.extraSigners.includes('mint') ? ', the mint key signs too' : ''}</div>
                <ul className="launch-ixs">{st.instructions.filter((x) => x.program !== 'computeBudget').map((x, j) => <li key={j}><span className="launch-ix">{x.program ?? 'program'} {x.name ?? 'instruction'}</span>{x.args && Object.keys(x.args).length ? <span className="faint"> {argsText(x.args).slice(0, 400)}</span> : null}</li>)}</ul>
                {sim && !sim.ok ? <div className="reason">{sim.failure?.program ?? 'A program'} refused: {sim.failure?.name ? `${sim.failure.name}, ` : ''}{sim.failure?.message}</div> : null}
                {!sig && i === next ? <button className="btn primary sm" type="button" disabled={busy !== null || !canSign} onClick={() => onSign(i)}>Simulate, then sign</button> : null}
              </li>
            );
          })}
        </ol>
      )}
    </>
  );
}

function Composer({ c, kind, onChange, onCreate, canSign, busy }: { c: { slot: number; modules: ModuleDraft[]; royaltyBps: number }; kind?: SlotKind; onChange: (v: { slot: number; modules: ModuleDraft[]; royaltyBps: number } | null) => void; onCreate: () => void; canSign: boolean; busy: boolean }) {
  const chk = compositeProblems(c.modules, kind);
  const usable = TEMPLATES.filter((t) => ![9, 41, 42, 43, 44, 45].includes(t.id) && t.kind !== 'war' && t.kind !== 'locked');
  const setM = (i: number, p: Partial<ModuleDraft>) => onChange({ ...c, modules: c.modules.map((m, j) => (j === i ? { ...m, ...p } : m)) });
  return (
    <div className="launch-slot launch-composer">
      <div className="launch-slot-head"><strong>Composite item for slot {c.slot}</strong><span className="faint">{c.modules.length} of {MAX_MODULES} modules, {chk.bytes} bytes{chk.host ? `, runs in a ${chk.host} slot` : ''}</span><button className="btn sm" type="button" onClick={() => onChange(null)}>Close</button></div>
      {c.modules.map((m, i) => {
        const t = TEMPLATES.find((x) => x.id === m.templateId);
        return (
          <div key={i} className="launch-slot-grid">
            <label className="field">Module {i + 1}<select value={m.templateId} onChange={(e) => setM(i, { templateId: Number(e.target.value), params: [] })}>{usable.map((x) => <option key={x.id} value={x.id}>{x.name} ({x.kind})</option>)}</select></label>
            <label className="field">Params<input value={m.params.join(', ')} onChange={(e) => setM(i, { params: e.target.value.split(',').map((x) => x.trim()).filter(Boolean).map(Number) })} placeholder={t ? t.fields.map((f) => f.name).join(', ') || 'none' : ''} /></label>
            <label className="field">First target<input type="number" min={0} value={m.targetStart} onChange={(e) => setM(i, { targetStart: Number(e.target.value) })} /></label>
            <label className="field">Targets<input type="number" min={0} value={m.targetCount} onChange={(e) => setM(i, { targetCount: Number(e.target.value) })} /></label>
          </div>
        );
      })}
      <div className="actions">
        <button className="btn sm" type="button" disabled={c.modules.length >= MAX_MODULES} onClick={() => onChange({ ...c, modules: [...c.modules, { templateId: usable[0]!.id, params: [], targetStart: 0, targetCount: 0 }] })}>Add a module</button>
        <label className="field launch-royalty">Royalty (bps)<input type="number" min={0} max={10000} value={c.royaltyBps} onChange={(e) => onChange({ ...c, royaltyBps: Number(e.target.value) })} /></label>
      </div>
      {chk.problems.length ? <div className="reason" role="alert">{chk.problems.join(' ')}</div> : <div className="faint">These modules pass the armory&apos;s composition rules the page can check; each field&apos;s floor and ceiling are checked on chain when the item is created.</div>}
      <button className="btn primary sm" type="button" disabled={!canSign || busy || chk.problems.length > 0} onClick={onCreate}>Create the composite item</button>
    </div>
  );
}
