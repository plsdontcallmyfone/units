// Changed by Hookwars: new page, a wallet's public profile: holdings, items owned, royalties, levels and skills, badges, guilds, agents, posts.
import { Fragment } from 'react';
import Link from 'next/link';
import { readSocial, type Feed } from '@/lib/social';
import { compact, int, short, sol } from '@/lib/format';
import { at, AGENT_STATUS, PROOF, when } from '@/lib/labels';
import { Empty, Head, Panel, ReadFailed, Stat } from '@/components/ui';
import { PostCard } from '@/components/post-card';
import { FollowButton, OpenProfile } from '@/components/social-buttons';
import s from '@/components/social.module.css';

type Skill = { id: number; name: string; counter: string; value: string; level: number; next: string | null };
type Profile = {
  wallet: string;
  levels: { profile: string | null; openedAt?: number; counters: Record<string, string> | null; skills: Skill[] | null; reason: string | null };
  holdings: { holding: string; mint: string; amount: string; voteLocked: string; name: string | null; symbol: string | null }[];
  guilds: { id: number; name: string | null; roles: string[]; depositedLamports: string | null }[];
  passports?: { passport: string; name: string; proof: number; status: number; role: string }[];
  itemsOwned?: { item: string; item_mint: string; template_id: number; template_name: string | null; level: number; author: string }[];
  authored?: { items: number; adoptedByMints: number };
  royalties?: { settledToAuthoredItemsLamports: string | null; claimsByMint: { cut_mint: string; claims: number; amount: string }[] };
  raids?: { count: number; volumeLamports: string | null };
  badges?: { signature: string; slot: string; ts: string; id: number }[];
  follows?: { followers: number; following: number };
  posts?: Feed;
  reason?: string;
};
const ADDR = /^[1-9A-HJ-NP-Za-km-z]{32,44}$/;
const COUNTER_LABEL: Record<string, string> = {
  itemsAuthored: 'Items authored', templatesRegistered: 'Templates registered', licencesSold: 'Licences sold', licenceRevenueLamports: 'Licence revenue',
  itemsSold: 'Items sold', itemsCrafted: 'Items crafted', repairs: 'Repairs', bookFills: 'Book fills', treatiesHeld: 'Treaties held', raids: 'Raids',
};

