// Changed by Hookwars: new page, one template: its sentence with the field names in place of
// values, its fields with floor, ceiling and forge rule (floors and ceilings from the chain once
// registered), what it may do, what it keeps per holder, and the payout instructions it adds.
// Gating (spec 18): the template's supply (cap, made, circulating, room left, minter) and the
// premium flow (an item created Licensed with its terms in one transaction).
import Link from 'next/link';
import { FAMILY_OF, TEMPLATES, arsenalSentence, type TemplateInfo } from '@hookwars/shared';
import { read } from '@/lib/api';
import { Empty, Head, Panel, Stat } from '@/components/ui';
import { Action, Actions } from '@/components/action';

const FORGE: Record<string, string> = { towardCeiling: 'toward ceiling', towardFloor: 'toward floor', keep: 'kept', floorWhenBothOn: 'toward floor when both are on', none: 'not forgeable' };
/** Per-holder data each template keeps (packages/shared ranges.ts decoders). */
const HOLDER_DATA: Record<number, string> = {
  1: 'season, raid points and loot tickets', 2: 'when the holder last met a siege', 7: 'when the holder bought, for the decay',
  17: 'the holder\'s last buy time', 18: 'the day and the holding at its start', 20: 'the slot of the holder\'s last buy',
  24: 'the pot epoch the holder joined', 26: 'the last buy day, the streak and its flags', 35: 'volume and rank badge',
  39: 'the guild tag', 40: 'when the holder began holding',
};

export default async function TemplatePage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const t = TEMPLATES.find((x) => String(x.id) === id);
  if (!t) return <><Head eyebrow="Template" title={`Template ${id}`} /><Panel title="Template"><Empty title="No such template" what={<>See <Link href="/armory/templates">every template</Link>.</>} /></Panel></>;
  const [reg, sup] = await Promise.all([read<TemplateInfo[]>('/v1/templates'), read<SupplyRead | null>(`/v1/templates/${t.id}/supply`)]);
  const r = reg.ok ? reg.data.find((x) => x.templateId === t.id) : undefined;
  const sentence = t.id >= 10 ? arsenalSentence(t.id, (i) => `[${t.fields[i]?.name ?? i}]`, `[${t.targets}]`, []) : null;
  return (
    <>
      <Head eyebrow={`Template ${t.id} · ${FAMILY_OF[t.id] ?? t.kind} · spec ${t.spec}`} title={t.name} lede={sentence ?? 'The sentence fills in with an item\'s own values on its page.'}
        right={<Link className="btn sm" href="/armory">Mint or forge</Link>} />
      <div className="grid cols-main">
        <Panel title="Fields" meta={r ? `registered, ${r.status}` : 'not registered on this cluster'} flush>
          <table><thead><tr><th>#</th><th>Field</th><th>Floor</th><th>Ceiling</th><th>Forge</th></tr></thead>
            <tbody>{t.fields.map((f) => {
              const rf = r?.fields.find((x) => x.index === f.index);
              return <tr key={f.index}><td className="faint">{f.index}</td><td>{f.name}<div className="faint">{f.format}</div></td><td>{rf ? String(rf.min) : String(f.floor)}</td><td>{rf ? String(rf.max) : String(f.ceiling)}</td><td className="muted">{t.forgeable ? FORGE[f.forge] : 'not forgeable'}</td></tr>;
            })}</tbody></table>
        </Panel>
        <Panel title="Manifest">
          <dl className="kv">
            <dt>Slot kind</dt><dd>{t.kind}</dd>
            <dt>Callbacks</dt><dd>{t.callbacks.length ? t.callbacks.join(', ') : 'none: read by other programs'}</dd>
            <dt>Targets</dt><dd>{t.targets}</dd>
            <dt>Holder data</dt><dd>{t.dataBytes ? `${t.dataBytes} bytes: ${HOLDER_DATA[t.id] ?? 'decoded on the token page'}` : 'none'}</dd>
            <dt>Forgeable</dt><dd>{t.forgeable ? 'yes' : 'no'}</dd>
          </dl>
        </Panel>
      </div>
      <div style={{ height: 16 }} />
      <SupplyPanel templateId={t.id} sup={sup.ok ? sup.data : undefined} error={sup.ok ? undefined : sup.error} />
      {t.id === 24 || t.id === 36 || t.id === 38 ? <div style={{ height: 16 }} /> : null}
      {t.id === 24 ? (
        <Panel title="Loyalty pot">
          <Actions>
            <Action route="loyalty/init" title="Open the pot" what="Once per token and slot, after the item is equipped." fields={[{ name: 'mint', label: 'Token', kind: 'key' }, { name: 'slot', label: 'Slot', kind: 'int' }]} />
            <Action route="loyalty/claim" title="Claim your share" what="Wallets that held through the whole period claim their share of that period's pot." fields={[{ name: 'mint', label: 'Token', kind: 'key' }, { name: 'slot', label: 'Slot', kind: 'int' }]} />
          </Actions>
        </Panel>
      ) : null}
      {t.id === 36 ? (
        <Panel title="Referral">
          <Actions>
            <Action route="referral/set" title="Name your referrer" what="Once per token, before your first buy counts." fields={[{ name: 'mint', label: 'Token', kind: 'key' }, { name: 'referrer', label: 'Referrer', kind: 'key' }]} />
            <Action route="referral/settle" title="Pay a referrer" what="Anyone may crank the referral cut owed for a buyer." fields={[{ name: 'mint', label: 'Token', kind: 'key' }, { name: 'buyer', label: 'Buyer', kind: 'key' }, { name: 'slot', label: 'Slot', kind: 'int' }]} />
          </Actions>
        </Panel>
      ) : null}
      {t.id === 38 ? (
        <Panel title="First Blood">
          <Action route="first-blood/init" title="Open First Blood" what="Once per token, after the item is equipped." fields={[{ name: 'mint', label: 'Token', kind: 'key' }]} />
        </Panel>
      ) : null}
    </>
  );
}

