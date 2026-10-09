// Changed by Hookwars: new file. Every prepare that acts on one token, as forms on its page; each
// signs in the wallet and sends through /api/v1/submit (components/action.tsx).
import { Action } from './action';

export function TokenActions({ mint, hasWar }: { mint: string; hasWar: boolean }) {
  const m = { mint };
  return (
    <div className="act-grid">
      <Action route="buy" title="Buy" what="Buys this token for SOL on its pool." fixed={m} fields={[{ name: 'amount', label: 'SOL in', kind: 'sol' }, { name: 'minOut', label: 'Least tokens out (base units)', kind: 'amount', optional: true }]} />
      <Action route="raid" title="Raid" what="Buys this token by selling a rival it targets; earns raid points where a Raid item is equipped." fixed={{ target: mint }} fields={[{ name: 'rival', label: 'Rival token', kind: 'key' }, { name: 'amount', label: 'Rival tokens to sell (base units)', kind: 'amount' }, { name: 'minOut', label: 'Least tokens out (base units)', kind: 'amount', optional: true }]} />
      <Action route="proposals" title="Propose an item" what="For a Vote slot. Empty item proposes clearing the slot." fixed={{ ...m, targets: [] }} fields={[{ name: 'slot', label: 'Slot', kind: 'int' }, { name: 'item', label: 'Item', kind: 'key', optional: true }, { name: 'targets', label: 'Targets', kind: 'keys', optional: true, hint: 'comma separated mints' }, { name: 'role', label: 'Treaty role', kind: 'text', optional: true, choices: [['none', 'none'], ['pay', 'pay'], ['receive', 'receive']] }]} />
      <Action route="votes" title="Vote" what="Locks the tokens you vote with in your wallet until the vote ends." fixed={m} fields={[{ name: 'slot', label: 'Slot', kind: 'int' }, { name: 'nonce', label: 'Proposal nonce', kind: 'amount' }, { name: 'support', label: 'For', kind: 'bool' }, { name: 'amount', label: 'Tokens (base units)', kind: 'amount' }]} />
      <Action route="proposals/finalize" title="Count a vote" what="Anyone, after the vote ends." fixed={m} fields={[{ name: 'slot', label: 'Slot', kind: 'int' }, { name: 'nonce', label: 'Proposal nonce', kind: 'amount' }]} />
      <Action route="votes/close" title="Close your vote" what="Unlocks your tokens after the vote is counted." fixed={m} fields={[{ name: 'slot', label: 'Slot', kind: 'int' }, { name: 'nonce', label: 'Proposal nonce', kind: 'amount' }]} />
      <Action route="settle" title="Settle a slot" what="Moves what the slot's item collected to its royalty and its destinations; the crank keeps the settle bounty." fixed={m} fields={[{ name: 'slot', label: 'Slot', kind: 'int' }]} />
      {hasWar ? <Action route="bounties" title="Claim a bounty" what="Pays a crank that performed a war action the War orders allow." fixed={m} /> : <Action route="war/init" title="Open the war chest" what="Once per token with a War slot." fixed={m} />}
      <Action route="quests" title="Claim a quest" fixed={m} fields={[{ name: 'season', label: 'Season', kind: 'int' }, { name: 'questId', label: 'Quest', kind: 'int', choices: [['1', 'raid quest'], ['2', 'forge quest']] }, { name: 'period', label: 'Period', kind: 'int' }]} />
      <Action route="rolls" title="Roll a loot ticket" fixed={m} fields={[{ name: 'nonce', label: 'Nonce', kind: 'amount' }, { name: 'oracleAccount', label: 'Randomness account', kind: 'key' }]} />
      <Action route="referral/set" title="Name your referrer" what="Where a Referral item is equipped." fixed={m} fields={[{ name: 'referrer', label: 'Referrer', kind: 'key' }]} />
      <Action route="loyalty/claim" title="Claim from the loyalty pot" what="Where a Loyalty Pot item is equipped." fixed={m} fields={[{ name: 'slot', label: 'Slot', kind: 'int' }]} />
      <Action route="commissions/open" title="Commission a hook" what="Escrow a bounty for a new item for one slot." fixed={{ tokenMint: mint }} fields={[{ name: 'slot', label: 'Slot', kind: 'int' }, { name: 'nonce', label: 'Nonce', kind: 'amount' }, { name: 'briefUri', label: 'Brief URI', kind: 'text' }, { name: 'bountyLamports', label: 'Bounty', kind: 'sol' }, { name: 'windowSecs', label: 'Window (seconds)', kind: 'int' }]} />
    </div>
  );
}
