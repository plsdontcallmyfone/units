import Link from 'next/link';
import type { BattleEvent, General, Page, ProposalInfo, SlotInfo, TreatyInfo, WarInfo } from '@hookwars/shared';
import { read } from '@/lib/api';
import type { MarketInfo } from '@/lib/mock';
import { DASH, ago, compact, int, short, sol, until } from '@/lib/format';
import { Empty, ReadFailed } from '@/components/ui';
import { TokenChart } from '@/components/token-chart';
import { CopyMint } from '@/components/copy-mint';
import { TokenActions } from '@/components/token-actions';
import './token-page.css';

type LaunchRow = Record<string, unknown>;
const KIND_LABEL: Record<string, string> = {
  raid: 'Raid', siege: 'Siege', siege_waited: 'Siege waited', counter_strike: 'Counter-strike', raze: 'Raze', return: 'Return',
  treaty_on: 'Treaty signed', treaty_off: 'Treaty ended', treaty_shared: 'Treaty shared', equip: 'Equip', proposal: 'Proposal',
  settle: 'Settle', bounty: 'Bounty', roll: 'Roll', loot: 'Loot', forge: 'Forge', quest: 'Quest', season: 'Season', prize: 'Prize',
};
const MOMENT_KIND: Record<string, string> = { raid: 'k-decision', siege: 'k-error', counter_strike: 'k-error', proposal: 'k-doubt', treaty_on: 'k-system', equip: 'k-system' };

function sentence(e: BattleEvent, mint: string): string {
  const other = e.otherMint ? short(e.otherMint) : 'a rival';
  const amt = e.amount ? sol(e.amount, 2) : '';
  switch (e.kind) {
    case 'raid': return e.mint === mint ? `Raided ${other} for ${amt}.` : `Raided by ${short(e.mint)} for ${amt}.`;
    case 'siege': return e.mint === mint ? `Laid siege to ${other}, spending ${amt} from the chest.` : `Besieged by ${short(e.mint)} with ${amt}.`;
    case 'treaty_on': return `Signed a treaty with ${other}.`;
    case 'proposal': return `A holder proposed an item for slot ${e.detail.slot ?? '?'}.`;
    case 'bounty': return `A crank claimed a ${amt} bounty.`;
    case 'equip': return `Equipped ${e.detail.template ?? 'an item'} in slot ${e.detail.slot ?? '?'}.`;
    case 'forge': return `Forged a level ${e.detail.level ?? '?'} ${e.detail.template ?? 'item'}.`;
    case 'loot': return `A loot roll dropped a ${e.detail.template ?? 'item'}.`;
    default: return `${KIND_LABEL[e.kind] ?? e.kind}${amt ? ` for ${amt}` : ''}.`;
  }
}
function hhmm(ts: number): string { const d = new Date(ts * 1000); return `${String(d.getUTCHours()).padStart(2, '0')}:${String(d.getUTCMinutes()).padStart(2, '0')}`; }

function Pn({ title, aside, className = '', flush, children }: { title: string; aside?: string; className?: string; flush?: boolean; children: React.ReactNode }) {
  return <section className={`pn ${className}`}><header className="pn-h"><h2>{title}</h2>{aside ? <span className="pn-aside">{aside}</span> : null}</header><div className={flush ? 'pn-b flush' : 'pn-b'} tabIndex={0} role="region" aria-label={title}>{children}</div></section>;
}

