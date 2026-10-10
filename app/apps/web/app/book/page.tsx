// Changed by Hookwars: new file (app pass v3). The order book (11 section 6): a limit order book
// per material (price, then time), class bids for any item of a template within parameter ranges,
// and the forms to place, cancel and crank. Every figure is read from the chain.
import Link from 'next/link';
import { TEMPLATES } from '@hookwars/shared';
import { read } from '@/lib/api';
import { short, sol } from '@/lib/format';
import { when } from '@/lib/labels';
import { Empty, Head, Panel, ReadFailed } from '@/components/ui';
import { Action, Actions } from '@/components/action';

type Order = { id: string; owner: string; price: string; size: string; expiresAt: string };
type Market = { address: string; baseMint: string; materialId: number; tickLamports: string; minSize: string; bids: Order[]; asks: Order[]; fills: string };
type ClassBid = { address: string; bidder: string; nonce: string; price: string; expiresAt: string; class: { templateId: number; minLevel: number } };
type Books = { config: { params: { takerBps: number; makerBps: number; slots: number } } | null; markets: Market[]; classBids: ClassBid[] };
const tname = (id: number) => TEMPLATES.find((t) => t.id === id)?.name ?? `template ${id}`;

function Side({ title, orders }: { title: string; orders: Order[] }) {
  return orders.length === 0 ? <p className="faint">No {title.toLowerCase()} resting.</p> : (
    <table><thead><tr><th>{title}</th><th className="r">Price</th><th className="r">Size</th><th>Expires</th></tr></thead>
      <tbody>{orders.map((o) => <tr key={o.id}><td>#{o.id} <span className="faint">{short(o.owner)}</span></td><td className="r">{sol(o.price, 6)}</td><td className="r">{o.size}</td><td className="faint">{o.expiresAt === '0' ? 'never' : when(o.expiresAt)}</td></tr>)}</tbody></table>
  );
}

export default async function BookPage() {
  const r = await read<Books>('/v1/book');
  const d = r.ok ? r.data : null;
  return (
    <>
      <Head eyebrow="Order book" title="Materials and item bids" lede="Each material trades on its own book, best price first and oldest first at a price. A class bid buys any item of a template within the ranges it names; the holder of a fitting item sells into it."
        right={<Link className="btn sm" href="/craft">Craft</Link>} />
      {!r.ok ? <Panel title="Books"><ReadFailed what="the order books" error={r.error} /></Panel> : !d!.config ? (
        <Panel title="Books"><Empty title="The order book is not set up on this cluster" what="It has no config here yet." /></Panel>
      ) : (
        <>
          <p className="muted">Taker fee {d!.config!.params.takerBps / 100}%, maker fee {d!.config!.params.makerBps / 100}%, {d!.config!.params.slots} resting orders a side.</p>
          {d!.markets.length === 0 ? <Panel title="Markets"><Empty title="No material has a book yet" what="Anyone at the Trader level the config names opens one for a material." /></Panel> : d!.markets.map((m) => (
            <div key={m.address}>
              <Panel title={`Material ${m.materialId}`} meta={`tick ${sol(m.tickLamports, 6)}, min size ${m.minSize}, ${m.fills} fills`}>
                <div className="grid cols-2"><Side title="Bids" orders={m.bids} /><Side title="Asks" orders={m.asks} /></div>
              </Panel>
              <div style={{ height: 16 }} />
            </div>
          ))}
          <Panel title="Class bids" meta={`${d!.classBids.length} open`} flush>
            {d!.classBids.length === 0 ? <Empty title="No class bids" what="A bid for any item of a template, from a level, within parameter ranges." /> : (
              <table><thead><tr><th>Class</th><th>Bidder</th><th className="r">Price</th><th>Expires</th></tr></thead>
                <tbody>{d!.classBids.map((b) => <tr key={b.address}><td>{tname(b.class.templateId)}{b.class.minLevel ? ` from L${b.class.minLevel}` : ''}</td><td>{short(b.bidder)}</td><td className="r">{sol(b.price, 4)}</td><td className="faint">{b.expiresAt === '0' ? 'never' : when(b.expiresAt)}</td></tr>)}</tbody></table>
            )}
          </Panel>
        </>
      )}
      <div style={{ height: 16 }} />
      <Panel title="Act">
        <Actions>
          <Action route="book/place" title="Place an order" what="Crosses the best opposite orders, then rests what is left." fields={[
            { name: 'materialId', label: 'Material', kind: 'int' }, { name: 'side', label: 'Side', kind: 'text', choices: [['bid', 'bid (buy)'], ['ask', 'ask (sell)']] },
            { name: 'price', label: 'Price per unit (lamports)', kind: 'amount' }, { name: 'size', label: 'Size (units)', kind: 'amount' }, { name: 'postOnly', label: 'Post only', kind: 'bool' },
          ]} />
          <Action route="book/cancel" title="Cancel an order" fields={[{ name: 'materialId', label: 'Material', kind: 'int' }, { name: 'id', label: 'Order id', kind: 'amount' }]} />
          <Action route="book/crank" title="Clear expired orders" what="Anyone removes expired orders and takes their bounties." fields={[{ name: 'materialId', label: 'Material', kind: 'int' }]} />
          <Action route="book/class-bid" title="Bid for a class" fields={[{ name: 'nonce', label: 'Nonce', kind: 'amount' }, { name: 'class', label: 'Class', kind: 'json', hint: '{"templateId":1,"minLevel":0,"paramMin":[],"paramMax":[]}' }, { name: 'price', label: 'Price', kind: 'sol' }]} />
          <Action route="book/class-bid/match" title="Sell into a class bid" fields={[{ name: 'bid', label: 'Class bid', kind: 'key' }, { name: 'itemMint', label: 'Your item mint', kind: 'key' }]} />
        </Actions>
      </Panel>
    </>
  );
}
