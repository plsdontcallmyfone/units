// Changed by Hookwars: new page (app pass 5). External templates (pass 4a, Hook Lab gaps 1 to 4):
// a builder posts a bond for a template program (`submit_template`), the Hook Lab checks it in two
// stages (build sandboxed with no key, then the signed property suite), and the admin settles the
// bond and registers the program behind the admin timelock. Every row is a chain account.
import Link from 'next/link';
import { read } from '@/lib/api';
import { short, sol } from '@/lib/format';
import { when } from '@/lib/labels';
import { Empty, Head, Panel, ReadFailed } from '@/components/ui';
import { Action, Actions } from '@/components/action';

type Submission = { address: string; program: string; codeHash: string | number[]; uriHash: string | number[]; submitter: string; bond: string; submittedAt: string };
const hex = (h: string | number[]) => (Array.isArray(h) ? h.map((b) => b.toString(16).padStart(2, '0')).join('') : String(h).replace(/^0x/, ''));

export default async function Submissions() {
  const r = await read<Submission[]>('/v1/templates/submissions');
  const rows = r.ok ? r.data : [];
  const bonded = rows.reduce((a, x) => a + BigInt(x.bond), 0n);
  return (
    <>
      <Head eyebrow="Armory" title="Template submissions" lede="Anyone can bring a hook template as its own program. The bond keeps the queue honest: an approved program gets it back, a rejected one may forfeit it. The Hook Lab's signed report is what the admin checks before registering."
        right={<Link className="btn sm" href="/armory/templates">All templates</Link>} />
      {!r.ok ? <Panel title="Submissions"><ReadFailed what="the submissions" error={r.error} /></Panel> : (
        <>
          <div className="stat-strip">
            <div><span className="label">Open submissions</span><b>{rows.length}</b></div>
            <div><span className="label">Bonds held</span><b>{sol(bonded.toString(), 4)}</b></div>
          </div>
          <Panel title="Waiting for a decision" flush>
            {rows.length === 0 ? <Empty title="No submission is waiting" what="Run the Hook Lab on your crate, deploy the program with no upgrade authority, then submit it here with its code hash." next={<Link href="/docs/guides/build-a-hook-template">Build a hook template</Link>} /> : (
              <table><thead><tr><th>Program</th><th>Code hash</th><th>Submitter</th><th className="r">Bond</th><th>Submitted</th></tr></thead>
                <tbody>{rows.map((x) => <tr key={x.address}><td className="addr">{short(x.program, 6)}</td><td title={hex(x.codeHash)}>{short(hex(x.codeHash), 8)}</td><td>{short(x.submitter)}</td><td className="r">{sol(x.bond, 4)}</td><td className="faint">{when(x.submittedAt)}</td></tr>)}</tbody></table>
            )}
          </Panel>
        </>
      )}
      <div style={{ height: 16 }} />
      <Panel title="Act" meta="settling and registering need the armory admin">
        <Actions>
          <Action route="templates/submit" title="Submit a template" what="Posts the bond; a high enough Builder level pays less." fields={[{ name: 'program', label: 'Program id', kind: 'key' }, { name: 'codeHash', label: 'Code hash (64 hex)', kind: 'text' }, { name: 'uriHash', label: 'Report URI hash (64 hex)', kind: 'text' }]} />
          <Action route="templates/settle" title="Settle a submission" what="Approve returns the bond; reject returns or forfeits it." fields={[{ name: 'program', label: 'Program id', kind: 'key' }, { name: 'approved', label: 'Approved', kind: 'bool' }, { name: 'forfeit', label: 'Forfeit the bond', kind: 'bool' }, { name: 'templateId', label: 'Registered template id', kind: 'int', optional: true }]} />
          <Action route="templates/register-external" title="Register an external template" what="From the Hook Lab report's register_template block: queue it first, then send it again with the same data once the timelock has passed." fields={[
            { name: 'templateProgram', label: 'Template program', kind: 'key' }, { name: 'data', label: 'Instruction data (base64)', kind: 'text' },
            { name: 'keys', label: 'Instruction accounts (JSON)', kind: 'json', optional: true }, { name: 'queue', label: 'Queue (yes) or register (no)', kind: 'bool' },
          ]} />
        </Actions>
      </Panel>
    </>
  );
}