export default async function TokenPage({ params }: { params: Promise<{ mint: string }> }) {
  const { mint } = await params;
  const [launches, slots, proposals, war, treaties, generals, feed, market] = await Promise.all([
    read<Page<LaunchRow>>('/v1/launches'), read<SlotInfo[]>(`/v1/launches/${mint}/slots`), read<ProposalInfo[]>(`/v1/launches/${mint}/proposals`),
    read<WarInfo | null>(`/v1/launches/${mint}/war`), read<TreatyInfo[]>(`/v1/launches/${mint}/treaties`), read<General[]>(`/v1/launches/${mint}/generals`),
    read<Page<BattleEvent>>('/v1/feed'), read<MarketInfo>(`/v1/launches/${mint}/market`),
  ]);
  const launch = launches.ok ? launches.data.items.find((l) => String(l.mint) === mint) : undefined;
  const name = String(launch?.name ?? short(mint, 6));
  const symbol = String(launch?.symbol ?? short(mint));
  const image = typeof launch?.image === 'string' ? launch.image : null;
  const s = slots.ok ? slots.data : [];
  const warSlot = s.find((x) => x.kind === 'war');
  const w = war.ok ? war.data : null;
  const mine = feed.ok ? feed.data.items.filter((e) => e.mint === mint || e.otherMint === mint) : [];
  const raids24 = mine.filter((e) => e.kind === 'raid' && e.ts > Date.now() / 1000 - 86400);
  const state = w?.underSiege ? 'siege' : mine.length ? 'war' : 'quiet';
  const m = market.ok ? market.data : null;
  const spent = w?.spent ? { siege: Number(w.spent.siege), counter: Number(w.spent.counter), bounties: Number(w.spent.bounties) } : null;
  const reserve = w?.chestBalance ? Number(w.chestBalance) : 0;
  const total = spent ? spent.siege + spent.counter + spent.bounties + reserve : 0;
  const pct = (v: number) => total ? `${Math.round((v / total) * 100)}%` : DASH;
  const signals: [string, string, number][] = [];
  if (w?.underSiege) signals.push([`under siege by ${short(w.underSiege.byMint ?? w.underSiege.byChest)} until ${hhmm(w.underSiege.until)} UTC`, until(w.underSiege.until), 3]);
  for (const p of proposals.ok ? proposals.data.filter((x) => x.status === 'open') : []) signals.push([`vote open on slot ${p.slot}: ${compact(p.votesFor)} for, ${compact(p.votesAgainst)} against`, `ends ${until(p.voteEnd)}`, 2]);
  if (raids24.length) signals.push([`${raids24.length} raids in the last 24h, ${sol(raids24.reduce((a, e) => a + Number(e.amount ?? 0), 0), 2)} of volume`, ago(raids24[0]!.ts), raids24.length > 2 ? 3 : 1]);
  if (m) signals.push([`price ${m.priceChange24h >= 0 ? 'up' : 'down'} ${Math.abs(m.priceChange24h)}% in 24h`, '24h', Math.abs(m.priceChange24h) > 20 ? 3 : Math.abs(m.priceChange24h) > 8 ? 2 : 1]);
  for (const x of s.filter((x) => x.kind !== 'locked' && !x.item)) signals.push([`slot ${x.slot} (${x.kind}) is empty, equips by ${x.equipRule}`, 'now', 1]);
  if (!warSlot?.item) signals.push(['no War orders: sieges, counter-strikes and bounties are off', 'now', 2]);

  return (
    <div className="cv3">
      <header className="cv3-hero">
        <div className="cv3-art">{image ? <img src={image} width={66} height={66} alt="" /> : <span>{symbol.slice(0, 3)}</span>}</div>
        <div className="cv3-name">
          <nav className="cv3-crumbs"><Link href="/">Projects</Link><span>/</span><span>${symbol}</span><em className={state === 'siege' ? 'bad' : state}>{state === 'siege' ? 'under siege' : state === 'war' ? 'at war' : 'quiet'}</em></nav>
          <h1>{name}</h1>
          <div className="cv3-links">
            <Link className="btn sm primary" href={`/war?mint=${mint}`}>Raid ↗</Link>
            <CopyMint mint={mint} />
            <a href={`https://solscan.io/token/${mint}?cluster=devnet`} target="_blank" rel="noreferrer">Solscan</a>
            <Link href={`/t/${mint}/generals`}>Generals</Link>
            <Link href="/armory">Armory</Link>
          </div>
        </div>
        <div className="cx-market">
          <div className="cx-mc"><span className="cx-label">War chest {w?.chest ? <i /> : null}</span><span className="cx-mc-v">{w?.chestBalance ? sol(w.chestBalance, 1) : DASH}</span><span className="cx-sub">{w?.fundedTotal ? `${sol(w.fundedTotal, 1)} funded all time` : w ? 'opened by init_war' : 'no chest'}</span></div>
          <dl className="cx-stats">
            <div><dt>Price</dt><dd>{m ? `${m.priceSol.toPrecision(3)} SOL` : DASH}</dd></div>
            <div><dt>Raids 24h</dt><dd>{feed.ok ? int(raids24.length) : DASH}</dd></div>
            <div><dt>Holders</dt><dd>{m ? int(m.holders) : DASH}</dd></div>
            <div><dt>Slots</dt><dd>{slots.ok ? `${s.filter((x) => x.item || x.locked).length}/${s.length}` : DASH}</dd></div>
          </dl>
        </div>
      </header>

      <div className="cv3-top">
        <section className="pn cv3-mind">
          <header className="cv3-who">
            <div className="who-av">W</div>
            <div className="who-n"><b>War orders</b><span>{warSlot ? `slot ${warSlot.slot} · equips by ${warSlot.equipRule}${warSlot.noticeSecs ? ` · ${warSlot.noticeSecs / 60} min notice` : ''}` : 'no War slot on this token'}</span></div>
            <span className={`cx-state ${warSlot?.item ? 'on' : w?.underSiege ? 'halted' : ''}`}><i />{warSlot?.item ? 'armed' : w?.underSiege ? 'besieged' : 'unarmed'}</span>
          </header>
          <div className="cv3-char">
            <div className="char-a"><b>{warSlot?.item ? warSlot.item.templateName : 'No orders'}</b><span>{warSlot?.item ? `level ${warSlot.item.level} · ${warSlot.item.source} · owner ${short(warSlot.item.owner)}` : 'every war action is off'}</span></div>
            <p><span className="k">orders</span>{warSlot?.item ? warSlot.item.paramsText : 'Funding still accrues to the chest. Sieges, counter-strikes, razes and bounties need a War orders item in the War slot, equipped by the slot rule.'}</p>
            {w?.underSiege ? <p className="voice">Under siege by {short(w.underSiege.byMint ?? w.underSiege.byChest)} until {hhmm(w.underSiege.until)} UTC.</p> : null}
            <p className="desc">{s.length ? `${s.length} slots fixed at launch: ${s.map((x) => x.kind).join(', ')}.` : 'No slot table indexed for this mint.'}</p>
          </div>
          <div className="cv3-feedh"><span className="k">Battle log</span><span className="live"><i />live</span></div>
          <div className="cv3-feed scroll">
            {!feed.ok ? <ReadFailed what="the feed" error={feed.error} /> : mine.length === 0 ? <div className="mf-empty">Nothing has happened to this token yet.</div> : mine.map((e) => (
              <div className="mf-moment" key={`${e.signature}:${e.ordinal}`}>
                <div className="mf-time">{ago(e.ts)}<span>{hhmm(e.ts)}</span></div>
                <div className="mf-body"><div className={`mf-th ${MOMENT_KIND[e.kind] ?? ''}`}><span className="mf-k">{KIND_LABEL[e.kind] ?? e.kind}{e.actor ? ` · ${short(e.actor)}` : ''}</span><p>{sentence(e, mint)}</p></div></div>
              </div>
            ))}
          </div>
        </section>

        <div className="cv3-mid">
          <Pn title="Chart" aside={m ? '15m candles, 24h' : 'no market data'} className="cv3-chart">
            {m ? <TokenChart series={m.series} symbol={symbol} /> : <Empty title="No market data for this mint" what="Price, chest balance and raid volume chart here once the indexer records swaps and chest moves for this token." />}
          </Pn>
          <div className="cv3-pair">
            <Pn title="Slots" aside={slots.ok ? `${s.length} fixed at launch` : undefined} className="mission">
              {!slots.ok ? <ReadFailed what="slots" error={slots.error} /> : s.length === 0 ? <p className="dim">No slot table indexed for this mint.</p> : (
                <ul className="ms-done">{s.map((x) => <li key={x.slot}><span className={`tag ${x.locked ? 't-locked' : x.item ? 't-active' : 't-empty'}`}>{x.locked ? 'kit' : x.item ? x.item.templateName : 'empty'}</span><span style={{ color: 'var(--ink-2)' }}>slot {x.slot} · {x.kind} · {x.equipRule}</span>{x.openProposal ? <span className="tag">vote open</span> : null}</li>)}</ul>
              )}
            </Pn>
            <Pn title="Chest split" aside={w ? 'all time' : undefined} className="thesis">
              {!spent ? <p className="dim">The split of chest spend shows once the chest has moved.</p> : (
                <>
                  <div className="split" role="img" aria-label="Chest: siege, counter-strikes, bounties, reserve"><i className="s-siege" style={{ flexGrow: spent.siege || 1 }} /><i className="s-counter" style={{ flexGrow: spent.counter || 1 }} /><i className="s-bounties" style={{ flexGrow: spent.bounties || 1 }} /><i className="s-reserve" style={{ flexGrow: reserve || 1 }} /></div>
                  <ul className="legend"><li><i className="s-siege" />Siege <b>{pct(spent.siege)}</b></li><li><i className="s-counter" />Counter <b>{pct(spent.counter)}</b></li><li><i className="s-bounties" />Bounties <b>{pct(spent.bounties)}</b></li><li><i className="s-reserve" />Reserve <b>{pct(reserve)}</b></li></ul>
                  <p>The chest spends only through bounded cranks the War orders allow. What is not spent stays as reserve for the next siege or counter-strike.</p>
                </>
              )}
            </Pn>
          </div>
        </div>

        <div className="cv3-side">
          <Pn title="Chest" aside={w?.chestBalance ? sol(w.chestBalance, 2) : undefined} className="cv3-treasury">
            {!war.ok ? <ReadFailed what="the war state" error={war.error} /> : !w ? <p className="dim">No war chest. A token with a War slot opens its chest with init_war after launch.</p> : (
              <>
                <ul className="assets">
                  <li><b>SOL</b><span>{w.chestBalance ? (Number(w.chestBalance) / 1e9).toLocaleString('en-US', { maximumFractionDigits: 2 }) : DASH}</span><em>reserve</em><i style={{ width: pct(reserve) === DASH ? '0%' : pct(reserve) }} /></li>
                  <li><b>Treaty inbox</b><span>{w.treatyInbox?.balance ? (Number(w.treatyInbox.balance) / 1e9).toFixed(2) : '0'}</span><em>{w.treatyInbox ? short(w.treatyInbox.address) : 'none'}</em><i style={{ width: '0%' }} /></li>
                  <li><b>Captured</b><span>{w.captured.length}</span><em>{w.razedProceeds ? sol(w.razedProceeds, 2) : 'no razes'}</em><i style={{ width: '0%' }} /></li>
                </ul>
                <p className="cv3-bb"><span className="k">chest</span><b>{short(w.chest ?? '', 6)}</b> <em>funded {w.fundedTotal ? sol(w.fundedTotal, 2) : DASH} all time{m ? `, ${sol(m.feesToChest, 2)} from fees` : ''}</em></p>
              </>
            )}
          </Pn>
          <Pn title="Moves" aside="on chain" className="cv3-moves">
            <div className="scroll" tabIndex={0} role="region" aria-label="Moves list">
              {mine.length === 0 ? <p className="dim" style={{ padding: '12px 0' }}>No moves yet.</p> : (
                <ul className="mv">{mine.map((e) => <li key={`${e.signature}:${e.ordinal}`}><a href={`https://solscan.io/tx/${e.signature}?cluster=devnet`} target="_blank" rel="noreferrer"><p>{sentence(e, mint)}</p><span><em className={e.kind === 'siege' && e.otherMint === mint ? 'bad' : 'ok'}>on chain</em> · {ago(e.ts)}</span></a><span className="mv-tx">tx ↗</span></li>)}</ul>
              )}
            </div>
          </Pn>
          <Pn title="Event radar" aside={`${signals.length} signals`} className="radar">
            {signals.length === 0 ? <p className="dim">Nothing to flag.</p> : <ul className="rd">{signals.map(([text, time, sev], i) => <li key={i} className={`sev-${sev}`}><i /><span>{text}</span><time>{time}</time></li>)}</ul>}
          </Pn>
        </div>
      </div>

      <div className="cv3-base">
        <Pn title="Proposals" aside="holders propose items for Vote and Performance slots" flush>
          {!proposals.ok ? <ReadFailed what="proposals" error={proposals.error} /> : proposals.data.length === 0 ? <Empty title="No proposals" what="Voting locks your tokens in your wallet until the vote ends; after the notice anyone executes it." /> : (
            <table><thead><tr><th>Slot</th><th>Item</th><th>Status</th><th className="num">For</th><th className="num">Against</th><th className="num">Ends</th></tr></thead>
              <tbody>{proposals.data.map((p) => <tr key={p.proposal}><td>{p.slot}</td><td>{p.item ? <Link href={`/armory/items/${p.item.item}`}>{p.item.templateName} L{p.item.level}</Link> : DASH}</td><td><span className={`tag ${p.status === 'open' ? 't-active' : ''}`}>{p.status}</span></td><td className="num">{compact(p.votesFor)}</td><td className="num">{compact(p.votesAgainst)}</td><td className="num faint">{p.status === 'open' ? until(p.voteEnd) : DASH}</td></tr>)}</tbody></table>
          )}
        </Pn>
        <Pn title="Treaties" aside={treaties.ok ? `${treaties.data.length}` : undefined}>
          {!treaties.ok ? <ReadFailed what="treaties" error={treaties.error} /> : treaties.data.length === 0 ? <p className="dim">No treaties. A treaty is one item both tokens equip, each aiming it at the other. Payments land in the treaty inbox and stream to holders through the kit.</p> : <ul className="ms-done">{treaties.data.map((t, i) => <li key={i}><span className="tag t-active">treaty</span>{JSON.stringify(t).slice(0, 60)}</li>)}</ul>}
        </Pn>
        <Pn title="Generals" aside="top raiders this season" flush>
          {!generals.ok ? <ReadFailed what="generals" error={generals.error} /> : generals.data.length === 0 ? <Empty title="No raiders yet" what="A raid is a buy of this token paid for by selling a rival its Raid item targets." /> : (
            <table><thead><tr><th>#</th><th>Raider</th><th className="num">Points</th><th className="num">Volume</th></tr></thead>
              <tbody>{generals.data.map((g) => <tr key={g.owner}><td>{g.rank}</td><td className="addr">{short(g.owner, 6)}</td><td className="num">{int(g.raidPoints)}</td><td className="num">{sol(g.raidVolume, 2)}</td></tr>)}</tbody></table>
          )}
        </Pn>
      </div>
      {/* Changed by Hookwars (app v2): every action on this token, signed in the wallet. */}
      <div className="cv3-acts">
        <Pn title="Act" aside="signed in your wallet, sent through the backend">
          <TokenActions mint={mint} hasWar={Boolean(w?.chest)} />
        </Pn>
      </div>
    </div>
  );
}
