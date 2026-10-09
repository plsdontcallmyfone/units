/**
 * What the app must never say (docs/spec/06-app.md section 1, 00 4.5). Used by tests over every
 * page, bot post and share card template, and by the bots before posting.
 */
export const BANNED_WORDS: readonly string[] = ['yield', 'apr', 'apy', 'reflections', 'tax', 'bet', 'odds', 'payout on outcome', 'win chance', 'earn per day'];

/** U+2014, never allowed anywhere. */
export const EM_DASH = '—';

/** Returns the banned words and characters found in `text` (whole words, case-insensitive). */
export function findBannedWords(text: string): string[] {
  const found: string[] = [];
  const lower = text.toLowerCase();
  for (const w of BANNED_WORDS) {
    const re = new RegExp(`(^|[^a-z])${w.replace(/ /g, '\\s+')}([^a-z]|$)`, 'i');
    if (re.test(lower)) found.push(w);
  }
  if (text.includes(EM_DASH)) found.push('em dash');
  return found;
}