type SupplyRead = {
  templateId: number; tracked: boolean; maxSupply?: number | null; lootReserve?: number; issued?: number; drops?: number; forged?: number; burned?: number;
  made?: number; circulating?: number; authorRoom?: number | null; dropRoom?: number | null; minterRule?: 'authorOnly' | 'openUntilCap'; minter?: string;
};

const dash = (v: number | null | undefined) => (v === null || v === undefined ? '-' : v.toLocaleString('en-US'));

function SupplyPanel({ templateId, sup, error }: { templateId: number; sup?: SupplyRead | null; error?: string }) {
  if (error) return <Panel title="Supply"><Empty title="Supply not read" what={`The API could not read this template's supply: ${error}`} /></Panel>;
  if (!sup) return <Panel title="Supply"><Empty title="Not registered" what="This template is not registered on this cluster, so it has no supply." /></Panel>;
  if (!sup.tracked) {
    return (
      <Panel title="Supply" meta="uncapped">
        <Empty title="No cap on this cluster" what="This template has no supply account yet: anyone its authoring rule allows may create copies, and copies are not counted on chain." />
      </Panel>
    );
  }
  return (
    <Panel title="Supply" meta={sup.minterRule === 'authorOnly' ? 'issued by its minter only' : 'open until the cap'}>
      <div className="stat-strip">
        <Stat label="Cap" value={sup.maxSupply === null ? 'uncapped' : dash(sup.maxSupply)} sub={sup.maxSupply === null ? 'counted only' : `${dash(sup.lootReserve)} kept for drops`} />
        <Stat label="Made" value={dash(sup.made)} sub={`${dash(sup.issued)} by authors, ${dash(sup.drops)} as drops`} />
        <Stat label="Circulating" value={dash(sup.circulating)} sub={`${dash(sup.forged)} forged, ${dash(sup.burned)} burned`} />
        <Stat label="Left for authors" value={sup.authorRoom === null ? 'no cap' : dash(sup.authorRoom)} sub={sup.dropRoom === null ? 'drops uncapped' : `${dash(sup.dropRoom)} left for drops`} />
      </div>
      <dl className="kv">
        <dt>Minter</dt><dd>{sup.minter ? <Link href={`/address/${sup.minter}`}>{sup.minter.slice(0, 4)}...{sup.minter.slice(-4)}</Link> : '-'}</dd>
        <dt>Rule</dt><dd>{sup.minterRule === 'authorOnly' ? 'Only the minter creates new copies; everyone else buys them on the market.' : 'Anyone may create copies until the cap.'}</dd>
      </dl>
      <Actions>
        <Action route="items/premium" title="Create a licensed item" what="Creates a copy and sets its licence terms in one transaction, so it is never listed without terms. Counts against the cap."
          fixed={{ templateId }}
          fields={[
            { name: 'params', label: 'Params', kind: 'ints', hint: 'comma separated, the fields above in order' },
            { name: 'priceLamports', label: 'Licence price', kind: 'sol' },
            { name: 'termSecs', label: 'Term (seconds)', kind: 'int' },
            { name: 'per', label: 'Priced', kind: 'int', choices: [['0', 'per token'], ['1', 'per period']] },
            { name: 'maxLive', label: 'Live licences at once', kind: 'int' },
            { name: 'royaltyBps', label: 'Royalty (bps)', kind: 'int' },
          ]} />
        <Action route="items/minter" title="Hand over the minter" what="The current minter hands the right to issue new copies to another wallet." fixed={{ templateId }}
          fields={[{ name: 'newMinter', label: 'New minter', kind: 'key' }]} />
      </Actions>
    </Panel>
  );
}
