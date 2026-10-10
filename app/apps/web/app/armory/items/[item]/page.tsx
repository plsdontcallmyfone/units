import Link from 'next/link';
import type { BattleEvent, ItemSummary } from '@hookwars/shared';
import { read } from '@/lib/api';
import { short } from '@/lib/format';
import { Empty, Head, Panel, ReadFailed } from '@/components/ui';
import { Action, Actions } from '@/components/action';
import { AccessPanel } from '@/components/access-panel';

type Wear = { maxCharges: number; used: number; dormant: boolean; repairs: number };

export default async function ItemPage({ params }: { params: Promise<{ item: string }> }) {
  const { item } = await params;
  const r = await read<ItemSummary & { history: BattleEvent[] }>(`/v1/items/${item}`);
  if (!r.ok) return <><Head eyebrow="Item" title={short(item, 6)} /><Panel title="Item"><ReadFailed what="this item" error={r.error} /></Panel></>;
  const it = r.data;
  // Changed by Hookwars (app pass v3): craft wear (13 E-3); null for an item that does not wear.
  const w = await read<Wear | null>(`/v1/items/${it.item}/wear`);
  const wear = w.ok ? w.data : null;
  return (
    <>
      <Head eyebrow={`${it.templateName} · level ${it.level}`} right={wear?.dormant ? <span className="chip warn" title="Worn out: every hook call gets its default answer until it is repaired">dormant</span> : undefined} title={it.paramsText} lede={`Royalty ${it.royaltyBps / 100}% of what it collects, paid to whoever holds it.`} />
      <div className="grid cols-main">
        <Panel title="Record">
          <dl className="kv">
            <dt>Item</dt><dd className="addr">{it.item}</dd>
            <dt>Owner</dt><dd className="addr">{it.owner ?? '-'}</dd>
            <dt>Author</dt><dd className="addr">{it.author}</dd>
            <dt>Source</dt><dd>{it.source}</dd>
            <dt>Params</dt><dd>{it.params.join(', ')}</dd>
            <dt>Wear</dt><dd>{!w.ok ? 'could not be read' : !wear ? 'does not wear' : `${wear.used} of ${wear.maxCharges} charges used${wear.repairs ? `, repaired ${wear.repairs} times` : ''}${wear.dormant ? ', dormant' : ''}`}</dd>
            <dt>Equipped on</dt><dd>{it.equippedOn.length ? it.equippedOn.map((e) => <Link key={e.mint} href={`/t/${e.mint}`}>{e.symbol} </Link>) : 'no token'}</dd>
          </dl>
        </Panel>
        <Panel title="Royalty position" meta="unsettled, settles to royalty, claimable">
          {it.royalties.length === 0 ? <Empty title="Nothing collected yet" what="Royalties appear per cut mint once the item has run and its vaults are settled." /> : null}
        </Panel>
      </div>
      <div style={{ height: 16 }} />
      {/* Changed by Hookwars (app pass 5): access modes, approvals and licences (spec 14 E-1). */}
      <AccessPanel itemMint={it.itemMint} />
      <div style={{ height: 16 }} />
      <Panel title="History" flush>
        {it.history.length === 0 ? <Empty title="No history yet" what="Equips, cuts, settlements and claims of this item land here." /> : (
          <div className="rows">{it.history.map((h) => <div className="row" key={`${h.signature}:${h.ordinal}`}><span className="chip">{h.kind}</span><span className="muted">{short(h.signature, 8)}</span><span className="faint">{h.amount ?? ''}</span></div>)}</div>
        )}
      </Panel>
      {/* Changed by Hookwars (app v2): the item's actions, signed in the wallet. */}
      <div style={{ height: 16 }} />
      <Panel title="Act" meta={<Link href={`/marketplace/items/${it.itemMint}`}>sell or rent on the market</Link>}>
        <Actions>
          <Action route="royalties" title="Claim royalty" what="Settled royalty in one cut mint goes to the holder." fixed={{ item: it.item, itemMint: it.itemMint }} fields={[{ name: 'cutMint', label: 'Cut mint', kind: 'key' }, { name: 'amount', label: 'Amount (base units)', kind: 'amount' }]} />
          <Action route="forge" title="Forge" what="Burns this item and another of the same template into one a level higher. Two composites forge when their modules run the same templates in the same order." fixed={{ itemA: it.item }} fields={[{ name: 'itemB', label: 'Second item', kind: 'key' }]} />
          {wear ? <Action route="craft/repair" title="Repair" what={wear.dormant ? 'This item is dormant: a repair restores charges and wakes it.' : 'Restores charges before the item wears out.'} fixed={{ itemMint: it.itemMint }} fields={[{ name: 'recipeId', label: 'Repair recipe', kind: 'int' }]} /> : null}
          <Action route="items/fuse" title="Fuse" what={<>Burns this item and others you hold into one composite. One way. <Link href="/craft">Presets and recipes</Link></>} fields={[{ name: 'components', label: 'Components', kind: 'json', hint: `[{"itemMint":"${it.itemMint}","start":0,"count":1}, ...]` }, { name: 'royaltyBps', label: 'Royalty (bps)', kind: 'int' }]} />
        </Actions>
      </Panel>
    </>
  );
}
