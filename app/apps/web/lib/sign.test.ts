// Changed by Hookwars: the launch signing flow adds the mint signature and sends stages in order.
import { describe, expect, it } from 'vitest';
import { Keypair, SystemProgram, TransactionMessage, VersionedTransaction } from '@solana/web3.js';
import type { PreparedTx } from '@hookwars/shared';
import { signAndSend, withExtraSigners } from './sign';

const payer = Keypair.generate(); const mint = Keypair.generate();
const prep = (label: string, signers: PreparedTx['extraSigners'], extra?: Keypair): PreparedTx => {
  const ix = SystemProgram.transfer({ fromPubkey: payer.publicKey, toPubkey: extra?.publicKey ?? Keypair.generate().publicKey, lamports: 1 });
  if (extra) ix.keys[1]!.isSigner = true;
  const tx = new VersionedTransaction(new TransactionMessage({ payerKey: payer.publicKey, recentBlockhash: '11111111111111111111111111111111', instructions: [ix] }).compileToV0Message());
  return { transaction: Buffer.from(tx.serialize()).toString('base64'), version: 'v0', stage: 0, label, extraSigners: signers };
};
const wallet = { signAllTransactions: async <T extends VersionedTransaction>(txs: T[]) => { txs.forEach((t) => t.sign([payer])); return txs; } };

describe('launch signing', () => {
  it('signs with the mint only where a stage asks for it', () => {
    const [a, b] = withExtraSigners([prep('prepare', ['mint'], mint), prep('equip', [])], { mint });
    expect(a!.signatures[1]!.some((x) => x !== 0)).toBe(true);
    expect(b!.signatures).toHaveLength(1);
    expect(() => withExtraSigners([prep('prepare', ['mint'], mint)], {})).toThrow(/mint key/);
  });
  it('sends every stage in order and stops at the first refusal', async () => {
    const seen: string[] = [];
    const ok = (async (_u: string, init: RequestInit) => { seen.push(String(init.body)); return new Response(JSON.stringify({ signature: `s${seen.length}` })); }) as unknown as typeof fetch;
    expect(await signAndSend(wallet, [prep('a', []), prep('b', [])], {}, () => {}, ok)).toEqual(['s1', 's2']);
    const bad = (async () => new Response(JSON.stringify({ error: 'refused' }), { status: 409 })) as unknown as typeof fetch;
    await expect(signAndSend(wallet, [prep('a', []), prep('b', [])], {}, () => {}, bad)).rejects.toThrow('a: refused');
  });
});
