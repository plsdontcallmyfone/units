// Changed by Hookwars: new file (explorer v2). An account decoded by its owner: mints with their
// slot table and equipped items, items, templates, passports, pools with their price ring, war
// chests, and whatever the indexer recorded naming it. Nothing here is computed beyond the ring's
// average, which is the ring's own cumulative difference over its own window.
import Link from 'next/link';
import { read } from '@/lib/api';
import { DASH, int, short, sol } from '@/lib/format';
import { flatten, q64, type AddressPage, type Fact } from '@/lib/explorer';
import { Empty, Head, Panel, ReadFailed, Stat } from '@/components/ui';
import { Addr, Facts, SearchBox, Signatures, Story } from '@/components/explorer-ui';
import '../../explorer/explorer.css';

const SLOT_KIND = ['fee', 'reward', 'defense', 'relation', 'pool', 'locked', 'war'];

export default async function Address({ params }: { params: Promise<{ key: string }> }) {
  const { key } = await params;
  const r = await read<AddressPage>(`/v1/explorer/address/${encodeURIComponent(key)}`);
  if (!r.ok) {
    return (<><Head eyebrow="Account" title={short(key, 6)} /><Panel title="Search"><SearchBox /></Panel><div style={{ height: 16 }} /><Panel title="Account"><ReadFailed what="this account" error={r.error} /></Panel></>);
  }
  const a = r.data;
  const fields: Fact[] = a.decoded.data ? flatten(a.decoded.data) : [];
  const ring = a.extra.ring;
  const ix = a.indexed;
  return (
    <>
      <Head eyebrow={a.kind} title={short(key, 6)} lede={<span className="x-val">{key}</span>}
        right={a.executable ? <Link className="btn sm" href={`/program/${key}`}>Program activity</Link> : undefined} />
      <div className="stats">
        <Stat label="Kind" value={a.kind} />
        <Stat label="Balance" value={a.lamports === null ? DASH : sol(a.lamports, 9)} />
        <Stat label="Owner" value={a.owner ? <Addr k={a.owner} /> : DASH} />
        <Stat label="Size" value={a.space === null ? DASH : `${int(a.space)} bytes`} />
      </div>
      {!a.exists ? (
        <Panel title="Account"><Empty title="No account at this address" what="Nothing is stored here on this cluster. A wallet that never received anything, or an account that was closed, reads this way." next={<SearchBox />} /></Panel>
      ) : (
        <div className="x-grid2">
          <Panel title={a.decoded.type ? `${a.decoded.program} ${a.decoded.type}` : 'Data'} meta={fields.length ? `${fields.length} fields` : undefined} flush={fields.length > 0}>
            {a.decoded.error ? <p className="reason">Could not decode: {a.decoded.error}</p> : null}
            {fields.length ? <div style={{ padding: '16px 20px' }}><Facts facts={fields} /></div> : <Empty title={a.executable ? 'A program' : 'Not a units account'} what={a.executable ? 'Its activity is on the program page.' : 'The owner is not one of the units programs, so there is no interface to decode it with.'} />}
          </Panel>
          <div className="grid">
            {ring ? (
              <Panel title="Price ring" meta={`${ring.filled} of ${ring.len} filled, every ${ring.spacing} s`}>
                <Facts facts={[
                  ['Last price', `${q64(ring.lastPriceQ64)} quote per base unit`],
                  ['Average over the ring', ring.twapQ64 ? `${q64(ring.twapQ64)} quote per base unit` : 'not enough entries'],
                  ['Window', ring.windowSecs ? `${int(ring.windowSecs)} s` : DASH],
                  ['Last update', ring.lastTs === '0' ? DASH : new Date(Number(ring.lastTs) * 1000).toISOString()],
                ]} />
              </Panel>
            ) : null}
            {a.extra.warChest !== undefined || a.extra.warState !== undefined ? (
              <Panel title="War">
                <Facts facts={[['War chest', a.extra.warChest ? a.extra.warChest.address : 'none'], ['Chest balance', a.extra.warChest ? sol(a.extra.warChest.lamports, 9) : DASH], ['War state', a.extra.warState ?? 'none']]} />
              </Panel>
            ) : null}
            {ix?.slots ? (
              <Panel title="Slots and equipped items" flush>
                {ix.slots.length === 0 ? <Empty title="No slot rows indexed" what="The indexer records a mint's slots from its SlotsInitialized and SlotEquipped events." /> : (
                  <div className="x-scroll"><table>
                    <thead><tr><th>#</th><th>Kind</th><th>Max cut</th><th>Item</th><th>Template</th><th>Manifest</th></tr></thead>
                    <tbody>{ix.slots.map((s) => (
                      <tr key={String(s.slot)}>
                        <td>{String(s.slot)}</td>
                        <td>{SLOT_KIND[Number(s.kind)] ?? String(s.kind)}</td>
                        <td className="tabular">{s.max_cut_bps === null ? DASH : `${Number(s.max_cut_bps) / 100}%`}</td>
                        <td>{s.item ? <Addr k={String(s.item)} /> : <span className="faint">empty</span>}</td>
                        <td>{s.template_id === null || s.template_id === undefined ? DASH : String(s.template_id)}</td>
                        <td>{s.manifest ? <details><summary className="faint">manifest</summary><Facts facts={flatten(s.manifest)} /></details> : DASH}</td>
                      </tr>
                    ))}</tbody>
                  </table></div>
                )}
              </Panel>
            ) : null}
            {ix?.item ? (
              <Panel title="Item">
                <Facts facts={flatten({ template: ix.item.template_id, level: ix.item.level, author: ix.item.author, royaltyBps: ix.item.royalty_bps, source: ix.item.source, params: ix.item.params, owner: ix.owner?.owner ?? null })} />
                {ix.equippedOn && ix.equippedOn.length ? <><div className="sep" /><Facts facts={ix.equippedOn.map((e) => [`Slot ${e.slot} of`, e.mint])} /></> : null}
              </Panel>
            ) : null}
            {ix?.template ? <Panel title="Template, as indexed"><Facts facts={flatten(ix.template)} /></Panel> : null}
          </div>
        </div>
      )}
      <div style={{ height: 16 }} />
      <div className="x-grid2">
        <Panel title="Events naming this account" meta={ix ? `${ix.events.length} newest` : 'indexer not connected'} flush>
          {!ix ? <Empty title="History needs the indexer" what="The account above is read from the chain; its event history comes from the indexer, which this backend is not connected to." />
            : ix.events.length === 0 ? <Empty title="No indexed events" what="No event the indexer holds names this address in a top-level field." />
            : <Story lines={ix.events.filter((e) => e.story).map((e) => ({ ...e.story!, facts: [['Transaction', e.signature], ...e.story!.facts] }))} />}
        </Panel>
        <Panel title="Recent transactions" flush>
          <Signatures list={a.recent} empty={<Empty title="No transactions" what="The cluster has no signatures for this address." />} />
        </Panel>
      </div>
    </>
  );
}
