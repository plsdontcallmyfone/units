// Changed by Hookwars: new page, achievement badges (10 section 3): each badge type, its criterion
// and how many have been awarded; recent awards from the indexer. A badge is soulbound and pays nothing.
import Link from 'next/link';
import { read } from '@/lib/api';
import { int, short } from '@/lib/format';
import { when } from '@/lib/labels';
import { Empty, Head, Panel, ReadFailed } from '@/components/ui';
import { Action } from '@/components/action';

type Criterion = { name: 'FirstSiege'; mint: string } | { name: 'RaidPoints'; mint: string; min: number } | { name: 'ForgeLevel'; minLevel: number };
type Badge = { badge: string; id: number; mint: string; name: string; criterion: Criterion; claimsOpenAt: string; awarded: string };
type Award = { signature: string; ts: string | number; id: number; recipient: string; claimant: string };

function rule(c: Criterion) {
  if (c.name === 'FirstSiege') return <>first to lay siege from <Link href={`/t/${c.mint}`}>{short(c.mint)}</Link></>;
  if (c.name === 'RaidPoints') return <>{int(c.min)} raid points on <Link href={`/t/${c.mint}`}>{short(c.mint)}</Link></>;
  return <>forged an item to level {c.minLevel}</>;
}

export default async function Badges() {
  const r = await read<{ items: Badge[]; recent: Award[] }>('/v1/badges');
  return (
    <>
      <Head eyebrow="Badges" title="Achievement badges" lede="A badge is a soulbound token minted to whoever meets its criterion on chain. It cannot be sold and confers nothing but the record." />
      <div className="grid cols-main">
        <Panel title="Badge types" meta={r.ok ? `${r.data.items.length}` : undefined} flush>
          {!r.ok ? <ReadFailed what="badges" error={r.error} /> : r.data.items.length === 0 ? <Empty title="No badge types on this cluster" what="The social program's admin creates badge types; each lists its criterion here." /> : (
            <table><thead><tr><th>#</th><th>Badge</th><th>Criterion</th><th className="r">Awarded</th><th>Claims open</th></tr></thead>
              <tbody>{r.data.items.map((b) => <tr key={b.badge}><td>{b.id}</td><td>{b.name}</td><td className="muted">{rule(b.criterion)}</td><td className="r">{int(b.awarded)}</td><td className="faint">{when(b.claimsOpenAt)}</td></tr>)}</tbody></table>
          )}
        </Panel>
        <Panel title="Claim a badge">
          <Action route="badges/claim" title="Claim" cta="Claim badge" what="The program checks the criterion against your on-chain record; a claim that does not meet it fails without cost beyond the fee." fields={[{ name: 'badgeId', label: 'Badge id', kind: 'int' }, { name: 'recipient', label: 'Recipient', kind: 'key', optional: true, hint: 'empty for your wallet' }]} />
        </Panel>
      </div>
      <div style={{ height: 16 }} />
      <Panel title="Recent awards" flush>
        {!r.ok ? <ReadFailed what="awards" error={r.error} /> : r.data.recent.length === 0 ? <Empty title="No awards indexed yet" what="Each award lands here as the indexer reads the BadgeAwarded event." /> : (
          <table><thead><tr><th>Badge</th><th>Recipient</th><th>When</th></tr></thead>
            <tbody>{r.data.recent.map((a) => <tr key={`${a.signature}:${a.recipient}`}><td>{a.id}</td><td>{short(a.recipient)}</td><td className="faint">{when(a.ts)}</td></tr>)}</tbody></table>
        )}
      </Panel>
    </>
  );
}
