// D-4 (17-randomness): the keeper steps of a Switchboard loot roll, against a mocked router. Each
// prepared transaction is checked like a crank before the runtime would sign it.
import { describe, expect, it } from 'vitest';
import { Keypair, PublicKey, SystemProgram, TransactionInstruction } from '@solana/web3.js';
import { hookwars } from '@hookwars/sdk';
import { commitAndRoll, createRandomness, LootError, revealAndMint } from './loot.ts';
import type { Router } from './router.ts';
import { CRANK_ROUTES } from './router.ts';

const agent = Keypair.generate().publicKey;
const mint = Keypair.generate().publicKey;
const ix = (programId: PublicKey, signers: PublicKey[] = []) =>
  new TransactionInstruction({ programId, keys: signers.map((pubkey) => ({ pubkey, isSigner: true, isWritable: true })), data: Buffer.alloc(8) });

function router(answer: (route: string, body: Record<string, unknown>) => TransactionInstruction[]): Router & { seen: [string, Record<string, unknown>][] } {
  const seen: [string, Record<string, unknown>][] = [];
  return {
    seen,
    async prepare(route, body) { seen.push([route, body]); return { instructions: answer(route, body), tables: [] }; },
    async quoteBuy() { return 0n; },
    async quoteSell() { return 0n; },
  };
}

describe('Switchboard loot keeper (D-4)', () => {
  it('create, commit and roll, then reveal: the routes, the bodies and the co-signing randomness key', async () => {
    const r = router((route, body) => {
      if (route === 'rolls/randomness/prepare') return [ix(hookwars.SWITCHBOARD_DEVNET, [agent, new PublicKey(body.randomness as string)])];
      if (route === 'rolls/prepare') return [ix(hookwars.SWITCHBOARD_DEVNET, [agent]), ix(hookwars.WAR_ID, [agent])];
      return [ix(hookwars.SWITCHBOARD_DEVNET, [agent]), ix(hookwars.WAR_ID, [agent])];
    });
    const { randomness, step } = await createRandomness(r, agent);
    expect(step.signers.map((k) => k.publicKey.toBase58())).toEqual([randomness.publicKey.toBase58()]);
    const roll = await commitAndRoll(r, agent, mint, 7n, randomness.publicKey);
    expect(roll.instructions).toHaveLength(2);
    const reveal = await revealAndMint(r, agent, agent, mint, 7n);
    expect(reveal.instructions).toHaveLength(2);
    expect(r.seen.map(([route]) => route)).toEqual(['rolls/randomness/prepare', 'rolls/prepare', 'rolls/reveal/prepare']);
    expect(r.seen[1]![1]).toEqual({ owner: agent.toBase58(), mint: mint.toBase58(), nonce: '7', oracleAccount: randomness.publicKey.toBase58() });
  });

  it('refuses an answer that calls another program or needs a key the runtime does not hold', async () => {
    const other = router(() => [ix(SystemProgram.programId, [agent])]);
    await expect(commitAndRoll(other, agent, mint, 1n, Keypair.generate().publicKey)).rejects.toThrow(LootError);
    const foreign = router(() => [ix(hookwars.WAR_ID, [agent, Keypair.generate().publicKey])]);
    await expect(revealAndMint(foreign, agent, agent, mint, 1n)).rejects.toThrow(/signer the runtime does not hold/);
  });

  it('revealing is a permissionless crank route', () => {
    expect((CRANK_ROUTES as readonly string[]).includes('rolls/reveal/prepare')).toBe(true);
  });
});
