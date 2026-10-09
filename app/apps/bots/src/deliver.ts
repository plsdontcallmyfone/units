// Changed by Hookwars: marks an event posted only after a successful send (app audit A-10).
/**
 * One pass of the bots (app audit A-10): an event counts as posted only after every enabled channel
 * answered 2xx, the state is written after each success, and one failing event does not stop the
 * rest. Events with nothing to say are marked as seen so they are not considered again.
 */
import type { BattleEvent } from '@hookwars/shared';

export type Sender = (text: string) => Promise<boolean>;

export interface PassResult { posted: number; failed: number; skipped: number }

export async function deliver(
  events: BattleEvent[],
  posted: Set<string>,
  textFor: (e: BattleEvent) => string | null,
  send: Sender,
  save: (posted: Set<string>) => void,
): Promise<PassResult> {
  const out: PassResult = { posted: 0, failed: 0, skipped: 0 };
  for (const e of events) {
    const key = `${e.signature}:${e.ordinal}`;
    if (posted.has(key)) continue;
    const text = textFor(e);
    if (!text) { posted.add(key); save(posted); out.skipped++; continue; }
    let ok = false;
    try { ok = await send(text); } catch { ok = false; }
    if (ok) { posted.add(key); save(posted); out.posted++; } else out.failed++;
  }
  return out;
}
