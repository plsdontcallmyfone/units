'use client';
// Changed by Hookwars: new file (launch page polish). The token page's buy: the same prepare route
// (`buy/prepare` cuts a buy to the curve's exact remainder near graduation, fuzz audit 1 finding 1),
// and, as the amount is typed, the API's quote saying whether that cut applies and to how much.
import { useEffect, useState } from 'react';
import { Action } from './action';

const LAMPORTS = 1_000_000_000n;
function toLamports(s: string | undefined): bigint | null {
  const m = /^(\d+)(?:\.(\d{1,9}))?$/.exec((s ?? '').trim());
  return m ? BigInt(m[1]!) * LAMPORTS + BigInt((m[2] ?? '').padEnd(9, '0')) : null;
}
const sol = (l: string) => { const n = BigInt(l); const f = (n % LAMPORTS).toString().padStart(9, '0').replace(/0+$/, ''); return `${n / LAMPORTS}${f ? `.${f}` : ''} SOL`; };

function Quote({ mint, amount, owner }: { mint: string; amount: string | undefined; owner: string | null }) {
  const [q, setQ] = useState<{ amountIn: string; remainder: boolean } | { error: string } | null>(null);
  const lamports = toLamports(amount);
  useEffect(() => {
    setQ(null);
    if (!owner || lamports === null || lamports === 0n) return;
    const t = setTimeout(() => {
      fetch(`/api/v1/launch/buy-quote?mint=${mint}&owner=${owner}&amount=${lamports}`).then((r) => r.json()).then((b: { amountIn?: string; remainder?: boolean; error?: string }) => {
        setQ(b.amountIn !== undefined ? { amountIn: b.amountIn, remainder: Boolean(b.remainder) } : { error: b.error ?? 'No quote.' });
      }).catch(() => setQ({ error: 'The backend is not reachable.' }));
    }, 400);
    return () => clearTimeout(t);
  }, [mint, owner, lamports]);
  if (!owner || lamports === null) return null;
  if (!q) return <p className="faint">Checking the curve...</p>;
  if ('error' in q) return <p className="faint">{q.error}</p>;
  return q.remainder
    ? <p className="reason" role="status">The curve can fill only {sol(q.amountIn)} more: this buy is cut to that exact remainder so it completes the curve.</p>
    : <p className="faint">The whole {sol(q.amountIn)} goes to the curve.</p>;
}

export function BuyAction({ mint }: { mint: string }) {
  return (
    <Action route="buy" title="Buy" what="Buys this token for SOL on its pool. Fees are taken out of the trade, never added on top." fixed={{ mint }}
      fields={[{ name: 'amount', label: 'SOL in', kind: 'sol' }, { name: 'minOut', label: 'Least tokens out (base units)', kind: 'amount', optional: true }]}
      note={(v, w) => <Quote mint={mint} amount={v.amount} owner={w} />} />
  );
}
