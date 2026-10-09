import type { PrizeVaultInfo, SeasonInfo } from '@hookwars/shared';
import { read } from '@/lib/api';
import { sol, short } from '@/lib/format';
import { Empty, Head, Panel, ReadFailed, Stat } from '@/components/ui';

const WEIGHT_LABEL: Record<string, string> = { raidVolumeWon: 'Raid volume won', sieges: 'Sieges', siegeSpend: 'Siege spend', timesBesieged: 'Times besieged', counterStrikes: 'Counter-strikes', treatySecs: 'Treaty seconds' };

export default async function Seasons() {
  const [cur, prize] = await Promise.all([read<SeasonInfo | null>('/v1/seasons/current'), read<PrizeVaultInfo>('/v1/prize-vault')]);
  const s = cur.ok ? cur.data : null;
  return (
    <>
      <Head eyebrow="Seasons" title="King of the hill, on chain" lede="A season scores each token from its own war counters with weights published before it opens. After it ends anyone submits the leader; anyone can challenge with a higher score. The winner's chest gets a share of the protocol fees collected during the next season." />
      <div className="stats" style={{ marginBottom: 16 }}>
        <Stat label="Season" value={s ? `#${s.number}` : '-'} sub={s ? (s.finalized ? 'finalized' : 'running') : 'none open'} />
        <Stat label="Leader" value={s?.leader ? short(s.leader.mint) : '-'} sub={s?.leader ? `score ${s.leader.score}` : undefined} />
        <Stat label="Prize vault" value={prize.ok ? sol(prize.data.lamports) : '-'} sub={prize.ok ? short(prize.data.vault) : undefined} />
        <Stat label="Last winner" value={prize.ok && prize.data.lastWinner ? short(prize.data.lastWinner) : '-'} />
      </div>
      <div className="grid cols-main">
        <Panel title="Current season" flush>
          {!cur.ok ? <ReadFailed what="the season" error={cur.error} /> : !s ? (
            <Empty title="No season is open on this cluster" what="An admin proposes a season with its dates and weights behind the timelock; once its start passes, anyone opens it." />
          ) : (
            <table><thead><tr><th>Counter</th><th className="num">Weight</th></tr></thead>
              <tbody>{Object.entries(s.weights).map(([k, v]) => <tr key={k}><td>{WEIGHT_LABEL[k] ?? k}{k === 'timesBesieged' && s.penalizeBesieged ? <span className="faint"> (subtracted)</span> : null}</td><td className="num">{v}</td></tr>)}</tbody></table>
          )}
        </Panel>
        <Panel title="The prize">
          <p className="muted" style={{ margin: 0 }}>A share (SEASON_PRIZE_SHARE_BPS{prize.ok && prize.data.shareBps !== null ? `, ${prize.data.shareBps / 100}%` : ', to set'}) of the protocol's fees collected during the next season, paid to the winner's war chest. Nothing pays on who wins a war or a siege.</p>
        </Panel>
      </div>
    </>
  );
}
