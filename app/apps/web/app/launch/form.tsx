'use client';
// Changed by Hookwars: the form now makes the mint key, sends the mint and the virtual quote, and
// signs and sends the staged launch through the API (app/INTEGRATION.md, app audit A-2).
import { useEffect, useMemo, useState } from 'react';
import { Keypair, type VersionedTransaction } from '@solana/web3.js';
import { MAX_VIRTUAL_QUOTE, MIN_VIRTUAL_QUOTE, PARAMS, TEMPLATES, type EquipRule, type PreparedTx, type SlotKind } from '@hookwars/shared';
import { signAndSend } from '@/lib/sign';

type SlotDraft = { kind: SlotKind; rule: EquipRule; maxCutBps: number; noticeSecs: number; templateId: number | null; launchItem: string };

const MAX_SLOTS = PARAMS.find((p) => p.name === 'MAX_SLOTS')?.value ?? 4;
const MAX_CUTTING = PARAMS.find((p) => p.name === 'MAX_CUTTING_SLOTS')?.value ?? 3;
const KINDS: SlotKind[] = ['fee', 'reward', 'defense', 'relation', 'pool', 'war'];
const RULES: EquipRule[] = ['locked', 'vote', 'performance'];
const STEPS = ['Token', 'Rules', 'Slots', 'Review'];

declare global { interface Window { solana?: { isPhantom?: boolean; publicKey?: { toBase58(): string }; connect(): Promise<{ publicKey: { toBase58(): string } }>; signAllTransactions?<T extends VersionedTransaction>(txs: T[]): Promise<T[]> } } }

const LAMPORTS = 1_000_000_000n;
/** SOL as typed (up to 9 decimals) to lamports; null when it is not a number. */
function toLamports(s: string): bigint | null {
  const m = /^(\d+)(?:\.(\d{1,9}))?$/.exec(s.trim());
  return m ? BigInt(m[1]!) * LAMPORTS + BigInt((m[2] ?? '').padEnd(9, '0')) : null;
}
const sol = (l: bigint) => `${l / LAMPORTS}`;

