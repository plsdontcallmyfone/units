import Link from 'next/link';
import type { BattleEvent, Page, PrizeVaultInfo, SeasonInfo, WarMap } from '@hookwars/shared';
import { read } from '@/lib/api';
import { DASH, ago, sol, short } from '@/lib/format';
import { Empty, Head, Panel, ReadFailed, Stat } from '@/components/ui';

const KIND_LABEL: Record<string, string> = {
  raid: 'Raid', siege: 'Siege', siege_waited: 'Siege waited', counter_strike: 'Counter-strike', raze: 'Raze', return: 'Return',
  treaty_on: 'Treaty signed', treaty_off: 'Treaty ended', treaty_shared: 'Treaty shared', equip: 'Equip', proposal: 'Proposal',
  settle: 'Settle', bounty: 'Bounty', roll: 'Roll', loot: 'Loot', forge: 'Forge', quest: 'Quest', season: 'Season', prize: 'Prize',
};

/** Positions nodes on a circle; coordinates rounded so server and client render the same SVG. */
function layout(n: number, i: number): [number, number] {
  const a = (2 * Math.PI * i) / Math.max(1, n);
  return [Math.round(500 + 380 * Math.cos(a)), Math.round(220 + 160 * Math.sin(a))];
}

function MapView({ map }: { map: WarMap }) {
  const pos = new Map(map.nodes.map((nd, i) => [nd.mint, layout(map.nodes.length, i)]));
  return (
    <svg viewBox="0 0 1000 440" width="100%" height="100%" role="img" aria-label={`War map: ${map.nodes.length} tokens, ${map.edges.length} relations`}>
      {map.edges.map((e, i) => {
        const a = pos.get(e.from); const b = pos.get(e.to);
        if (!a || !b) return null;
        return <line key={i} x1={a[0]} y1={a[1]} x2={b[0]} y2={b[1]} stroke={e.kind === 'siege' ? 'var(--bad)' : e.kind === 'treaty' ? 'var(--ok)' : 'var(--accent)'} strokeWidth={2} strokeDasharray={e.kind === 'raid' ? '6 4' : undefined} />;
      })}
      {map.nodes.map((nd) => {
        const p = pos.get(nd.mint)!;
        return (
          <g key={nd.mint}>
            <circle cx={p[0]} cy={p[1]} r={18} fill="var(--panel-2)" stroke={nd.underSiege ? 'var(--bad)' : 'var(--line)'} strokeWidth={2} />
            <text x={p[0]} y={p[1] + 36} textAnchor="middle" fill="var(--dim)" fontSize={13}>{nd.symbol}</text>
          </g>
        );
      })}
    </svg>
  );
}

export default async function WarRoom() {
  const [map, feed, season, prize] = await Promise.all([
    read<WarMap>('/v1/map'), read<Page<BattleEvent>>('/v1/feed'), read<SeasonInfo | null>('/v1/seasons/current'), read<PrizeVaultInfo>('/v1/prize-vault'),
  ]);
  const raids = feed.ok ? feed.data.items.filter((e) => e.kind === 'raid').length : null;
  const sieges = feed.ok ? feed.data.items.filter((e) => e.kind === 'siege').length : null;
  const s = season.ok ? season.data : null;
  return (
    <>
      <Head eyebrow="War room" title="Every war, live" lede="Raids, sieges, counter-strikes and treaties between tokens, read from the chain as they land. Nothing here is a forecast." />
      <div className="stats" style={{ marginBottom: 16 }}>
        <Stat label="Tokens on the map" value={map.ok ? map.data.nodes.length : DASH} />
        <Stat label="Relations" value={map.ok ? map.data.edges.length : DASH} sub="raids, sieges, treaties" />
        <Stat label="Raids in feed" value={raids ?? DASH} />
        <Stat label="Sieges in feed" value={sieges ?? DASH} />
        <Stat label="Season" value={s ? `#${s.number}` : DASH} sub={s ? (s.finalized ? 'finalized' : `ends ${new Date(s.endsAt * 1000).toUTCString().slice(5, 22)}`) : 'none open on this cluster'} />
        <Stat label="Prize vault" value={prize.ok ? sol(prize.data.lamports) : DASH} sub={prize.ok && prize.data.shareBps !== null ? `${prize.data.shareBps / 100}% to the winner` : 'share to set'} />
      </div>
      <div className="grid cols-main">
        <div className="grid">
          <Panel title="Map" meta="dashed: raid · red: siege · green: treaty" flush>
            {!map.ok ? <ReadFailed what="the map" error={map.error} /> : map.data.nodes.length === 0 ? (
              <Empty title="No wars yet" what="A war starts when a token equips a Raid, Shield or Spy aimed at another token, and someone trades through it." next={<>Templates and items are in the <Link href="/armory" style={{ color: 'var(--accent)' }}>Armory</Link>.</>} />
            ) : <div className="map-wrap"><MapView map={map.data} /></div>}
          </Panel>
          <Panel title="Sieges" meta="readiness per War orders and rival">
            <Empty title="No siege is building" what="A siege becomes due when raid volume from a rival passes the War orders threshold and the interval has passed. Rivals whose kit pays holder rewards cannot be besieged (SiegeTargetHasRewards)." next="Readiness bars appear here once a token has War orders and inbound raids." />
          </Panel>
        </div>
        <div className="grid">
          <Panel title="Battle feed" meta="newest first" flush>
            {!feed.ok ? <ReadFailed what="the feed" error={feed.error} /> : feed.data.items.length === 0 ? (
              <Empty title="Quiet so far" what="Raids, sieges, forges, bounties and season results land here as the indexer reads them, each linked to its transaction." />
            ) : (
              <div className="rows">
                {feed.data.items.map((e) => (
                  <div className="row" key={`${e.signature}:${e.ordinal}`}>
                    <span className="chip accent">{KIND_LABEL[e.kind] ?? e.kind}</span>
                    <div style={{ minWidth: 0 }}><Link href={`/t/${e.mint}`}>{short(e.mint)}</Link>{e.otherMint ? <span className="faint"> vs {short(e.otherMint)}</span> : null}</div>
                    <span className="faint" style={{ fontSize: 12 }}>{ago(e.ts)}</span>
                  </div>
                ))}
              </div>
            )}
          </Panel>
          <Panel title="Under siege">
            <Empty title="Nobody is under siege" what="A token shows here while another chest's siege holds it, with the besieger and the time it ends." />
          </Panel>
        </div>
      </div>
    </>
  );
}
