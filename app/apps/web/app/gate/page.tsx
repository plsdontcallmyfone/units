// Changed by Hookwars: new page (gating, docs/spec/18-gating.md part C): use units hooks on a
// Token-2022 token of any protocol. A mint whose transfer hook is the gate registers, binds items
// it licensed or owns, and runs them on every transfer: refusals, holder stamps and price reads,
// never cuts. Chain state only, through /v1/gate/<mint>.
import Link from 'next/link';
import { TEMPLATES } from '@hookwars/shared';
import { read } from '@/lib/api';
import { Empty, Head, Panel, ReadFailed, Stat } from '@/components/ui';
import { Action, Actions } from '@/components/action';

export const dynamic = 'force-dynamic';

const GATE_ID = 'CKMGqUdYzVyTr4myZsXo8rEMeBA5QoqdABinbZfvh8Dj';
/** The templates the app binds on external tokens (spec 18 section 3.3: no token-side cut). */
const GATEABLE = [17, 18, 19, 20, 22, 26, 39];

type Binding = { slot: number; item: string; itemMint: string; templateId: number; kind: 'licence' | 'owned'; proof: string; live: boolean | null; dataBytes: number; boundAt: string };
type GateRead = {
  mint: string; token2022: boolean; hookProgram: string | null; hookedByGate: boolean; registered: boolean;
  authority?: string; venue?: string | null; strict?: boolean; bindings?: Binding[];
};

const short = (k: string) => `${k.slice(0, 4)}...${k.slice(-4)}`;
const nameOf = (id: number) => TEMPLATES.find((t) => t.id === id)?.name ?? `Template ${id}`;

export default async function GatePage({ searchParams }: { searchParams: Promise<{ mint?: string }> }) {
  const { mint } = await searchParams;
  const valid = mint && /^\w{32,44}$/.test(mint) ? mint : null;
  const r = valid ? await read<GateRead | null>(`/v1/gate/${valid}`) : null;
  return (
    <>
      <Head eyebrow="External tokens" title="Use units hooks on your token"
        lede={<>Any Token-2022 token, from any protocol, can run units items as its transfer hook. Set the gate (<code style={{ overflowWrap: 'anywhere' }}>{GATE_ID}</code>) as your mint&apos;s transfer hook program, register here, then bind items you licensed or own. A binding stays live only while its licence does.</>} />
      <Panel title="Look up a mint">
        <form className="action-fields" action="/gate" method="get">
          <label className="field">Token-2022 mint<input name="mint" defaultValue={valid ?? ''} placeholder="address" /></label>
          <div className="action-row"><button type="submit" className="btn sm">Read</button></div>
        </form>
      </Panel>
      <div style={{ height: 16 }} />
      {!valid ? (
        <Panel title="Status"><Empty title="No mint chosen" what="Enter a Token-2022 mint above to see its gate, its bindings and what it can do next." /></Panel>
      ) : !r || !r.ok ? (
        <ReadFailed what="the mint's gate" error={r && !r.ok ? r.error : 'no response'} />
      ) : !r.data ? (
        <Panel title="Status"><Empty title="No such mint" what="No account exists at this address on this cluster." /></Panel>
      ) : <Status g={r.data} />}
      <div style={{ height: 16 }} />
      <div className="grid cols-main">
        <Panel title="What runs on an external token">
          <dl className="kv">
            <dt>Can</dt><dd>Refuse a transfer, stamp per-wallet memory (cooldowns, streaks, tags), read prices.</dd>
            <dt>Cannot</dt><dd>Take a cut, burn or discount: a Token-2022 hook has no authority over the tokens moved. Income comes from licence terms.</dd>
            <dt>Lapse</dt><dd>When a licence ends or the vault no longer holds the item, the binding allows every transfer, and anyone may unbind it.</dd>
            <dt>Holder memory</dt><dd>Kept by the gate per wallet. A strict mint refuses transfers to or from wallets without it; the venue (your pool&apos;s authority) is exempt.</dd>
          </dl>
        </Panel>
        <Panel title="Templates the app binds" flush>
          <table><thead><tr><th>#</th><th>Template</th></tr></thead>
            <tbody>{GATEABLE.map((id) => <tr key={id}><td className="faint">{id}</td><td><Link href={`/armory/templates/${id}`}>{nameOf(id)}</Link></td></tr>)}</tbody></table>
        </Panel>
      </div>
    </>
  );
}

