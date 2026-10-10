// Changed by Hookwars: new page, every template the items program runs (04, 08, 09, 10), grouped
// by family, each linking to its page. Registration state comes from the chain.
import Link from 'next/link';
import { FAMILY_OF, TEMPLATES, type TemplateInfo } from '@hookwars/shared';
import { read } from '@/lib/api';
import { Head, Panel } from '@/components/ui';

const FAMILIES = ['Fee', 'Defense', 'Reward', 'Relation', 'War', 'Burn', 'Social', 'Utility'] as const;

export default async function Templates() {
  const reg = await read<TemplateInfo[]>('/v1/templates');
  const byId = new Map((reg.ok ? reg.data : []).map((t) => [t.templateId, t]));
  return (
    <>
      <Head eyebrow="Armory" title="Templates" lede={`${TEMPLATES.length} templates. A template is the code; an item is one minted with its own values inside the template's floor and ceiling.${reg.ok ? '' : ' Registration could not be read, so none is shown as registered.'}`} right={<Link className="btn sm" href="/armory/templates/submissions">Submissions</Link>} />
      <div className="grid cols-2">
        {FAMILIES.map((f) => {
          const ts = TEMPLATES.filter((t) => FAMILY_OF[t.id] === f);
          if (!ts.length) return null;
          return (
            <Panel key={f} title={f} meta={`${ts.length}`} flush>
              <table><tbody>{ts.map((t) => {
                const r = byId.get(t.id);
                return <tr key={t.id}><td style={{ width: 44 }} className="faint">{t.id}</td><td><Link href={`/armory/templates/${t.id}`}>{t.name}</Link><div className="faint">{t.kind} slot{t.dataBytes ? `, ${t.dataBytes} bytes per holder` : ''}</div></td><td className="r">{r ? <span className={`chip ${r.status === 'active' ? 'ok' : ''}`}>{r.status}</span> : <span className="chip">not on this cluster</span>}</td></tr>;
              })}</tbody></table>
            </Panel>
          );
        })}
      </div>
    </>
  );
}
