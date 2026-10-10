// Changed by Hookwars: new page (app pass 5). What waits on a timelock (pass 4a L-1, secfix3 M-8):
// the armory's queued admin actions and the order books' proposed market terms and parameters,
// each with the time it may apply. Every row is a chain account; nothing is estimated.
import { read } from '@/lib/api';
import { short, sol } from '@/lib/format';
import { when } from '@/lib/labels';
import { Empty, Head, Panel, ReadFailed } from '@/components/ui';
import { Action, Actions } from '@/components/action';

type Queued = { address: string; actionHash: string | number[]; admin: string; readyAt: string };
type Terms = { address: string; market: string; tickLamports: string; minSize: string; readyAt: string };
type BookParams = { address: string; treasury: string; readyAt: string };
type Queue = { armoryAdmin: string | null; queued: Queued[]; marketTerms: Terms[]; bookParams: BookParams[] };

const hex = (h: string | number[]) => (Array.isArray(h) ? h.map((b) => b.toString(16).padStart(2, '0')).join('') : String(h).replace(/^0x/, ''));
const now = () => Math.floor(Date.now() / 1000);
function Ready({ at }: { at: string }) {
  const t = Number(at);
  return t <= now() ? <span className="ready">ready since {when(at)}</span> : <span className="waiting">ready {when(at)}</span>;
}

export default async function Governance() {
  const r = await read<Queue>('/v1/governance/queue');
  const d = r.ok ? r.data : null;
  const waiting = d ? d.queued.length + d.marketTerms.length + d.bookParams.length : 0;
  const ready = d ? [...d.queued, ...d.marketTerms, ...d.bookParams].filter((x) => Number(x.readyAt) <= now()).length : 0;
  return (
    <>
      <Head eyebrow="Governance" title="Timelocked changes" lede="Every admin setter waits out a public timelock before it applies: it is queued with a hash of exactly what will change, anyone can read it here, and it applies only after its ready time. Market terms of a book work the same way." />
      {!r.ok ? <Panel title="Queue"><ReadFailed what="the timelock queue" error={r.error} /></Panel> : (
        <>
          <div className="stat-strip">
            <div><span className="label">Waiting</span><b>{waiting}</b></div>
            <div><span className="label">Ready to apply</span><b>{ready}</b></div>
            <div><span className="label">Armory admin</span><b style={{ fontSize: 16 }}>{d!.armoryAdmin ? short(d!.armoryAdmin, 6) : 'not set up here'}</b></div>
          </div>
          <Panel title="Armory admin queue" meta={`${d!.queued.length} queued`} flush>
            {d!.queued.length === 0 ? <Empty title="Nothing queued" what="A template registration, retirement, economy change, preset, protocol fee or access parameter change shows here from the moment it is queued until it applies or is cancelled." /> : (
              <table><thead><tr><th>Action hash</th><th>Queued by</th><th>Applies</th></tr></thead>
                <tbody>{d!.queued.map((q) => <tr key={q.address}><td className="addr" title={hex(q.actionHash)}>{short(hex(q.actionHash), 10)}</td><td>{short(q.admin)}</td><td><Ready at={q.readyAt} /></td></tr>)}</tbody></table>
            )}
          </Panel>
          <div style={{ height: 16 }} />
          <div className="grid cols-2">
            <Panel title="Proposed market terms" meta={`${d!.marketTerms.length}`} flush>
              {d!.marketTerms.length === 0 ? <Empty title="No terms proposed" what="The book's admin proposes a new tick and minimum size for one material's book; anyone applies it after the timelock." /> : (
                <table><thead><tr><th>Book</th><th className="r">Tick</th><th className="r">Min size</th><th>Applies</th></tr></thead>
                  <tbody>{d!.marketTerms.map((t) => <tr key={t.address}><td>{short(t.market)}</td><td className="r">{sol(t.tickLamports, 9)}</td><td className="r">{t.minSize}</td><td><Ready at={t.readyAt} /></td></tr>)}</tbody></table>
              )}
            </Panel>
            <Panel title="Proposed book parameters" meta={`${d!.bookParams.length}`} flush>
              {d!.bookParams.length === 0 ? <Empty title="No parameters proposed" what="Fees, slots, bounds and the skill minimum of the order book change only through this queue." /> : (
                <table><thead><tr><th>Treasury</th><th>Applies</th></tr></thead>
                  <tbody>{d!.bookParams.map((p) => <tr key={p.address}><td>{short(p.treasury)}</td><td><Ready at={p.readyAt} /></td></tr>)}</tbody></table>
              )}
            </Panel>
          </div>
        </>
      )}
      <div style={{ height: 16 }} />
      <Panel title="Act" meta="admin actions need the admin wallet; applying is open to anyone">
        <Actions>
          <Action route="armory/queue" title="Queue an admin action" what="Queues the hash of the exact setter call; the setter applies with the same arguments after the timelock." fields={[{ name: 'actionHash', label: 'Action hash (64 hex)', kind: 'text' }]} />
          <Action route="armory/queue/cancel" title="Cancel a queued action" fields={[{ name: 'actionHash', label: 'Action hash (64 hex)', kind: 'text' }]} />
          <Action route="book/market-terms/propose" title="Propose market terms" what="Bounded by the book's tick and minimum size limits, checked again when applied." fields={[{ name: 'materialId', label: 'Material', kind: 'int' }, { name: 'tickLamports', label: 'Tick (lamports)', kind: 'amount' }, { name: 'minSize', label: 'Minimum size (units)', kind: 'amount' }]} />
          <Action route="book/market-terms/apply" title="Apply market terms" what="Anyone, after the ready time." fields={[{ name: 'materialId', label: 'Material', kind: 'int' }]} />
          <Action route="armory/access-params" title="Access parameters" what="Licence tier and level, the Hook Lab bond and its discount, licence term bounds. Queue first, then apply with the same values." fields={[
            { name: 'params', label: 'Parameters', kind: 'json', hint: '{"licenceTier1Lamports":"0","licenceTier1Level":0,"labBondLamports":"0","labBondDiscountLevel":0,"labBondDiscountBps":0,"licenceMinSecs":0,"licenceMaxSecs":0}' },
            { name: 'queue', label: 'Queue (yes) or apply (no)', kind: 'bool' },
          ]} />
        </Actions>
      </Panel>
    </>
  );
}
