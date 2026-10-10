// Changed by Hookwars: D-4 (17-randomness). The keeper steps of a loot roll on Switchboard
// randomness, for tickets the agent key itself holds: create a randomness account (a fresh key the
// runtime generates and co-signs with), then commit and roll in one transaction, then, a few slots
// later, reveal and mint in one transaction. Every prepared transaction goes through the same
// checks as a crank (review 3 M-10) before the runtime signs it.
import { Keypair, type AddressLookupTableAccount, type PublicKey, type TransactionInstruction } from '@solana/web3.js';
import type { Router } from './router.ts';
import { preparedProblems } from './guard.ts';

export interface LootStep {
  label: string;
  instructions: TransactionInstruction[];
  tables: AddressLookupTableAccount[];
  /** Keys besides the agent key that sign (the new randomness account on create). */
  signers: Keypair[];
}

export class LootError extends Error {}

async function step(router: Router, route: string, body: Record<string, unknown>, label: string, agent: PublicKey, extra: Keypair[] = []): Promise<LootStep> {
  const prepared = await router.prepare(route, body);
  const allowed = [agent, ...extra.map((k) => k.publicKey)];
  const foreign = prepared.instructions.flatMap((ix) => ix.keys.filter((k) => k.isSigner && !allowed.some((a) => a.equals(k.pubkey))));
  if (foreign.length) throw new LootError(`${label}: needs a signer the runtime does not hold (${foreign[0]!.pubkey.toBase58()})`);
  const bad = preparedProblems(route, prepared.instructions, agent, agent);
  if (bad.length) throw new LootError(`${label}: ${bad.join('; ')}`);
  return { label, instructions: prepared.instructions, tables: prepared.tables, signers: extra };
}

/** Step 1: a new Switchboard randomness account whose authority is the agent key. */
export async function createRandomness(router: Router, agent: PublicKey, randomness: Keypair = Keypair.generate()): Promise<{ randomness: Keypair; step: LootStep }> {
  const s = await step(router, 'rolls/randomness/prepare', { owner: agent.toBase58(), randomness: randomness.publicKey.toBase58() }, 'create randomness', agent, [randomness]);
  return { randomness, step: s };
}

/** Step 2: Switchboard's commit and the war program's `roll`, in one transaction. */
export function commitAndRoll(router: Router, agent: PublicKey, mint: PublicKey, nonce: bigint, randomness: PublicKey): Promise<LootStep> {
  return step(router, 'rolls/prepare', { owner: agent.toBase58(), mint: mint.toBase58(), nonce: nonce.toString(), oracleAccount: randomness.toBase58() }, 'commit and roll', agent);
}

/** Step 3: Switchboard's reveal and the war program's `reveal`, in one transaction (anyone may send it). */
export function revealAndMint(router: Router, revealer: PublicKey, owner: PublicKey, mint: PublicKey, nonce: bigint): Promise<LootStep> {
  return step(router, 'rolls/reveal/prepare', { revealer: revealer.toBase58(), owner: owner.toBase58(), mint: mint.toBase58(), nonce: nonce.toString() }, 'reveal loot', revealer);
}
