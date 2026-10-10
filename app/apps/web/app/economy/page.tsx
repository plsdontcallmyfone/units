// Changed by Hookwars: new file (economy panel). Where the money in units comes from and where it
// goes, per period: protocol revenue by source, the settle waterfall, builders and holders, the
// market, crafting and the book, crankers and agents, and war money. Every figure is a sum of
// indexed chain events or chain state; nothing is estimated, converted or filled in.
import Link from 'next/link';
import type { Metadata } from 'next';
import type { ReactNode } from 'react';
import { read, type Read } from '@/lib/api';
import { ago, compact, int, short, sol as solFmt, DASH } from '@/lib/format';
import { Empty, Head, ReadFailed } from '@/components/ui';
import { Columns, SplitBar, type Series } from '@/components/econ-charts';
import {
  CRANK_LABEL, DROP_SOURCE, FEE_SOURCES, PERIODS, PERIOD_LABEL, SOURCE_LABEL, SOURCE_WHAT,
  type Builders, type Craft, type Cranks, type Market, type Period, type Revenue, type Settlement, type War,
} from '@/lib/economy';
import './economy.css';

export const metadata: Metadata = { title: 'Economy | units' };

/** The categorical palette, in its fixed order (validated: adjacent CVD and normal-vision floors
 * pass on this surface; three slots sit under 3:1, so every chart carries a table of its values). */
const C = ['#2a78d6', '#eb6834', '#1baf7a', '#eda100', '#e87ba4', '#008300', '#4a3aa7', '#e34948'];
const SOURCE_COLOR = Object.fromEntries(FEE_SOURCES.map((s, i) => [s, C[i]!])) as Record<(typeof FEE_SOURCES)[number], string>;

/** SOL with enough digits that a small sum never rounds to zero. */
const sol = (v: string | null | undefined, digits?: number): string => {
  if (v === null || v === undefined) return DASH;
  const a = Math.abs(Number(v));
  return solFmt(v, digits ?? (a === 0 ? 0 : a < 1e5 ? 9 : a < 1e8 ? 6 : 4));
};
const big = (v: string | null | undefined): bigint => (v ? BigInt(v) : 0n);
const add = (...v: (string | null | undefined)[]): string | null => (v.every((x) => x === null || x === undefined) ? null : v.reduce((a, x) => a + big(x), 0n).toString());
const num = (v: string | null | undefined): number | null => (v === null || v === undefined ? null : Number(v));

function Section({ id, title, what, guide, right, children }: { id: string; title: string; what: string; guide: string; right?: ReactNode; children: ReactNode }) {
  return (
    <section className="econ-sec" id={id} aria-labelledby={`${id}-h`}>
      <div className="econ-sec-head">
        <div>
          <h2 id={`${id}-h`}>{title}</h2>
          <p>{what} <Link href={guide} className="econ-guide">How it works</Link></p>
        </div>
        {right}
      </div>
      {children}
    </section>
  );
}

function Card({ title, meta, children, flush }: { title: string; meta?: ReactNode; children: ReactNode; flush?: boolean }) {
  return (
    <div className="panel">
      <div className="panel-head"><h2>{title}</h2>{meta ? <span className="meta">{meta}</span> : null}</div>
      <div className={flush ? 'panel-body flush' : 'panel-body'}>{children}</div>
    </div>
  );
}

function Fig({ label, value, sub }: { label: string; value: ReactNode; sub?: ReactNode }) {
  return <div className="econ-fig"><div className="label">{label}</div><div className="econ-fig-v">{value}</div>{sub ? <div className="econ-fig-s">{sub}</div> : null}</div>;
}

function Failed<T>({ r, what, children }: { r: Read<T>; what: string; children: (d: T) => ReactNode }) {
  return r.ok ? <>{children(r.data)}</> : <div className="panel"><div className="panel-body econ-pad"><ReadFailed what={what} error={r.error} /></div></div>;
}

