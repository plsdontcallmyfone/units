// Changed by Hookwars: new file (explorer v2). One transaction, decoded: what happened (the story),
// every event in the order it ran, every instruction with its inner calls, and the logs.
import { read } from '@/lib/api';
import { DASH, int, short, sol } from '@/lib/format';
import { flatten, type Explained, type Ix } from '@/lib/explorer';
import { Empty, Head, Panel, ReadFailed, Stat } from '@/components/ui';
import { Addr, Facts, SearchBox, Story } from '@/components/explorer-ui';
import '../../explorer/explorer.css';

function Instruction({ ix, n }: { ix: Ix; n: string }) {
  const args = ix.args ? flatten(ix.args) : [];
  return (
    <div className="x-ix">
      <div className="x-ix-head">
        <span className="faint">#{n}</span>
        <span className="x-ix-name">{ix.name ?? 'unknown instruction'}</span>
        <Addr k={ix.programId} />
        {ix.undecoded ? <span className="chip warn" title={ix.undecoded}>not decoded</span> : null}
      </div>
      {ix.memo ? <p className="muted" style={{ margin: '6px 0 0', overflowWrap: 'anywhere' }}>{ix.memo.text}</p> : null}
      {args.length ? <details><summary>Arguments ({args.length})</summary><Facts facts={args} /></details> : null}
      {ix.accounts.length ? (
        <details><summary>Accounts ({ix.accounts.length})</summary>
          <Facts facts={ix.accounts.map((a) => [`${a.name}${a.signer ? ' (signs)' : ''}${a.writable ? ' (writes)' : ''}`, a.pubkey])} />
        </details>
      ) : null}
      {ix.inner.map((c, i) => <Instruction key={i} ix={c} n={`${n}.${i + 1}`} />)}
    </div>
  );
}

export default async function Tx({ params }: { params: Promise<{ sig: string }> }) {
  const { sig } = await params;
  const r = await read<Explained>(`/v1/explorer/tx/${encodeURIComponent(sig)}`);
  if (!r.ok) {
    return (<><Head eyebrow="Transaction" title={short(sig, 8)} /><Panel title="Search"><SearchBox /></Panel><div style={{ height: 16 }} /><Panel title="Transaction"><ReadFailed what="this transaction" error={r.error} /></Panel></>);
  }
  const t = r.data;
  return (
    <>
      <Head eyebrow="Transaction" title={short(sig, 8)} lede={<span className="x-val">{sig}</span>}
        right={t.ok ? <span className="chip ok">succeeded</span> : <span className="chip bad">failed</span>} />
      <div className="stats">
        <Stat label="Slot" value={t.slot === null ? DASH : int(t.slot)} />
        <Stat label="Time" value={t.blockTime ? new Date(t.blockTime * 1000).toISOString().slice(0, 16).replace('T', ' ') + ' UTC' : DASH} />
        <Stat label="Fee" value={t.fee === null ? DASH : sol(t.fee, 9)} />
        <Stat label="Compute units" value={t.computeUnits === null ? DASH : int(t.computeUnits)} />
        <Stat label="Events" value={int(t.events.length)} />
      </div>
      {t.failure ? (
        <Panel title="Why it failed">
          <Facts facts={[['Program', t.failure.programId], ['Error', t.failure.name ? `${t.failure.name} (${t.failure.code})` : t.failure.code === null ? DASH : String(t.failure.code)], ['Message', t.failure.message]]} />
        </Panel>
      ) : null}
      {t.failure ? <div style={{ height: 16 }} /> : null}
      <div className="x-grid2">
        <Panel title="What happened" meta={`${t.story.length} lines`} flush>
          {t.story.length === 0 ? <Empty title="Nothing units-specific" what="No units event or memo in this transaction; its instructions are listed alongside." /> : <Story lines={t.story} />}
        </Panel>
        <Panel title="Instructions" meta={`${t.instructions.length} top-level, ${t.programs.length} programs`} flush>
          {t.instructions.map((ix, i) => <Instruction key={i} ix={ix} n={String(i + 1)} />)}
        </Panel>
      </div>
      <div style={{ height: 16 }} />
      <Panel title="Events in order" meta={t.logsTruncated ? 'logs truncated: events past the cut come from inner calls only' : `${t.events.length}`} flush>
        {t.events.length === 0 ? <Empty title="No events" what="The units programs emitted no event in this transaction." /> : (
          <div className="x-scroll">
            <table>
              <thead><tr><th>#</th><th>Program</th><th>Event</th><th>Instruction</th><th>Fields</th></tr></thead>
              <tbody>{t.events.map((e) => (
                <tr key={e.ordinal}>
                  <td className="tabular">{e.ordinal + 1}</td>
                  <td>{e.program}</td>
                  <td>{e.name}</td>
                  <td className="tabular">#{e.instruction + 1}</td>
                  <td><details><summary className="faint">{Object.keys(e.data).length} fields</summary><Facts facts={flatten(e.data)} /></details></td>
                </tr>
              ))}</tbody>
            </table>
          </div>
        )}
      </Panel>
      <div style={{ height: 16 }} />
      <Panel title="Logs" meta={t.logs ? `${t.logs.length} lines` : undefined} flush>
        {t.logs && t.logs.length ? <details><summary className="faint" style={{ padding: '12px 20px' }}>Show the program logs</summary><ol className="x-logs">{t.logs.map((l, i) => <li key={i}>{l}</li>)}</ol></details> : <Empty title="No logs" what="The cluster returned no logs for this transaction." />}
      </Panel>
    </>
  );
}
