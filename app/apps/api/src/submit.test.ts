// Changed by Hookwars: tests for the submit route's checks (app audit A-2: the site sends through the API).
import { describe, expect, it } from 'vitest';
import { Keypair, SystemProgram, TransactionMessage, VersionedTransaction } from '@solana/web3.js';
import { signedTransaction, submit } from './submit.ts';

const payer = Keypair.generate();
const tx = () => new VersionedTransaction(new TransactionMessage({
  payerKey: payer.publicKey, recentBlockhash: '11111111111111111111111111111111',
  instructions: [SystemProgram.transfer({ fromPubkey: payer.publicKey, toPubkey: Keypair.generate().publicKey, lamports: 1 })],
}).compileToV0Message());
const b64 = (t: VersionedTransaction) => Buffer.from(t.serialize()).toString('base64');

describe('submit', () => {
  it('refuses an unsigned transaction', () => {
    expect(() => signedTransaction({ transaction: b64(tx()) })).toThrow(/signature/);
  });
  it('refuses junk and oversize input', () => {
    expect(() => signedTransaction({ transaction: 'not base64!' })).toThrow(/base64/);
    expect(() => signedTransaction({ transaction: 'A'.repeat(2000) })).toThrow(/packet/);
    expect(() => signedTransaction({ transaction: 'AAAA' })).toThrow(/not a transaction/);
    expect(() => signedTransaction({})).toThrow();
  });
  it('sends a signed transaction and returns its signature', async () => {
    const t = tx(); t.sign([payer]);
    let sent = 0;
    const conn = {
      sendRawTransaction: async () => { sent++; return 'sig'; },
      getLatestBlockhash: async () => ({ blockhash: 'b', lastValidBlockHeight: 1 }),
      confirmTransaction: async () => ({ value: { err: null } }),
    } as never;
    expect(await submit(conn, { transaction: b64(t) })).toEqual({ signature: 'sig' });
    expect(sent).toBe(1);
  });
  it('reports a refusal as a sentence without the raw error', async () => {
    const t = tx(); t.sign([payer]);
    const conn = { sendRawTransaction: async () => { throw Object.assign(new Error('rpc http://secret'), { logs: ['Program log: Error Message: slot is locked.'] }); } } as never;
    await expect(submit(conn, { transaction: b64(t) })).rejects.toThrow('The programs refused this: Error Message: slot is locked.');
  });
});