function Status({ g }: { g: GateRead }) {
  if (!g.token2022) return <Panel title="Status"><Empty title="Not a Token-2022 mint" what="The gate runs as a Token-2022 transfer hook; this mint belongs to another token program." /></Panel>;
  if (!g.hookedByGate) {
    return (
      <Panel title="Status">
        <Empty title="Its transfer hook is not the gate"
          what={g.hookProgram ? <>This mint&apos;s transfer hook program is <code style={{ overflowWrap: 'anywhere' }}>{g.hookProgram}</code>. Its transfer-hook authority can point it at the gate.</> : 'This mint has no transfer hook program set. Its transfer-hook authority can set it to the gate.'} />
      </Panel>
    );
  }
  const fixed = { mint: g.mint };
  if (!g.registered) {
    return (
      <Panel title="Status" meta="hooked by the gate, not registered">
        <Empty title="Not registered yet" what="Transfers of this mint fail until it registers: the gate has no list of accounts for it." />
        <Actions>
          <Action route="gate/register" title="Register the mint" what="Signed by the mint's transfer-hook authority. The venue is your AMM pool's authority, whose transfers count as buys and sells." fixed={fixed}
            fields={[{ name: 'venue', label: 'Venue', kind: 'key', optional: true }, { name: 'strict', label: 'Strict holder memory', kind: 'bool' }]} />
        </Actions>
      </Panel>
    );
  }
  const bindings = g.bindings ?? [];
  return (
    <Panel title="Status" meta={g.strict ? 'strict' : 'not strict'}>
      <div className="stat-strip">
        <Stat label="Bindings" value={`${bindings.length} of 4`} sub={`${bindings.filter((b) => b.live).length} live`} />
        <Stat label="Authority" value={g.authority ? short(g.authority) : '-'} />
        <Stat label="Venue" value={g.venue ? short(g.venue) : 'none'} sub={g.venue ? 'its transfers are buys and sells' : 'every transfer is a send'} />
      </div>
      {bindings.length ? (
        <table><thead><tr><th>Slot</th><th>Item</th><th>Template</th><th>Proof</th><th>State</th></tr></thead>
          <tbody>{bindings.map((b) => (
            <tr key={b.slot}><td>{b.slot}</td><td><Link href={`/armory/items/${b.itemMint}`}>{short(b.itemMint)}</Link></td><td>{nameOf(b.templateId)}</td>
              <td className="muted">{b.kind === 'licence' ? 'licence' : 'held by the vault'}</td>
              <td className={b.live ? 'ready' : 'waiting'}>{b.live === null ? '-' : b.live ? 'live' : 'lapsed: allows every transfer'}</td></tr>
          ))}</tbody></table>
      ) : <Empty title="Nothing bound" what="Bind an item you licensed for this mint, or one you sent to the gate's vault." />}
      <Actions>
        <Action route="gate/bind" title="Bind an item" what="The item's licence for this mint (bought on the item's page) or the vault's holding of it is checked now and on every transfer." fixed={fixed}
          fields={[{ name: 'itemMint', label: 'Item mint', kind: 'key' }, { name: 'slot', label: 'Slot (0 to 3)', kind: 'int' }, { name: 'kind', label: 'Proof', kind: 'text', choices: [['licence', 'a licence for this mint'], ['owned', 'the vault holds it']] }]} />
        <Action route="gate/unbind" title="Unbind" what="The authority at any time; anyone once the binding lapsed." fixed={fixed} fields={[{ name: 'slot', label: 'Slot', kind: 'int' }]} />
        <Action route="gate/holder" title="Open holder memory" what="Pays one wallet's holder memory for this mint. Strict mints need it before a wallet can send or receive." fixed={fixed}
          fields={[{ name: 'wallet', label: 'Wallet', kind: 'key', optional: true, hint: 'yours when empty' }]} />
        <Action route="gate/venue" title="Set the venue" what="The pool authority whose transfers are buys and sells, and whether holder memory is required." fixed={fixed}
          fields={[{ name: 'venue', label: 'Venue', kind: 'key', optional: true }, { name: 'strict', label: 'Strict holder memory', kind: 'bool' }]} />
      </Actions>
    </Panel>
  );
}