export function LaunchForm() {
  const [step, setStep] = useState(0);
  const [name, setName] = useState('');
  const [symbol, setSymbol] = useState('');
  const [virtualSol, setVirtualSol] = useState('');
  const [prepared, setPrepared] = useState<{ txs: PreparedTx[]; mint: Keypair; body: Record<string, unknown> } | null>(null);
  const [sent, setSent] = useState<string[]>([]);
  const [kit, setKit] = useState(false);
  const [slots, setSlots] = useState<SlotDraft[]>([]);
  const [wallet, setWallet] = useState<string | null>(null);
  const [result, setResult] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [hasWallet, setHasWallet] = useState<boolean | null>(null);
  useEffect(() => { setHasWallet(Boolean(window.solana)); }, []);

  const bytesFree = kit ? 32 : 64;
  const bytesUsed = useMemo(() => slots.reduce((n, s) => {
    const t = TEMPLATES.find((x) => x.id === s.templateId);
    return n + (t && t.dataBytes > 0 ? t.dataBytes + 1 : 0);
  }, 0), [slots]);
  const cutting = slots.filter((s) => s.maxCutBps > 0 && s.kind !== 'pool' && s.kind !== 'war').length;
  const problems: string[] = [];
  if (slots.length + (kit ? 1 : 0) > MAX_SLOTS) problems.push(`At most ${MAX_SLOTS} slots, the kit's included (TooManySlots).`);
  if (bytesUsed > bytesFree) problems.push(`The items need ${bytesUsed} bytes of holder data; ${bytesFree} are left (HookDataOverflow).`);
  if (cutting > MAX_CUTTING - (kit ? 1 : 0)) problems.push(`Too many token-side cutting slots (TooManyCuttingSlots).`);
  if (slots.filter((s) => s.kind === 'war').length > 1) problems.push('One War slot at most.');
  const virtualQuote = toLamports(virtualSol);
  const quoteOk = virtualQuote !== null && virtualQuote >= MIN_VIRTUAL_QUOTE && virtualQuote <= MAX_VIRTUAL_QUOTE;
  const tokenOk = name.trim().length > 0 && name.length <= 32 && /^[^\s]{1,10}$/.test(symbol) && quoteOk;

  const addSlot = () => setSlots([...slots, { kind: 'pool', rule: 'vote', maxCutBps: 0, noticeSecs: 0, templateId: null, launchItem: '' }]);
  const upd = (i: number, p: Partial<SlotDraft>) => setSlots(slots.map((s, j) => (j === i ? { ...s, ...p } : s)));

  async function connect() {
    if (!window.solana) return;
    const r = await window.solana.connect();
    setWallet(r.publicKey.toBase58());
  }

  async function prepare() {
    if (!wallet) return;
    setBusy(true); setResult(null); setPrepared(null); setSent([]);
    try {
      // A fresh mint for every prepare; its key stays in this page and signs the stages that need it.
      const mint = Keypair.generate();
      const body = { owner: wallet, mint: mint.publicKey.toBase58(), name, symbol, virtualQuote: virtualQuote?.toString(), kit, slots };
      const r = await fetch('/api/v1/launch/prepare', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) });
      const b = await r.json() as { error?: string; transactions?: PreparedTx[] };
      if (r.ok && b.transactions) { setPrepared({ txs: b.transactions, mint, body }); setResult(`Prepared ${b.transactions.length} transactions for ${mint.publicKey.toBase58()}.`); }
      else setResult(b.error ?? `HTTP ${r.status}`);
    } catch (e) {
      setResult(e instanceof Error ? e.message : String(e));
    } finally { setBusy(false); }
  }

  async function launch() {
    const w = window.solana;
    if (!prepared || !w?.signAllTransactions) { setResult('This wallet cannot sign several transactions at once.'); return; }
    setBusy(true);
    try {
      const wallet2 = { signAllTransactions: <T extends VersionedTransaction>(txs: T[]) => w.signAllTransactions!(txs) };
      const first = await signAndSend(wallet2, prepared.txs, { mint: prepared.mint }, (i, s) => setSent((x) => [...x.slice(0, i), s]));
      // The second phase is built from the chain once the equips have landed (deposit slices, item registries).
      const r = await fetch('/api/v1/launch/prepare', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ ...prepared.body, phase: 'launch' }) });
      const b = await r.json() as { error?: string; transactions?: PreparedTx[] };
      if (!r.ok || !b.transactions) throw new Error(b.error ?? `HTTP ${r.status}`);
      const n = first.length;
      const second = await signAndSend(wallet2, b.transactions, { mint: prepared.mint }, (i, s) => setSent((x) => [...x.slice(0, n + i), s]));
      setResult(`Launched ${prepared.mint.publicKey.toBase58()} in ${n + second.length} transactions.`); setPrepared(null);
    } catch (e) {
      setResult(e instanceof Error ? e.message : String(e));
    } finally { setBusy(false); }
  }

  return (
    <div className="grid cols-main">
      <section className="panel">
        <div className="panel-head">
          <h2>Step {step + 1} of 4: {STEPS[step]}</h2>
          <div style={{ display: 'flex', gap: 6 }}>{STEPS.map((s, i) => <span key={s} className={`chip ${i === step ? 'accent' : ''}`}>{s}</span>)}</div>
        </div>
        <div className="panel-body" style={{ display: 'grid', gap: 14 }}>
          {step === 0 && (
            <>
              <label className="field">Name<input value={name} maxLength={32} onChange={(e) => setName(e.target.value)} placeholder="Shown on the board and the token page" /></label>
              <label className="field">Ticker<input value={symbol} maxLength={10} onChange={(e) => setSymbol(e.target.value.replace(/\s/g, ''))} placeholder="1 to 10 characters, no spaces" /></label>
              <label className="field">Virtual SOL reserve<input value={virtualSol} inputMode="decimal" onChange={(e) => setVirtualSol(e.target.value)} placeholder={`${sol(MIN_VIRTUAL_QUOTE)} to ${sol(MAX_VIRTUAL_QUOTE)} SOL: sets the opening price`} /></label>
            </>
          )}
          {step === 1 && (
            <>
              <label style={{ display: 'flex', gap: 10, alignItems: 'center' }}><input type="checkbox" checked={kit} onChange={(e) => setKit(e.target.checked)} /> Kit rules (holder rewards, burn, max wallet, locks)</label>
              <p className="muted" style={{ margin: 0 }}>Any kit module puts the kit in slot 0, locked, with 32 bytes of holder data. {kit ? '32 bytes are left for items.' : 'Without it all 64 bytes are left for items.'} The kit's rule values are chosen as on the upstream launch form.</p>
            </>
          )}
          {step === 2 && (
            <>
              {slots.length === 0 ? <p className="muted" style={{ margin: 0 }}>No item slots yet. A token can launch with none and still trade; it just has nothing to swap in later.</p> : null}
              {slots.map((s, i) => {
                const fits = TEMPLATES.filter((t) => t.kind === s.kind);
                return (
                  <div key={i} className="panel" style={{ padding: 12, display: 'grid', gap: 10, gridTemplateColumns: 'repeat(auto-fit, minmax(150px, 1fr))' }}>
                    <label className="field">Kind<select value={s.kind} onChange={(e) => upd(i, { kind: e.target.value as SlotKind, templateId: null })}>{KINDS.map((k) => <option key={k}>{k}</option>)}</select></label>
                    <label className="field">Equip rule<select value={s.rule} onChange={(e) => upd(i, { rule: e.target.value as EquipRule })}>{RULES.map((r) => <option key={r}>{r}</option>)}</select></label>
                    <label className="field">Max cut (bps)<input type="number" min={0} max={10000} value={s.maxCutBps} onChange={(e) => upd(i, { maxCutBps: Number(e.target.value) })} /></label>
                    <label className="field">Notice (s)<input type="number" min={0} value={s.noticeSecs} onChange={(e) => upd(i, { noticeSecs: Number(e.target.value) })} /></label>
                    <label className="field">Template<select value={s.templateId ?? ''} onChange={(e) => upd(i, { templateId: e.target.value ? Number(e.target.value) : null })}><option value="">none yet</option>{fits.map((t) => <option key={t.id} value={t.id}>{t.name}</option>)}</select></label>
                    <label className="field">Launch item<input value={s.launchItem} onChange={(e) => upd(i, { launchItem: e.target.value })} placeholder="item address" /></label>
                    <button className="btn" type="button" onClick={() => setSlots(slots.filter((_, j) => j !== i))}>Remove</button>
                  </div>
                );
              })}
              <div><button className="btn" type="button" onClick={addSlot} disabled={slots.length + (kit ? 1 : 0) >= MAX_SLOTS}>Add a slot</button>{slots.length + (kit ? 1 : 0) >= MAX_SLOTS ? <div className="reason">MAX_SLOTS is {MAX_SLOTS}, the kit's slot included.</div> : null}</div>
            </>
          )}
          {step === 3 && (
            <>
              <dl className="kv">
                <dt>Token</dt><dd>{name || '-'} ({symbol || '-'})</dd>
                <dt>Virtual reserve</dt><dd>{virtualSol || '-'} SOL</dd>
                <dt>Kit</dt><dd>{kit ? 'slot 0, locked, 32 bytes' : 'none'}</dd>
                <dt>Item slots</dt><dd>{slots.length ? slots.map((s, i) => <div key={i}>{s.kind}, {s.rule}, up to {s.maxCutBps / 100}%{s.templateId ? `, ${TEMPLATES.find((t) => t.id === s.templateId)?.name}` : ''}</div>) : 'none'}</dd>
                <dt>Fixed at launch</dt><dd className="muted">Kinds, bounds, equip rules, notices and data ranges. The creator can't change these.</dd>
              </dl>
              <div className="sep" />
              <div className="steps">
                {(prepared ? [...prepared.txs.map((t) => t.label), "The mint's lookup table", 'Launch', 'War chest'] : ['prepare_launch: the mint with its slots', 'equip_prepared: one per launch item', "The mint's lookup table", 'create_prepared_launch: supply, pool, pool items registry', 'init_war, init_raid_ledger']).map((t, i) => (
                  <div className="step" key={`${i}${t}`}><span className="n">{i + 1}</span><span>{t}</span><span className="faint">{sent[i] ? `sent ${sent[i]!.slice(0, 8)}` : 'waiting'}</span></div>
                ))}
              </div>
            </>
          )}
          {problems.length ? <div className="reason" role="alert">{problems.join(' ')}</div> : null}
          <div className="actions">
            <button className="btn" type="button" disabled={step === 0} onClick={() => setStep(step - 1)}>Back</button>
            {step < 3 ? (
              <span className="right">
                <button className="btn primary" type="button" disabled={step === 0 && !tokenOk} onClick={() => setStep(step + 1)}>Next</button>
                {step === 0 && !tokenOk ? <div className="reason">Name 1 to 32 characters, ticker 1 to 10 without spaces, virtual reserve {sol(MIN_VIRTUAL_QUOTE)} to {sol(MAX_VIRTUAL_QUOTE)} SOL.</div> : null}
              </span>
            ) : (
              <span className="right">
                {wallet && prepared ? <button className="btn primary" type="button" disabled={busy} onClick={launch}>{busy ? 'Sending' : 'Sign and send the launch'}</button>
                  : wallet ? <button className="btn primary" type="button" disabled={busy || problems.length > 0} onClick={prepare}>{busy ? 'Preparing' : 'Prepare the launch'}</button>
                  : <button className="btn primary" type="button" onClick={connect} disabled={hasWallet === false}>Connect a wallet</button>}
                {!wallet && hasWallet === false ? <div className="reason">No Solana wallet in this browser: install one, then reload.</div> : null}
              </span>
            )}
          </div>
          {result ? <div className="panel" style={{ padding: 12 }} role="status">{result}</div> : null}
        </div>
      </section>
      <aside className="grid">
        <section className="panel">
          <div className="panel-head"><h2>Budget</h2><span className="meta">checked as you go</span></div>
          <div className="panel-body">
            <dl className="kv">
              <dt>Slots</dt><dd>{slots.length + (kit ? 1 : 0)} of {MAX_SLOTS}</dd>
              <dt>Holder data</dt><dd>{bytesUsed} of {bytesFree} bytes</dd>
              <dt>Cutting slots</dt><dd>{cutting} of {MAX_CUTTING - (kit ? 1 : 0)}</dd>
            </dl>
          </div>
        </section>
        <section className="panel">
          <div className="panel-head"><h2>War chest funding</h2></div>
          <div className="panel-body muted">A chest is funded by launching through a companion with war_bps of every creator fee claim, and by items, partners and donations. A plain launch funds its chest only from the last three.</div>
        </section>
      </aside>
    </div>
  );
}
