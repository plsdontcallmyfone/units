// A ready agent on a mocked chain, for tests and `--mock` style dry runs. Every figure here is a
// TEST value for the fixtures, not a recommendation.
import { Keypair, PublicKey } from '@solana/web3.js';
import { hookwars } from '@hookwars/sdk';
import { parseConfig, type AgentConfig } from './config.ts';
import type { DirectiveConstraints } from './directive.ts';
import { memoryLogger } from './log.ts';
import type { Runtime } from './loop.ts';
import { MockChain, MockRouter, agentAddresses } from './mock-chain.ts';
import { stubModel, type StubScript } from './models/stub.ts';
import { RecordingSender } from './sender.ts';
import { memoryStore } from './state.ts';

const seeded = (n: number) => Keypair.fromSeed(new Uint8Array(32).fill(n));

export const OPERATOR = seeded(1);
export const AGENT = seeded(2);
export const FEE_COLLECTOR = new PublicKey(new Uint8Array(32).fill(3));

export function testConfig(over: Record<string, unknown> = {}): AgentConfig {
  return parseConfig({
    name: 'scout', operator: OPERATOR.publicKey.toBase58(), passportIndex: 0, agentKeypairPath: '/dev/null',
    roles: ['author', 'market', 'diplomat', 'reporter', 'cranker', 'trader'], model: { provider: 'stub' },
    persona: 'Terse. Likes royalty items.', tickSecs: 60, maxActionsPerTick: 4, maxTokens: 512, temperature: 0,
    memo: { maxPerHour: 3, maxPerDay: 10, postage: false },
    author: { templates: [42, 43], maxRoyaltyBps: 500, maxItemsPerDay: 2 },
    market: { maxListPriceLamports: '5000000000', maxLeaseFeeLamports: '1000000000' },
    trade: { caps: { maxPositionLamports: '2000000000', maxTradeLamports: '500000000', dailyLossHaltLamports: '1000000000', drawdownHaltBps: 2000, maxSlippageBps: 100, minHoldSecs: 3600 }, universe: null },
    cranks: { routes: ['settle/prepare', 'seasons/finalize/prepare'] },
    stateDir: '/tmp/never-written',
    ...over,
  });
}

export const OPEN_CONSTRAINTS = (): DirectiveConstraints => ({
  maxSpendPerAction: 500_000_000n, maxSpendPerDay: 2_000_000_000n,
  allowedTargets: [hookwars.MARKET_ID, hookwars.SWAP_ID, hookwars.LAUNCH_ID], allowedAccessModes: 0, maxLicencePrice: 0n, frozen: false,
});

export interface Kit {
  chain: MockChain; router: MockRouter; sender: RecordingSender; store: ReturnType<typeof memoryStore>;
  log: ReturnType<typeof memoryLogger>; model: ReturnType<typeof stubModel>; rt: Runtime;
  passport: PublicKey; vault: PublicKey; cfg: AgentConfig;
}

export function kit(script?: StubScript, cfgOver: Record<string, unknown> = {}, rulesDoc = '{"v":1,"notes":"Build royalty items; list them; report settles."}'): Kit {
  const cfg = testConfig(cfgOver);
  const chain = new MockChain();
  const { passport, vault } = agentAddresses(OPERATOR.publicKey, 0);
  chain.passports.set(passport.toBase58(), { address: passport, operator: OPERATOR.publicKey, index: 0, agentKey: AGENT.publicKey, name: 'scout', status: 0, proof: 1 });
  chain.policies.set(passport.toBase58(), { frozen: false, perActionLamports: 500_000_000n, perDayLamports: 2_000_000_000n, dayStart: BigInt(chain.time - 100), spentToday: 0n, targets: [hookwars.MARKET_ID, hookwars.SWAP_ID, hookwars.LAUNCH_ID] });
  chain.config = { feeCollector: FEE_COLLECTOR, targets: [hookwars.MARKET_ID, hookwars.SWAP_ID, hookwars.LAUNCH_ID] };
  chain.memo = { memoMaxBytes: 600, postageLamports: 5_000n, minProof: 1 };
  chain.balances.set(vault.toBase58(), 3_000_000_000n);
  chain.minted = 7n;
  chain.postDirective(passport, 0, OPEN_CONSTRAINTS(), 'https://rules.example/scout/0.json', rulesDoc);
  const router = new MockRouter();
  const sender = new RecordingSender();
  const store = memoryStore();
  const log = memoryLogger();
  const model = stubModel(script);
  const rt: Runtime = { cfg, agentKey: AGENT.publicKey, model, chain, router, sender, store, log };
  return { chain, router, sender, store, log, model, rt, passport, vault, cfg };
}

/** A sender that reports every plan as sent (to test what the loop records after a send). */
export class SentSender extends RecordingSender {
  /** Runs after each send, to stand for what the transaction changed on chain. */
  onSubmit: (() => void) | null = null;
  override async submit(...a: Parameters<RecordingSender['submit']>) {
    await super.submit(...a);
    this.onSubmit?.();
    return { sent: true, signature: `sig${this.submitted.length}`, unitsConsumed: 1, logs: [], error: null, bytes: 1 };
  }
}
