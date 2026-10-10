// Runtime gap R-5: `spend` adds one call level to a trade route. A simulation that hits the limit is
// reported as such, and the loop refuses that token's trades on that side for a day instead of
// failing every tick (a live measurement on devnet is still to come; see the README).
import { describe, expect, it } from 'vitest';
import { PublicKey, TransactionInstruction } from '@solana/web3.js';
import { hookwars, token } from '@hookwars/sdk';
import { tick } from './loop.ts';
import { replay } from './models/stub.ts';
import { DEEP_ROUTE_SECS } from './plan.ts';
import { invokeDepth, isDepthLimit, RecordingSender } from './sender.ts';
import { kit } from './testkit.ts';

const TOKEN_MINT = new PublicKey(new Uint8Array(32).fill(12));
const OTHER = new PublicKey(new Uint8Array(32).fill(13));
const LOGS = ['Program A invoke [1]', 'Program B invoke [2]', 'Program C invoke [3]', 'Program D invoke [4]', 'Program E invoke [5]', 'Program E failed: Cross-program invocation call depth too deep'];

class DeepSender extends RecordingSender {
  override async submit(...a: Parameters<RecordingSender['submit']>) {
    await super.submit(...a);
    return { sent: false, signature: null, unitsConsumed: 1, logs: LOGS, error: 'call depth limit', bytes: 1, depthLimited: true };
  }
}

describe('call depth (R-5)', () => {
  it('reads the deepest invoke level and recognises the depth error', () => {
    expect(invokeDepth(LOGS)).toBe(5);
    expect(isDepthLimit({ InstructionError: [1, 'CallDepth'] }, [])).toBe(true);
    expect(isDepthLimit(null, LOGS)).toBe(true);
    expect(isDepthLimit({ InstructionError: [1, { Custom: 6000 }] }, ['Program A invoke [1]'])).toBe(false);
  });

  it('a buy that hits the limit is remembered, and the next one is refused before any prepare', async () => {
    const buy = { type: 'trade', side: 'buy', mint: TOKEN_MINT.toBase58(), amountIn: '100000000', minOut: '995000', reason: 'r' };
    const k = kit(replay([{ actions: [buy] }, { actions: [buy] }]), {}, '{"v":1}');
    k.chain.mints.set(TOKEN_MINT.toBase58(), { creator: OTHER.toBase58(), itemAuthors: [] });
    k.router.quotes.set(TOKEN_MINT.toBase58(), 1_000_000n);
    const swap = new TransactionInstruction({ programId: hookwars.SWAP_ID, keys: [{ pubkey: k.vault, isSigner: true, isWritable: true }], data: Buffer.from([9]) });
    k.router.builders.set('buy/prepare', (b) => [token.createHolding(new PublicKey(b.owner as string), TOKEN_MINT, new PublicKey(b.owner as string)), swap]);
    k.rt.sender = new DeepSender();
    const first = await tick(k.rt);
    expect(first.results[0]).toMatchObject({ ok: false });
    expect(k.store.current.deepRoutes).toEqual([{ mint: TOKEN_MINT.toBase58(), side: 'buy', at: k.chain.time }]);
    const prepared = k.router.prepared.length;
    const second = await tick(k.rt);
    expect(second.results[0]!.refusals!.join(' ')).toMatch(/call depth/);
    expect(k.router.prepared.length).toBe(prepared);
    expect(DEEP_ROUTE_SECS).toBe(86_400);
  });
});
