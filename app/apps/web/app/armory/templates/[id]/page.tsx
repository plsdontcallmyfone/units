// Changed by Hookwars: new page, one template: its sentence with the field names in place of
// values, its fields with floor, ceiling and forge rule (floors and ceilings from the chain once
// registered), what it may do, what it keeps per holder, and the payout instructions it adds.
import Link from 'next/link';
import { FAMILY_OF, TEMPLATES, arsenalSentence, type TemplateInfo } from '@hookwars/shared';
import { read } from '@/lib/api';
import { Empty, Head, Panel } from '@/components/ui';
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
  const reg = await read<TemplateInfo[]>('/v1/templates');
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
