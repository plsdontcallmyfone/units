// Changed by Hookwars: signs a prepare's transactions in the browser (the mint key where a stage
// needs it, then the wallet) and sends them in order through the API's submit route, which holds
// the RPC (app audit A-2). The site never sees an RPC URL or a server key.
import { Keypair, VersionedTransaction } from '@solana/web3.js';
import type { PreparedTx } from '@hookwars/shared';

/** Base64 without Node's Buffer, which the browser bundle does not have. */
export const fromB64 = (s: string): Uint8Array => Uint8Array.from(atob(s), (c) => c.charCodeAt(0));
export const toB64 = (b: Uint8Array): string => btoa(Array.from(b, (x) => String.fromCharCode(x)).join(''));

export interface SigningWallet { signAllTransactions<T extends VersionedTransaction>(txs: T[]): Promise<T[]> }

/** Decodes the prepared transactions and adds the extra signatures a stage asks for. */
export function withExtraSigners(prepared: PreparedTx[], extra: { mint?: Keypair; config?: Keypair; randomness?: Keypair }): VersionedTransaction[] {
  return prepared.map((p) => {
    const tx = VersionedTransaction.deserialize(fromB64(p.transaction));
    for (const s of p.extraSigners ?? []) {
      const kp = extra[s];
      if (!kp) throw new Error(`"${p.label}" needs the ${s} key, which this page does not hold.`);
      tx.sign([kp]);
    }
    return tx;
  });
}

/** Signs every transaction with the wallet in one prompt, then sends each and waits for it before the next. */
export async function signAndSend(wallet: SigningWallet, prepared: PreparedTx[], extra: { mint?: Keypair; config?: Keypair; randomness?: Keypair }, progress: (i: number, sig: string) => void, post: typeof fetch = fetch): Promise<string[]> {
  const signed = await wallet.signAllTransactions(withExtraSigners(prepared, extra));
  const out: string[] = [];
  for (let i = 0; i < signed.length; i++) {
    const r = await post('/api/v1/submit', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ transaction: toB64(signed[i]!.serialize()) }) });
    const b = await r.json() as { signature?: string; error?: string };
    if (!r.ok || !b.signature) throw new Error(`${prepared[i]!.label}: ${b.error ?? `HTTP ${r.status}`}`);
    out.push(b.signature); progress(i, b.signature);
  }
  return out;
}
