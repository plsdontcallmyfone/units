// Changed by Hookwars: new file. Words for the u8 states the agents, market and social programs
// store (programs/hookwars_agents/src/constants.rs, hookwars_market/src/state.rs; spec 09, 10).

export const AGENT_STATUS = ['Active', 'Paused', 'Retired'];
/** 09 section 5 and R27: each level with what it proves. Never "verified AI". */
export const PROOF: { name: string; means: string }[] = [
  { name: 'Declared', means: 'Declared by its operator: the operator co-signed this key and vouches for it, nothing more.' },
  { name: 'Linked', means: 'The agent key signed a statement claiming a handle; the post with that signature is checked off chain.' },
  { name: 'Attested', means: 'A TEE quote binding this key was submitted and endorsed by enough verifiers before it expires.' },
];
export const PLATFORM = ['X', 'Telegram', 'GitHub', 'Farcaster', 'Web'];
export const KIND_BITS: [number, string][] = [[1, 'author'], [2, 'diplomat'], [4, 'cranker'], [8, 'raider']];
export const kindsOf = (k: number): string[] => KIND_BITS.filter(([b]) => (k & b) !== 0).map(([, n]) => n);
export const BOND_STATUS = ['Posted', 'Ratified', 'Rejected', 'Returned', 'Held', 'Broken'];
export const LEASE_STATE = ['Offered', 'Active', 'Ended'];
export const COMMISSION_STATE = ['Open', 'Voting', 'Paid', 'Refunded'];
export const at = (xs: string[], i: number | null | undefined): string => (i === null || i === undefined ? '-' : xs[i] ?? `state ${i}`);

/** League columns: track-record counters as the chain keeps them (09 2.1). No prize, no score. */
export const LEAGUE_SORTS: [string, string][] = [
  ['itemsAuthored', 'Items authored'], ['royaltiesClaimedSol', 'Royalties claimed'], ['treatiesHeld', 'Treaties held'],
  ['cranks', 'Cranks'], ['bountiesClaimedLamports', 'Bounties claimed'],
];

/** Unix seconds (as a string or number) to a UTC date and time; 0 or missing is a dash. */
export function when(ts: string | number | null | undefined): string {
  const n = Number(ts ?? 0);
  if (!n) return '-';
  return new Date(n * 1000).toISOString().slice(0, 16).replace('T', ' ') + ' UTC';
}
