/**
 * The posts of 06 7.1, built only from indexed events: every post links the transaction it
 * reports, never states a projection, and passes the banned-words check before it is sent.
 * Thresholds and quiet hours are operator settings, not protocol parameters.
 */
import { findBannedWords, type BattleEvent } from '@hookwars/shared';

export interface BotConfig {
  siteUrl: string;
  explorerTx: (sig: string) => string;
  /** Operator setting: smallest raid volume (lamports) worth a post; null posts every raid. */
  minRaidLamports: bigint | null;
}

const short = (a: string | null) => (a ? `${a.slice(0, 4)}...${a.slice(-4)}` : 'a rival');
const sol = (l: string | null) => (l === null ? '' : `${(Number(l) / 1e9).toLocaleString('en-US', { maximumFractionDigits: 4 })} SOL`);

export function postFor(e: BattleEvent, cfg: BotConfig): string | null {
  const tx = cfg.explorerTx(e.signature);
  let text: string | null = null;
  switch (e.kind) {
    case 'raid': {
      if (cfg.minRaidLamports !== null && BigInt(e.amount ?? '0') < cfg.minRaidLamports) return null;
      text = `${short(e.mint)} is raiding ${short(e.otherMint)} holders: ${sol(e.amount)}. Join: ${cfg.siteUrl}/t/${e.mint}/raid?from=${e.otherMint ?? ''}`;
      break;
    }
    case 'siege': text = `${short(e.mint)} besieged ${short(e.otherMint)}: spent ${sol(e.amount)}.`; break;
    case 'counter_strike': text = `${short(e.mint)} struck back: ${sol(e.amount)} bought and burned.`; break;
    case 'raze': text = `${short(e.mint)} sold captured ${short(e.otherMint)}.`; break;
    case 'return': text = `${short(e.mint)} returned captured ${short(e.otherMint)} under their treaty.`; break;
    case 'season': text = `Season result: ${short(e.mint)} wins.`; break;
    case 'prize': text = `Prize paid: ${sol(e.amount)} to ${short(e.mint)}'s war chest.`; break;
    default: return null;
  }
  const out = `${text} ${tx}`;
  return findBannedWords(out).length ? null : out;
}
