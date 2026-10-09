import Link from 'next/link';
import type { BattleEvent, ItemSummary } from '@hookwars/shared';
import { read } from '@/lib/api';
import { short } from '@/lib/format';
import { Empty, Head, Panel, ReadFailed } from '@/components/ui';

export default async function ItemPage({ params }: { params: Promise<{ item: string }> }) {
  const { item } = await params;
  const r = await read<ItemSummary & { history: BattleEvent[] }>(`/v1/items/${item}`);
  if (!r.ok) return <><Head eyebrow="Item" title={short(item, 6)} /><Panel title="Item"><ReadFailed what="this item" error={r.error} /></Panel></>;
  const it = r.data;
  return (
    <>
      <Head eyebrow={`${it.templateName} · level ${it.level}`} title={it.paramsText} lede={`Royalty ${it.royaltyBps / 100}% of what it collects, paid to whoever holds it.`} />
      <div className="grid cols-main">
        <Panel title="Record">
          <dl className="kv">
            <dt>Item</dt><dd className="addr">{it.item}</dd>
            <dt>Owner</dt><dd className="addr">{it.owner ?? '-'}</dd>
            <dt>Author</dt><dd className="addr">{it.author}</dd>
            <dt>Source</dt><dd>{it.source}</dd>
            <dt>Params</dt><dd>{it.params.join(', ')}</dd>
            <dt>Equipped on</dt><dd>{it.equippedOn.length ? it.equippedOn.map((e) => <Link key={e.mint} href={`/t/${e.mint}`}>{e.symbol} </Link>) : 'no token'}</dd>
          </dl>
        </Panel>
        <Panel title="Royalty position" meta="unsettled, settles to royalty, claimable">
          {it.royalties.length === 0 ? <Empty title="Nothing collected yet" what="Royalties appear per cut mint once the item has run and its vaults are settled." /> : null}
        </Panel>
      </div>
      <div style={{ height: 16 }} />
      <Panel title="History" flush>
        {it.history.length === 0 ? <Empty title="No history yet" what="Equips, cuts, settlements and claims of this item land here." /> : (
          <div className="rows">{it.history.map((h) => <div className="row" key={`${h.signature}:${h.ordinal}`}><span className="chip">{h.kind}</span><span className="muted">{short(h.signature, 8)}</span><span className="faint">{h.amount ?? ''}</span></div>)}</div>
        )}
      </Panel>
    </>
  );
}