export default async function EconomyPage({ searchParams }: { searchParams: Promise<{ period?: string }> }) {
  const sp = await searchParams;
  const period: Period = (PERIODS as readonly string[]).includes(sp.period ?? '') ? (sp.period as Period) : '7d';
  const q = `?period=${period}`;
  const [rev, set, bld, mkt, crf, crk, war] = await Promise.all([
    read<Revenue>(`/v1/economy/revenue${q}`), read<Settlement>(`/v1/economy/settlement${q}`), read<Builders>(`/v1/economy/builders${q}`),
    read<Market>(`/v1/economy/market${q}`), read<Craft>(`/v1/economy/craft${q}`), read<Cranks>(`/v1/economy/cranks${q}`), read<War>(`/v1/economy/war${q}`),
  ]);
  const scope = period === 'season' && rev.ok && rev.data.window.seasonNumber !== null ? `season ${rev.data.window.seasonNumber}` : PERIOD_LABEL[period].toLowerCase();
  const settledToHolders = set.ok ? add(set.data.quote.holderRoyalty, set.data.quote.author, set.data.quote.rent) : null;
  const bounties = crk.ok ? add(...crk.data.byKind.map((k) => k.bountyLamports)) : null;

  return (
    <div className="econ">
      <Head eyebrow="Economy" title="Where the money goes" lede="Fees the protocol takes, what items earn their holders and authors, what trades in the market and the book, and what war chests take in and spend. Every figure is summed from chain events; amounts in a token's own units are never added to SOL."
        right={<nav className="econ-periods" aria-label="Period">{PERIODS.map((p) => <Link key={p} href={`/economy?period=${p}`} className={p === period ? 'on' : ''} aria-current={p === period ? 'page' : undefined} scroll={false}>{PERIOD_LABEL[p]}</Link>)}</nav>} />

      <div className="econ-strip" role="list" aria-label={`Totals, ${scope}`}>
        <div role="listitem"><Fig label="Protocol revenue" value={rev.ok ? sol(rev.data.totalLamports) : DASH} sub={rev.ok && rev.data.totalLamports ? `${rev.data.sources.filter((s) => s.lamports).length} sources` : 'nothing recorded'} /></div>
        <div role="listitem"><Fig label="Paid to item holders and authors" value={sol(settledToHolders)} sub={set.ok ? `${int(set.data.quote.settles)} settles` : DASH} /></div>
        <div role="listitem"><Fig label="Item sales" value={mkt.ok ? sol(mkt.data.sales.volumeLamports) : DASH} sub={mkt.ok ? `${int(mkt.data.sales.count)} sold, ${int(mkt.data.activeListings)} listed now` : DASH} /></div>
        <div role="listitem"><Fig label="War chests funded" value={war.ok ? sol(war.data.inflows.funded) : DASH} sub={war.ok ? `${int(war.data.outflows.sieges)} sieges` : DASH} /></div>
        <div role="listitem"><Fig label="Crank bounties" value={sol(bounties)} sub={crk.ok ? `${int(crk.data.byKind.reduce((a, k) => a + k.runs, 0))} runs` : DASH} /></div>
      </div>

      <Section id="revenue" title="Protocol revenue" what={`What the protocol collected, by source, ${scope}. Amounts in bridged SOL or SOL.`} guide="/docs/concepts/hook-economy">
        <Failed r={rev} what="protocol revenue">{(d) => {
          const series: Series[] = FEE_SOURCES.filter((s) => d.sources.find((x) => x.source === s)?.lamports).map((s) => ({ key: s, label: SOURCE_LABEL[s], color: SOURCE_COLOR[s] }));
          return (
            <div className="econ-rev">
              <Card title="By period" meta={d.totalLamports ? `${sol(d.totalLamports)} in all` : undefined}>
                {d.series.length === 0 ? <Empty title="No protocol fees in this period" what="Fees appear once swaps, sales, licences, fills or recipes pay the protocol and the indexer has read them." next={<>Try <Link href="/economy?period=all">all time</Link>, or check the <Link href="/explorer">explorer</Link> for indexed events.</>} />
                  : <Columns label="Protocol revenue by source over time" series={series} buckets={d.series.map((b) => ({ t: b.t, values: Object.fromEntries(Object.entries(b.bySource).map(([k, v]) => [k, Number(v)])) }))} from={d.window.from} to={d.window.to} bucketSecs={d.window.bucketSecs} />}
              </Card>
              <Card title="By source" flush>
                <table className="econ-table">
                  <thead><tr><th>Source</th><th className="hide-sm">What it is</th><th className="num">Collected</th><th className="num">Events</th></tr></thead>
                  <tbody>{d.sources.map((s) => (
                    <tr key={s.source} className={s.lamports ? '' : 'dim'}>
                      <td className="nowrap"><i className="sw" style={{ background: SOURCE_COLOR[s.source] }} aria-hidden />{SOURCE_LABEL[s.source]}<div className="faint show-sm">{SOURCE_WHAT[s.source]}</div></td>
                      <td className="hide-sm faint-cell">{SOURCE_WHAT[s.source]}</td>
                      <td className="num">{sol(s.lamports)}</td><td className="num">{s.events ? int(s.events) : DASH}</td>
                    </tr>
                  ))}</tbody>
                </table>
                <div className="econ-foot">
                  {d.otherMints.length ? <span>Item-run fees in each token&apos;s own units: {d.otherMints.slice(0, 4).map((o) => <span key={o.mint + o.source}><Link href={`/t/${o.mint}`}>{short(o.mint)}</Link> {compact(o.amount)}</span>).reduce<ReactNode[]>((a, x, i) => (i ? [...a, ', ', x] : [x]), [])}{d.otherMints.length > 4 ? `, and ${d.otherMints.length - 4} more` : ''}</span> : <span>No item-run fees in this period; they are paid in each token&apos;s own units.</span>}
                  {d.configTotals.book || d.configTotals.craft ? <span>Lifetime totals the programs keep: order book {sol(d.configTotals.book)}, recipes {sol(d.configTotals.craft)}.</span> : null}
                </div>
              </Card>
            </div>
          );
        }}</Failed>
      </Section>

      <Section id="settlement" title="Settlement waterfall" what="Where the SOL side of every settled item cut went: the template author first, then rent to a lessor, the holder's royalty, the settler's bounty, and the item's own destinations." guide="/docs/concepts/how-a-trade-runs">
        <Failed r={set} what="settlements">{(d) => (
          <div className="grid cols-2">
            <Card title="SOL side of settles" meta={d.quote.settles ? `${int(d.quote.settles)} settles` : undefined}>
              {d.quote.settles === 0 ? <Empty title="No settles in this period" what="A settle pays out what an equipped item collected. Anyone can run one from a token's slot page." next={<Link href="/projects">Find a token with equipped items</Link>} /> : (
                <SplitBar label="Settled SOL by destination" parts={[
                  { key: 'author', label: 'Template authors', what: 'The author share of each royalty', color: C[0]!, value: num(d.quote.author) },
                  { key: 'rent', label: 'Lessors', what: 'Rent on leased items', color: C[1]!, value: num(d.quote.rent) },
                  { key: 'holder', label: 'Item holders', what: 'Royalty left after author and rent', color: C[2]!, value: num(d.quote.holderRoyalty) },
                  { key: 'bounty', label: 'Settlers', what: 'Bounty to whoever ran the settle', color: C[3]!, value: num(d.quote.bounty) },
                  { key: 'dest', label: 'Item destinations', what: 'Collectors, partners and chests the item names', color: C[4]!, value: num(d.quote.destinations) },
                ]} />
              )}
              <p className="econ-note">The DEX took {sol(d.dexShare)} as its protocol share on the same pools before any cut reached an item. The token-side protocol share is in the table beside, in each token&apos;s units.</p>
            </Card>
            <Card title="Token side, per token" meta="in each token's own units" flush>
              {d.tokenSide.length === 0 ? <Empty title="No token-side settles" what="Token-side cuts are paid in the token itself, so they are listed per token and never added together." /> : (
                <table className="econ-table">
                  <thead><tr><th>Token</th><th className="num">Settles</th><th className="num">Protocol</th><th className="num">Royalty</th><th className="num">Destinations</th><th className="num">Burned</th></tr></thead>
                  <tbody>{d.tokenSide.map((t) => (
                    <tr key={t.mint}><td><Link href={`/t/${t.mint}`}>{short(t.mint)}</Link></td><td className="num">{int(t.settles)}</td><td className="num">{int(t.protocol)}</td><td className="num">{int(add(t.holderRoyalty, t.author, t.rent))}</td><td className="num">{int(t.destinations)}</td><td className="num">{int(t.burned)}</td></tr>
                  ))}</tbody>
                </table>
              )}
            </Card>
          </div>
        )}</Failed>
      </Section>

      <Section id="builders" title="Builders and holders" what="Who earns from hooks: template authors by their share, holders by royalties claimed and licence income, and the items that moved the most." guide="/docs/concepts/hooks-and-items">
        <Failed r={bld} what="builders">{(d) => (
          <div className="econ-three">
            <Card title="Templates by author share" flush>
              {d.templates.length === 0 ? <Empty title="No author shares paid" what="A template's author earns a share of every royalty its items collect, once settles run." /> : (
                <table className="econ-table"><thead><tr><th>Template</th><th className="num">Earned</th></tr></thead>
                  <tbody>{d.templates.map((t) => <tr key={t.templateId}><td><Link href={`/armory/templates/${t.templateId}`}>{t.name ?? `Template ${t.templateId}`}</Link><div className="faint">{int(t.payments)} payments</div></td><td className="num">{sol(t.authorLamports)}</td></tr>)}</tbody></table>
              )}
            </Card>
            <Card title="Holders by income" flush>
              {d.holders.length === 0 ? <Empty title="No royalties claimed" what="Holders claim royalties in bridged SOL; licence income is paid at purchase." /> : (
                <table className="econ-table"><thead><tr><th>Wallet</th><th className="num">Royalties</th><th className="num">Licences</th></tr></thead>
                  <tbody>{d.holders.map((h) => <tr key={h.wallet}><td><Link href={`/u/${h.wallet}`} className="addr">{short(h.wallet)}</Link></td><td className="num">{sol(h.royaltyLamports)}</td><td className="num">{sol(h.licenceLamports)}</td></tr>)}</tbody></table>
              )}
            </Card>
            <Card title="Items by SOL settled" flush>
              {d.items.length === 0 ? <Empty title="No items settled" what="Items appear once a settle pays out their SOL-side cuts." /> : (
                <table className="econ-table"><thead><tr><th>Item</th><th className="num">Settled</th></tr></thead>
                  <tbody>{d.items.map((it) => <tr key={it.item}><td><Link href={`/armory/items/${it.item}`}>{it.name ?? 'Item'}</Link><div className="faint">{short(it.item)}{it.owner ? <> held by {short(it.owner)}</> : null}</div></td><td className="num">{sol(it.settledLamports)}<div className="faint">{int(it.settles)} settles</div></td></tr>)}</tbody></table>
              )}
            </Card>
          </div>
        )}</Failed>
      </Section>

      <Section id="market" title="Market" what="Item sales, listings by template, licences, leases and commissions. Sale prices are the only price history shown." guide="/docs/concepts/hook-economy">
        <Failed r={mkt} what="the market">{(d) => (
          <>
            <div className="econ-figs">
              <Fig label="Sales" value={int(d.sales.count)} sub={sol(d.sales.volumeLamports)} />
              <Fig label="Market fees" value={sol(d.sales.feeLamports)} sub={d.sales.resaleLamports ? `authors got ${sol(d.sales.resaleLamports)} on resale` : 'no resale shares'} />
              <Fig label="Licences live" value={int(d.licences.live)} sub={d.licences.bought ? `${int(d.licences.bought)} bought for ${sol(d.licences.incomeLamports)}` : 'none bought this period'} />
              <Fig label="Leases active" value={int(d.leases.active)} sub={d.leases.feesLamports || d.leases.rentLamports ? `fees ${sol(d.leases.feesLamports)}, rent ${sol(d.leases.rentLamports)}` : 'no fees or rent this period'} />
              <Fig label="Commissions open" value={int(d.commissions.open)} sub={d.commissions.openBountyLamports ? `${sol(d.commissions.openBountyLamports)} in bounties` : 'no open bounties'} />
            </div>
            <div className="grid cols-main">
              <Card title="By template" meta={`${int(d.activeListings)} active listings`} flush>
                {d.classes.length === 0 ? <Empty title="No listings or sales yet" what="A template shows here once one of its items is listed or sold." next={<Link href="/marketplace">Open the marketplace</Link>} /> : (
                  <table className="econ-table">
                    <thead><tr><th>Template</th><th className="num">Listed</th><th className="num">Floor</th><th className="num">Last sale</th><th className="num">Sales</th></tr></thead>
                    <tbody>{d.classes.map((c) => <tr key={c.templateId}><td><Link href={`/armory/templates/${c.templateId}`}>{c.name ?? `Template ${c.templateId}`}</Link></td><td className="num">{int(c.listings)}</td><td className="num">{sol(c.floorLamports)}</td><td className="num">{sol(c.lastSaleLamports)}{c.lastSaleTs ? <div className="faint">{ago(c.lastSaleTs)}</div> : null}</td><td className="num">{int(c.sales)}</td></tr>)}</tbody>
                  </table>
                )}
              </Card>
              <Card title="Licences and commissions" flush>
                <table className="econ-table">
                  <tbody>
                    <tr><td>Licence income<div className="faint">{d.licences.bought ? `${int(d.licences.bought)} bought this period` : 'none bought this period'}</div></td><td className="num">{sol(d.licences.incomeLamports)}</td></tr>
                    <tr><td className="ind">to the protocol</td><td className="num">{sol(d.licences.protocolLamports)}</td></tr>
                    <tr><td className="ind">to template authors</td><td className="num">{sol(d.licences.authorLamports)}</td></tr>
                    <tr><td className="ind">to item holders</td><td className="num">{sol(d.licences.holderLamports)}</td></tr>
                    <tr><td>Commissions paid<div className="faint">{int(d.commissions.refunded)} refunded</div></td><td className="num">{sol(d.commissions.paidLamports)}<div className="faint">{int(d.commissions.paid)} paid</div></td></tr>
                  </tbody>
                </table>
              </Card>
            </div>
          </>
        )}</Failed>
      </Section>

      <Section id="craft" title="Crafting and the order book" what="Materials against their season caps, what dropped and what recipes burned, the books for each material, and wear on items." guide="/docs/concepts/hook-economy">
        <Failed r={crf} what="crafting">{(d) => (
          <>
            <div className="econ-figs">
              <Fig label="Crafts" value={int(d.recipes.crafts)} sub={`${int(d.recipes.repairs)} repairs`} />
              <Fig label="Recipe fees" value={sol(d.recipes.feesLamports)} sub="protocol and season pool" />
              <Fig label="Class fills" value={int(d.classFills.count)} sub={d.classFills.volumeLamports ? sol(d.classFills.volumeLamports) : 'no class bids filled'} />
              <Fig label="Items dormant" value={int(d.wear.dormant)} sub={`of ${int(d.wear.tracked)} that wear`} />
            </div>
            {!d.chain && !d.bookChain ? <div className="panel"><div className="panel-body econ-pad"><Empty title="Material supply and the books could not be read" what="Supply, season caps and resting orders live on the chain, and the backend could not read the craft and book programs just now." next={<>Drops and fills above are from indexed events. <Link href="/craft">Craft</Link> and the <Link href="/book">order book</Link> read the chain directly.</>} /></div></div> : (
            <div className="grid cols-2">
              <Card title="Materials" meta="supply against the season cap" flush>
                {!d.chain ? <Empty title="Materials could not be read" what="The backend could not read the craft program just now; drops above are from indexed events." /> : d.materials.length === 0 ? <Empty title="No materials" what="The admin creates materials behind the timelock, each with a season cap." next={<Link href="/craft">Craft</Link>} /> : (
                  <table className="econ-table">
                    <thead><tr><th>Material</th><th>This season</th><th className="num">Dropped</th><th className="num">Burned</th><th className="num">Net</th></tr></thead>
                    <tbody>{d.materials.map((m) => {
                      const cap = Number(m.cap), used = Number(m.emittedThisSeason), pct = cap > 0 ? Math.min(100, (used / cap) * 100) : 0;
                      return (
                        <tr key={m.id}>
                          <td>{m.name}<div className="faint">{int(m.emittedTotal)} ever, {int(m.burnedTotal)} burned</div></td>
                          <td><div className="econ-meter" role="meter" aria-valuemin={0} aria-valuemax={cap} aria-valuenow={used} aria-label={`${m.name}: ${used} of ${cap} this season`}><span style={{ width: `${pct}%` }} /></div><div className="faint">{int(m.emittedThisSeason)} of {int(m.cap)}</div></td>
                          <td className="num">{int(m.dropped)}</td><td className="num">{int(m.burnedByRecipes)}</td><td className="num">{m.net === null ? DASH : `${Number(m.net) > 0 ? '+' : ''}${int(m.net)}`}</td>
                        </tr>
                      );
                    })}</tbody>
                  </table>
                )}
                {d.dropsBySource.length ? <div className="econ-foot"><span>Drops by source: {d.dropsBySource.map((x) => `${DROP_SOURCE[x.source] ?? `source ${x.source}`} ${int(x.amount)} of material ${x.materialId}`).join('; ')}</span><span>Burned counts this period&apos;s recipe uses at each recipe&apos;s current inputs.</span></div> : null}
              </Card>
              <Card title="Order books" meta="resting orders from the chain" flush>
                {!d.bookChain ? <Empty title="The books could not be read" what="The backend could not read the book program just now, so resting orders and depth are not shown." next={<Link href="/book">Order book</Link>} /> : d.books.length === 0 ? <Empty title="No material books" what="A book opens for a material once someone creates its market." next={<Link href="/book">Order book</Link>} /> : (
                  <table className="econ-table">
                    <thead><tr><th>Material</th><th className="num">Bid</th><th className="num">Ask</th><th className="num">Depth</th><th className="num">Fills</th></tr></thead>
                    <tbody>{d.books.map((b) => (
                      <tr key={b.market}>
                        <td><Link href={`/book?material=${b.materialId}`}>{b.name ?? `Material ${b.materialId}`}</Link><div className="faint">{b.lastPrice ? <>last {sol(b.lastPrice, 9)} {ago(b.lastTs)}</> : 'no fills yet'}</div></td>
                        <td className="num">{sol(b.bestBid, 9)}</td><td className="num">{sol(b.bestAsk, 9)}{b.spread ? <div className="faint">spread {sol(b.spread, 9)}</div> : null}</td>
                        <td className="num">{sol(b.bidDepth)}<div className="faint">{sol(b.askDepth)} asked</div></td>
                        <td className="num">{int(b.fills)}<div className="faint">{sol(b.volumeLamports)}</div></td>
                      </tr>
                    ))}</tbody>
                  </table>
                )}
              </Card>
            </div>
            )}
          </>
        )}</Failed>
      </Section>

      <Section id="cranks" title="Crankers and agents" what="Bounties paid to whoever runs the permissionless steps, the crankers named by their events, and what agents' passports have recorded." guide="/docs/concepts/agents">
        <Failed r={crk} what="crank bounties">{(d) => (
          <div className="econ-three">
            <Card title="Bounties by step" flush>
              {d.byKind.some((k) => k.runs) ? (
                <table className="econ-table"><thead><tr><th>Step</th><th className="num">Runs</th><th className="num">Bounties</th></tr></thead>
                  <tbody>{d.byKind.filter((k) => k.runs).map((k) => <tr key={k.kind}><td>{CRANK_LABEL[k.kind] ?? k.kind}</td><td className="num">{int(k.runs)}</td><td className="num">{sol(k.bountyLamports)}</td></tr>)}</tbody></table>
              ) : <Empty title="No cranks ran" what="Settles, sieges, razes, prizes, order expiries and companion steps pay whoever runs them." />}
              {d.byKind.some((k) => !k.runs) ? <div className="econ-foot"><span>Not run this period: {d.byKind.filter((k) => !k.runs).map((k) => (CRANK_LABEL[k.kind] ?? k.kind).toLowerCase()).join(', ')}.</span></div> : null}
            </Card>
            <Card title="Top crankers" meta="settles name no cranker" flush>
              {d.top.length === 0 ? <Empty title="No named crankers" what="Sieges, razes, prizes, expiries and companion cranks name who ran them." /> : (
                <table className="econ-table"><thead><tr><th>Wallet</th><th className="num">Runs</th><th className="num">Earned</th></tr></thead>
                  <tbody>{d.top.map((t) => <tr key={t.cranker}><td><Link href={`/u/${t.cranker}`} className="addr">{short(t.cranker)}</Link></td><td className="num">{int(t.runs)}</td><td className="num">{sol(t.bountyLamports)}</td></tr>)}</tbody></table>
              )}
            </Card>
            <Card title="Agents" meta="as each program records it" flush>
              {d.agents.length === 0 ? <Empty title="No agent records" what="Programs credit an agent's passport when its key authors, claims, cranks or reveals." next={<Link href="/agents">Agents</Link>} /> : (
                <table className="econ-table"><thead><tr><th>Agent</th><th className="num">Royalties</th><th className="num">Bounties</th></tr></thead>
                  <tbody>{d.agents.map((a) => <tr key={a.passport}><td><Link href={`/agents/${a.passport}`}>{a.name ?? short(a.passport)}</Link><div className="faint">{int(a.records)} records{a.byKind.crank ? `, crank value ${sol(a.byKind.crank)}` : ''}</div></td><td className="num">{sol(a.byKind.royaltyClaim ?? null)}</td><td className="num">{sol(a.byKind.bounty ?? null)}</td></tr>)}</tbody></table>
              )}
            </Card>
          </div>
        )}</Failed>
      </Section>

      <Section id="war" title="War money" what="What war chests took in and what they spent: funding from fees, treaty shares and coalition pots against siege spend, raze proceeds, the boss pool and the season prize." guide="/docs/concepts/war">
        <Failed r={war} what="war money">{(d) => (
          <>
            <div className="econ-figs">
              <Fig label="Chests funded" value={sol(d.inflows.funded)} sub={`${int(d.inflows.fundings)} fundings`} />
              <Fig label="Siege spend" value={sol(d.outflows.siegeSpent)} sub={d.outflows.counterStrikeSpent ? `counter-strikes ${sol(d.outflows.counterStrikeSpent)}` : `${int(d.outflows.sieges)} sieges, no counter-strikes`} />
              <Fig label="Raze proceeds" value={sol(d.outflows.razeProceeds)} sub={`${int(d.outflows.razes)} razes`} />
              <Fig label="Boss pool" value={sol(d.boss.funded)} sub={d.boss.claimed ? `claimed ${sol(d.boss.claimed)}` : 'nothing claimed'} />
              <Fig label="Season prize" value={sol(d.prize.toWinner)} sub={d.prize.paid ? `treasury ${sol(d.prize.toTreasury)}` : 'no prize paid'} />
            </div>
            <div className="grid cols-main">
              <Card title="Funded against siege spend">
                {d.series.length === 0 ? <Empty title="No war money moved in this period" what="Chests fill from fees routed by a token's companion and from treaty shares; sieges spend from them." next={<Link href="/war">War room</Link>} />
                  : <Columns label="War chests funded and siege spend over time" mode="group" series={[{ key: 'funded', label: 'Funded', color: C[0]! }, { key: 'spent', label: 'Siege spend', color: C[1]! }]} buckets={d.series.map((s) => ({ t: s.t, values: { funded: Number(s.funded), spent: Number(s.spent) } }))} from={d.window.from} to={d.window.to} bucketSecs={d.window.bucketSecs} />}
                <div className="econ-legend-row" aria-hidden><span><i style={{ background: C[0] }} />Funded</span><span><i style={{ background: C[1] }} />Siege spend</span></div>
                <dl className="econ-kv">
                  <div><dt>Routed by companions</dt><dd>{sol(d.inflows.fromCompanions)}</dd></div>
                  <div><dt>Treaty shares</dt><dd>{sol(d.inflows.treatyShared)}</dd></div>
                  <div><dt>Put into coalitions</dt><dd>{sol(d.inflows.coalitionContributed)}</dd></div>
                  <div><dt>Coalition siege spend</dt><dd>{sol(d.outflows.coalitionSiegeSpent)}</dd></div>
                  <div><dt>Raid bounties paid</dt><dd>{sol(d.outflows.bountiesPaid)}</dd></div>
                </dl>
              </Card>
              <Card title="Chests" flush>
                {d.chests.length === 0 ? <Empty title="No chest moved" what="A token's chest shows once it is funded or spends." /> : (
                  <table className="econ-table"><thead><tr><th>Token</th><th className="num">Funded</th><th className="num">Spent</th></tr></thead>
                    <tbody>{d.chests.map((c) => <tr key={c.mint}><td><Link href={`/t/${c.mint}`}>{short(c.mint)}</Link></td><td className="num">{sol(c.fundedLamports)}</td><td className="num">{sol(c.siegeSpentLamports)}{c.razeLamports !== '0' ? <div className="faint">raze {sol(c.razeLamports)}</div> : null}</td></tr>)}</tbody></table>
                )}
              </Card>
            </div>
          </>
        )}</Failed>
      </Section>
    </div>
  );
}
