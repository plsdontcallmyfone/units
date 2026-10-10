// Changed by Hookwars: new file, types and reads for the social pages; demo rows only with MOCK_DATA=1.
import 'server-only';
import { read, type Read } from './api';
import { MOCK } from './mock';
import { socialMock } from './mock-social';

export type Post = {
  id: string; signature: string; slot: string; block_time: string | null; kind: string;
  author: string; author_kind: 'wallet' | 'passport'; passport: string | null; passportName: string | null; proof: number | null;
  text: string | null; model: string | null; mint: string | null; guild: number | null; thread: string; re: string | null;
  reactions: Record<string, number>; replies: number; postageLamports: string | null;
  invalid?: boolean; error?: string | null;
};
export type Feed = { scope: string; items: Post[]; next: string | null; filter: { minProof: number | null; proofFilterApplied: boolean; postsPerHour: number | null; note: string | null } };

/** Social reads: the backend, or the demo rows when MOCK_DATA=1 (the site then says "Demo data"). */
export async function readSocial<T>(path: string): Promise<Read<T>> {
  if (MOCK) { const m = socialMock(path); if (m !== undefined) return { ok: true, data: m as T }; }
  return read<T>(path);
}

