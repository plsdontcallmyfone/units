'use client';
// Changed by Hookwars: new file. One form for every prepare route that is not a launch: connect the
// wallet, POST the fields with the wallet as owner to /api/v1/<route>/prepare, sign every returned
// transaction in one prompt and send each through /api/v1/submit (app audit A-2). The window.solana
// type is declared in app/launch/form.tsx.
import { useEffect, useState, type ReactNode } from 'react';
import type { VersionedTransaction } from '@solana/web3.js';
import type { PreparedTx } from '@hookwars/shared';
import { signAndSend } from '@/lib/sign';
import { bodyOf, type Field } from '@/lib/action-body';

export type { Field, FieldKind } from '@/lib/action-body';

export function Action({ route, title, what, fields = [], fixed = {}, cta, children, note }: { route: string; title: string; what?: ReactNode; fields?: Field[]; fixed?: Record<string, unknown>; cta?: string; children?: ReactNode; /** Changed by Hookwars (launch page): a line under the fields from what is typed, e.g. a buy's remainder quote. */ note?: (values: Record<string, string>, wallet: string | null) => ReactNode }) {
  const [values, setValues] = useState<Record<string, string>>({});
  const [wallet, setWallet] = useState<string | null>(null);
  const [hasWallet, setHasWallet] = useState<boolean | null>(null);
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<{ ok: boolean; text: string } | null>(null);
  const [sigs, setSigs] = useState<string[]>([]);
  useEffect(() => { setHasWallet(Boolean(window.solana)); if (window.solana?.publicKey) setWallet(window.solana.publicKey.toBase58()); }, []);

  async function connect() {
    if (!window.solana) return;
    try { const r = await window.solana.connect(); setWallet(r.publicKey.toBase58()); } catch (e) { setStatus({ ok: false, text: e instanceof Error ? e.message : String(e) }); }
  }

  async function run() {
    const w = window.solana;
    if (!wallet || !w) return;
    if (!w.signAllTransactions) { setStatus({ ok: false, text: 'This wallet cannot sign transactions here.' }); return; }
    const made = bodyOf(fields, values, fixed);
    if ('error' in made) { setStatus({ ok: false, text: made.error }); return; }
    setBusy(true); setStatus({ ok: true, text: 'Preparing...' }); setSigs([]);
    try {
      const r = await fetch(`/api/v1/${route}/prepare`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ ...made.body, owner: wallet }) });
      const b = await r.json() as { error?: string; transactions?: PreparedTx[] };
      if (!r.ok || !b.transactions) throw new Error(b.error ?? `HTTP ${r.status}`);
      setStatus({ ok: true, text: `Sign ${b.transactions.length === 1 ? 'the transaction' : `${b.transactions.length} transactions`} in your wallet.` });
      const signer = { signAllTransactions: <T extends VersionedTransaction>(txs: T[]) => w.signAllTransactions!(txs) };
      const out = await signAndSend(signer, b.transactions, {}, (_i, s) => setSigs((x) => [...x, s]));
      setStatus({ ok: true, text: `Done in ${out.length} ${out.length === 1 ? 'transaction' : 'transactions'}.` });
    } catch (e) {
      setStatus({ ok: false, text: e instanceof Error ? e.message : String(e) });
    } finally { setBusy(false); }
  }

  return (
    <div className="action">
      <div className="action-head"><b>{title}</b>{what ? <p className="muted">{what}</p> : null}</div>
      {children}
      {fields.length ? (
        <div className="action-fields">
          {fields.map((f) => (
            <label className="field" key={f.name}>{f.label}{f.optional ? <span className="faint"> (optional)</span> : null}
              {f.kind === 'bool' ? (
                <select value={values[f.name] ?? 'false'} onChange={(e) => setValues({ ...values, [f.name]: e.target.value })}><option value="true">yes</option><option value="false">no</option></select>
              ) : f.choices ? (
                <select value={values[f.name] ?? ''} onChange={(e) => setValues({ ...values, [f.name]: e.target.value })}><option value="">choose</option>{f.choices.map(([v, l]) => <option key={v} value={v}>{l}</option>)}</select>
              ) : (
                <input value={values[f.name] ?? ''} inputMode={f.kind === 'int' || f.kind === 'amount' || f.kind === 'sol' ? 'decimal' : 'text'} placeholder={f.hint ?? (f.kind === 'sol' ? 'SOL' : f.kind === 'key' ? 'address' : '')} onChange={(e) => setValues({ ...values, [f.name]: e.target.value })} />
              )}
            </label>
          ))}
        </div>
      ) : null}
      {note ? note(values, wallet) : null}
      <div className="action-row">
        {hasWallet === false ? <span className="muted">No Solana wallet in this browser.</span>
          : !wallet ? <button type="button" className="btn sm" onClick={connect}>Connect wallet</button>
          : <button type="button" className="btn sm primary" disabled={busy} onClick={run}>{busy ? 'Working...' : cta ?? title}</button>}
        {wallet ? <span className="faint">as {wallet.slice(0, 4)}...{wallet.slice(-4)}</span> : null}
      </div>
      {status ? <p className={status.ok ? 'muted' : 'bad'} role="status">{status.text}</p> : null}
      {sigs.length ? <ul className="action-sigs">{sigs.map((s) => <li key={s}><a href={`https://solscan.io/tx/${s}?cluster=devnet`} target="_blank" rel="noreferrer">{s.slice(0, 8)}...{s.slice(-8)}</a></li>)}</ul> : null}
    </div>
  );
}

/** A set of actions folded under one heading, so a page shows its reads first. */
export function Actions({ children }: { children: ReactNode }) {
  return <div className="act-grid">{children}</div>;
}
