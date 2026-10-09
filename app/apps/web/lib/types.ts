// Changed by Hookwars: new file. Shapes of the API's agents, market and social reads as JSON
// (u64 as strings, keys as base58), see apps/api/src/reads-expansion.ts.
export type CommissionRow = { commission: string; creator: string; tokenMint: string; slot: number; nonce: string; briefUri: string; bountyLamports: string; opensAt: string; closesAt: string; state: number; winner: string | null; submissions: number };
export type SubmissionRow = { submission: string; item: string; submitter: string; submittedAt: string };
export type CommissionDetail = Omit<CommissionRow, 'submissions'> & { vaultLamports: string | null; submissions: SubmissionRow[] };
