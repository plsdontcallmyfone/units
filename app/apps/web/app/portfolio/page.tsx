import type { ItemSummary, LootRollInfo, MyHoldingWar } from '@hookwars/shared';
import { read } from '@/lib/api';
import { short } from '@/lib/format';
import { Empty, Head, Panel, ReadFailed } from '@/components/ui';

export default async function Portfolio({ searchParams }: { searchParams: Promise<{ owner?: string }> }) {
  const { owner } = await searchParams;
  const w = owner ? await read<{ holdings: MyHoldingWar[]; items: ItemSummary[]; rolls: LootRollInfo[] }>(`/v1/wallet/${owner}/war`) : null;
  return (
    <>
      <Head eyebrow="Portfolio" title={owner ? short(owner, 6) : 'Your wallet'} lede="Items you hold with their royalty positions, loot rolls, raid points, tickets and vote locks." />
      <form className="panel" style={{ padding: 12, display: 'flex', gap: 8, marginBottom: 16 }} action="/portfolio">
        <input name="owner" defaultValue={owner ?? ''} placeholder="Wallet address" style={{ flex: 1 }} aria-label="Wallet address" />
        <button className="btn primary" type="submit">Show</button>
      </form>
      {!w ? <Panel title="Wallet"><Empty title="No wallet chosen" what="Enter an address to see what the indexer holds for it." /></Panel> : !w.ok ? <Panel title="Wallet"><ReadFailed what="this wallet" error={w.error} /></Panel> : (
        <div className="grid cols-2">
          <Panel title="Items" flush>{w.data.items.length ? <div className="rows">{w.data.items.map((i) => <div className="row" key={i.item}><span className="chip">{i.templateName}</span><span className="muted">{i.paramsText}</span><span className="faint">L{i.level}</span></div>)}</div> : <Empty title="No items" what="Items this wallet holds appear with what they have earned." />}</Panel>
          <Panel title="War holdings" flush>{w.data.holdings.length ? null : <Empty title="No raid points or tickets" what="Points and tickets live in your holdings of each token and appear once stamped by a raid." />}</Panel>
        </div>
      )}
    </>
  );
}
