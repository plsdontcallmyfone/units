// Changed by Hookwars: D-4 (17-randomness). Loot rolls on Switchboard On-Demand randomness. The war
// program reads Switchboard's own randomness account; these builders put Switchboard's create,
// commit and reveal instructions in front of ours. The Switchboard client is loaded only when a
// route needs it, and tests replace it with a mock.
import { Keypair, PublicKey, type Connection, type TransactionInstruction } from '@solana/web3.js';
import { hookwars } from '@hookwars/sdk';

/** What the API needs from Switchboard (one place, so tests can stand in for the network). */
export interface RandomnessGateway {
  /** `randomness_init` for a new account at `randomness` (the browser holds its key); `owner` pays and is the authority. */
  create(conn: Connection, program: PublicKey, randomness: PublicKey, owner: PublicKey): Promise<TransactionInstruction>;
  /** `randomness_commit` by `owner` (the authority), to land in the same transaction as `roll`. */
  commit(conn: Connection, program: PublicKey, randomness: PublicKey, owner: PublicKey): Promise<TransactionInstruction>;
  /** `randomness_reveal` paid by `payer`, with the value the oracle signed (to draw the item before sending). */
  reveal(conn: Connection, program: PublicKey, randomness: PublicKey, payer: PublicKey): Promise<{ ix: TransactionInstruction; value: Uint8Array }>;
}

/** The real gateway: `@switchboard-xyz/on-demand`, loaded on first use. */
export const switchboardGateway: RandomnessGateway = {
  async create(conn, program, randomness, owner) {
    const { sb, prog, queue } = await load(conn, program);
    // `Randomness.create` reads only the key's public half; the browser signs with the secret.
    const [, ix] = await sb.Randomness.create(prog, { publicKey: randomness } as Keypair, queue, owner);
    return ix;
  },
  async commit(conn, program, randomness, owner) {
    const { sb, prog, queue } = await load(conn, program);
    return new sb.Randomness(prog, randomness).commitIx(queue, owner);
  },
  async reveal(conn, program, randomness, payer) {
    const { sb, prog } = await load(conn, program);
    const ix = await new sb.Randomness(prog, randomness).revealIx(payer);
    // The program's coder is Anchor's Borsh coder (its `InstructionCoder` type omits `decode`).
    const coder = prog.coder.instruction as unknown as { decode(data: Buffer): { data?: { value?: number[] | Uint8Array } } | null };
    const decoded = coder.decode(ix.data);
    const value = decoded?.data?.value;
    if (!value || value.length !== 32) throw new Error('the Switchboard reveal carried no 32-byte value');
    return { ix, value: Uint8Array.from(value) };
  },
};

async function load(conn: Connection, program: PublicKey) {
  const sb = await import('@switchboard-xyz/on-demand');
  const prog = await sb.AnchorUtils.loadProgramFromConnection(conn as never, undefined, program as never);
  const queue = sb.getDefaultQueueAddress(program.equals(hookwars.SWITCHBOARD_MAINNET));
  return { sb, prog, queue };
}

let current: RandomnessGateway = switchboardGateway;
/** The gateway the prepares use. */
export const randomnessGateway = (): RandomnessGateway => current;
/** Tests only: stand in for Switchboard. */
export function setRandomnessGateway(g: RandomnessGateway): void { current = g; }
