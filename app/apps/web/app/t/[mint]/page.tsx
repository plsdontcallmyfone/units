import Link from 'next/link';
import type { General, ProposalInfo, SlotInfo, TreatyInfo, WarInfo } from '@hookwars/shared';
import { read } from '@/lib/api';
import { int, short } from '@/lib/format';
import { Empty, Head, Panel, ReadFailed, Stat } from '@/components/ui';

function bounds(s: SlotInfo): string {
  const parts = [`cuts up to ${s.bounds.maxCutBps / 100}%`];
  parts.push(s.bounds.mayRefuse ? 'may refuse' : 'never refuses');
  if (s.bounds.mayWriteData) parts.push(`${s.dataRange.len} bytes of holder data`);
  return parts.join(' · ');
}

export default async function TokenPage({ params }: { params: Promise<{ mint: string }> }) {
  const { mint } = await params;
  const [slots, proposals, war, treaties, generals] = await Promise.all([
    read<SlotInfo[]>(`/v1/launches/${mint}/slots`), read<ProposalInfo[]>(`/v1/launches/${mint}/proposals`),
    read<WarInfo | null>(`/v1/launches/${mint}/war`), read<TreatyInfo[]>(`/v1/launches/${mint}/treaties`), read<General[]>(`/v1/launches/${mint}/generals`),
  ]);
  const s = slots.ok ? slots.data : [];
  const warSlot = s.find((x) => x.kind === 'war');
  return (
    <>
      <Head eyebrow="Token" title={short(mint, 6)} lede="Slots, the items in them, proposals, the war chest and its treaties. Bounds were fixed at launch; only the items change." right={<Link className="btn" href={`/t/${mint}/generals`}>Generals</Link>} />
      <div className="stats" style={{ marginBottom: 16 }}>
        <Stat label="Slots" value={slots.ok ? s.length : '-'} sub={slots.ok ? `${s.filter((x) => x.item).length} filled` : undefined} />
        <Stat label="Open proposals" value={proposals.ok ? proposals.data.filter((p) => p.status === 'open').length : '-'} />
        <Stat label="War chest" value={war.ok ? (war.data ? 'open' : 'none') : '-'} sub={war.ok && war.data?.chest ? short(war.data.chest) : 'opened by init_war'} />
        <Stat label="Treaties" value={treaties.ok ? treaties.data.length : '-'} />
        <Stat label="Generals" value={generals.ok ? generals.data.length : '-'} sub="top raiders this season" />
      </div>
      <div className="grid cols-main">
        <div className="grid">
          <Panel title="Slots" meta="fixed for life: kind, bounds, rule, notice" flush>
            {!slots.ok ? <ReadFailed what="slots" error={slots.error} /> : s.length === 0 ? (
              <Empty title="No slot table indexed for this mint" what="A units mint gets its slot table in prepare_launch (SlotsInitialized). This mint has none in the indexer: it is not a units launch, or the indexer has not reached it." />
            ) : (
              <table>
                <thead><tr><th>Slot</th><th>Kind</th><th>Item</th><th>Rule</th></tr></thead>
                <tbody>
                  {s.map((x) => (
                    <tr key={x.slot}>
                      <td>{x.slot}</td>
                      <td><span className="chip">{x.kind}</span><div className="faint" style={{ fontSize: 12, marginTop: 4 }}>{bounds(x)}</div></td>
                      <td>{x.locked ? <span className="muted">The kit, locked at launch</span> : x.item ? <Link href={`/armory/items/${x.item.item}`}>{x.item.paramsText}</Link> : <span className="faint">Empty</span>}</td>
                      <td className="muted">{x.equipRule}{x.openProposal ? <div className="chip accent" style={{ marginTop: 4 }}>vote open</div> : null}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </Panel>
          <Panel title="War orders" meta="the War slot">
            {warSlot?.item ? <p style={{ margin: 0 }}>{warSlot.item.paramsText}</p> : <Empty title="No war orders: every war action is off" what="Funding still accrues to the chest. Sieges, counter-strikes, razes and bounties need a War orders item in the War slot, equipped by the slot's rule." />}
          </Panel>
          <Panel title="Proposals" flush>
            {!proposals.ok ? <ReadFailed what="proposals" error={proposals.error} /> : proposals.data.length === 0 ? (
              <Empty title="No proposals" what="Holders propose an item for a Vote or Performance slot. Voting locks your tokens in your wallet until the vote ends; after the notice anyone executes it, settling the outgoing item first." />
            ) : (
              <table><thead><tr><th>Slot</th><th>Status</th><th className="num">For</th><th className="num">Against</th></tr></thead>
                <tbody>{proposals.data.map((p) => <tr key={p.proposal}><td>{p.slot}</td><td>{p.status}</td><td className="num">{int(p.votesFor)}</td><td className="num">{int(p.votesAgainst)}</td></tr>)}</tbody></table>
            )}
          </Panel>
        </div>
        <div className="grid">
          <Panel title="War">
            {!war.ok ? <ReadFailed what="the war state" error={war.error} /> : !war.data ? (
              <Empty title="No war chest" what="A token with a War slot opens its chest with init_war after launch. A companion launch funds it with war_bps of every creator fee claim." />
            ) : <dl className="kv"><dt>Chest</dt><dd className="addr">{war.data.chest}</dd></dl>}
          </Panel>
          <Panel title="Treaties and inbox">
            {treaties.ok && treaties.data.length ? null : <Empty title="No treaties" what="A treaty is one item both tokens equip, each aiming it at the other. Payments land in the treaty inbox and stream to holders through the kit; a token without kit holder rewards cannot receive them." />}
          </Panel>
          <Panel title="Join the raid">
            <Empty title="No raid aimed from here" what="When this token equips a Raid aimed at a rival you hold, this panel quotes selling the rival for this token in one swap_route, with the fee cut and the points it earns." />
          </Panel>
        </div>
      </div>
    </>
  );
}