export default async function WalletPage({ params }: { params: Promise<{ wallet: string }> }) {
  const { wallet } = await params;
  if (!ADDR.test(wallet)) return <><Head eyebrow="Profile" title="Not an address" /><Panel title="Profile"><Empty title="Not a wallet address" what="A profile lives at /u/ followed by a Solana address." /></Panel></>;
  const r = await readSocial<Profile>(`/v1/u/${wallet}`);
  if (!r.ok) return <><Head eyebrow="Profile" title={short(wallet, 6)} /><Panel title="Profile"><ReadFailed what="this wallet" error={r.error} /></Panel></>;
  const p = r.data;
  const now = Date.now() / 1000;
  const skills = p.levels.skills ?? [];
  return (
    <>
      <Head eyebrow="Profile" title={short(wallet, 6)} lede={<span className={s.wrapText}>{wallet}</span>}
        right={<FollowButton target={wallet} />} />
      {p.reason ? <p className="muted">{p.reason}</p> : null}
      <div className="stats">
        <Stat label="Followers" value={int(p.follows?.followers)} />
        <Stat label="Following" value={int(p.follows?.following)} />
        <Stat label="Items authored" value={int(p.authored?.items)} sub={p.authored ? `on ${int(p.authored.adoptedByMints)} tokens` : undefined} />
        <Stat label="Royalties to its items" value={sol(p.royalties?.settledToAuthoredItemsLamports, 4)} sub="settled on chain" />
        <Stat label="Raid volume" value={sol(p.raids?.volumeLamports, 3)} sub={p.raids ? `${int(p.raids.count)} raids` : undefined} />
      </div>
      <div className="grid cols-main">
        <Panel title="Levels and skills" meta={p.levels.profile ? `profile opened ${when(p.levels.openedAt)}` : 'no profile'}>
          {!p.levels.profile ? <Empty title="No profile yet" what={p.levels.reason ?? 'Counters start when the wallet opens a profile.'} next={<OpenProfile wallet={wallet} />} /> : (
            <>
              {skills.length === 0 ? <p className="muted">{p.levels.reason ?? 'No skills defined on this cluster.'}</p> : (
                <div className={s.skills}>
                  {skills.map((k) => {
                    const v = Number(k.value); const nx = k.next ? Number(k.next) : null;
                    return (
                      <div key={k.id} className={s.skill}>
                        <span className="muted" style={{ textTransform: 'capitalize' }}>{k.name}</span>
                        <b>Level {k.level}</b>
                        <span className="faint" style={{ fontSize: 13 }}>{COUNTER_LABEL[k.counter] ?? k.counter}: {int(k.value)}{nx ? `, next at ${int(nx)}` : ', top level'}</span>
                        {nx ? <div className={s.bar} aria-hidden><span style={{ width: `${Math.min(100, (v / nx) * 100)}%` }} /></div> : null}
                      </div>
                    );
                  })}
                </div>
              )}
              {p.levels.counters ? (
                <dl className="kv" style={{ marginTop: 16 }}>
                  {Object.entries(p.levels.counters).map(([k, v]) => <Fragment key={k}><dt>{COUNTER_LABEL[k] ?? k}</dt><dd>{k === 'licenceRevenueLamports' ? sol(v, 4) : int(v)}</dd></Fragment>)}
                </dl>
              ) : null}
            </>
          )}
        </Panel>
        <Panel title="Agents" meta="passports this wallet runs">
          {!p.passports?.length ? <Empty title="No agents" what="A wallet that registers a passport, or is an agent's key, shows its agents here." /> : (
            <div className="rows">{p.passports.map((a) => (
              <div className="row" key={a.passport}>
                <span className="chip">{a.role}</span>
                <Link href={`/agents/${a.passport}/timeline`}>{a.name || short(a.passport, 4)}</Link>
                <span className="faint">{PROOF[a.proof]?.name ?? a.proof} · {at(AGENT_STATUS, a.status)}</span>
              </div>
            ))}</div>
          )}
        </Panel>
      </div>
      <div style={{ height: 16 }} />
      <div className="grid cols-2">
        <Panel title="Holdings" meta="token balances on chain" flush>
          {!p.holdings.length ? <Empty title="No holdings" what="Balances of tokens launched on units that this wallet holds appear here." /> : (
            <table><thead><tr><th>Token</th><th className="num">Amount</th><th className="num">Vote locked</th></tr></thead>
              <tbody>{p.holdings.map((h) => <tr key={h.holding}><td><Link href={`/t/${h.mint}`}>{h.symbol ?? short(h.mint, 4)}</Link>{h.name ? <span className="faint"> {h.name}</span> : null}</td><td className="num">{compact(h.amount)}</td><td className="num">{compact(h.voteLocked)}</td></tr>)}</tbody></table>
          )}
        </Panel>
        <Panel title="Items owned" flush>
          {!p.itemsOwned?.length ? <Empty title="No items" what="Items this wallet holds, from the armory's indexed history." /> : (
            <table><thead><tr><th>Item</th><th>Template</th><th className="num">Level</th><th>Author</th></tr></thead>
              <tbody>{p.itemsOwned.map((i) => <tr key={i.item}><td><Link href={`/marketplace/items/${i.item_mint}`}>{short(i.item_mint, 4)}</Link></td><td>{i.template_name ?? `template ${i.template_id}`}</td><td className="num">{i.level}</td><td>{i.author === wallet ? 'self' : <Link href={`/u/${i.author}`}>{short(i.author, 4)}</Link>}</td></tr>)}</tbody></table>
          )}
        </Panel>
        <Panel title="Royalty claims" meta="by the token the cut was paid in" flush>
          {!p.royalties?.claimsByMint.length ? <Empty title="No royalty claims" what="Claims of royalties an item earned, as RoyaltyClaimed events." /> : (
            <table><thead><tr><th>Token</th><th className="num">Claims</th><th className="num">Amount</th></tr></thead>
              <tbody>{p.royalties.claimsByMint.map((c) => <tr key={c.cut_mint}><td><Link href={`/t/${c.cut_mint}`}>{short(c.cut_mint, 4)}</Link></td><td className="num">{int(c.claims)}</td><td className="num">{compact(c.amount)}</td></tr>)}</tbody></table>
          )}
        </Panel>
        <Panel title="Guilds and badges">
          {!p.guilds.length && !p.badges?.length ? <Empty title="No guilds or badges" what="Guilds this wallet is an officer of, founded or paid into, and badges awarded to it." /> : (
            <div className="rows">
              {p.guilds.map((g) => <div className="row" key={`g${g.id}`}><span className="chip">guild</span><Link href={`/guilds/${g.id}`}>{g.name ?? `guild ${g.id}`}</Link><span className="faint">{g.roles.join(', ')}{g.depositedLamports ? `, paid in ${sol(g.depositedLamports, 3)}` : ''}</span></div>)}
              {(p.badges ?? []).map((b) => <div className="row" key={b.signature}><span className="chip ok">badge</span><Link href="/badges">badge {b.id}</Link><span className="faint">{when(b.ts)}</span></div>)}
            </div>
          )}
        </Panel>
      </div>
      <div style={{ height: 16 }} />
      <Panel title="Posts" meta="signed by this wallet">
        {!p.posts ? <Empty title="Posts not read" what="The indexer database is not reachable." /> : p.posts.items.length === 0 ? <Empty title="No posts" what="Posts this wallet signs appear here once the indexer reads them." next={<Link href="/feed">Go to the feed</Link>} /> : p.posts.items.map((x) => <PostCard key={x.id} p={x} now={now} />)}
      </Panel>
    </>
  );
}
