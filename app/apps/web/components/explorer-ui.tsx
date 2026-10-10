// Changed by Hookwars: new file (explorer v2), pieces the explorer pages share.
import Link from 'next/link';
import type { ReactNode } from 'react';
import { ago, short } from '@/lib/format';
import { hrefOf, isKey, otherNameOf, programNameOf, type Fact, type Signature, type StoryLine } from '@/lib/explorer';

/** An address as a link to its page, shortened, the full key on hover. */
export function Addr({ k, full }: { k: string; full?: boolean }) {
  const name = programNameOf(k) ?? otherNameOf(k);
  return <Link className="addr x-addr" href={hrefOf(k)} title={k}>{name ? `${name} program` : full ? k : short(k, 6)}</Link>;
}

/** A decoded value: addresses link, everything else is text as decoded. */
export function Val({ v }: { v: string }) {
  if (isKey(v)) return <Addr k={v} />;
  return <span className="x-val">{v}</span>;
}

export function Facts({ facts }: { facts: Fact[] }) {
  return (
    <dl className="kv x-kv">
      {facts.map(([k, v], i) => <FactRow key={`${k}-${i}`} k={k} v={v} />)}
    </dl>
  );
}

function FactRow({ k, v }: { k: string; v: string }) {
  return <><dt>{k}</dt><dd><Val v={v} /></dd></>;
}

const KIND_CHIP: Record<string, string> = { transfer: 'Transfer', swap: 'Swap', poolItems: 'Pool items', raid: 'Raid', settle: 'Settle', war: 'War', market: 'Market', craft: 'Craft', book: 'Book', agents: 'Agents', social: 'Social', armory: 'Armory', launch: 'Launch', memo: 'Memo', other: 'Event' };

export function Story({ lines }: { lines: StoryLine[] }) {
  return (
    <ol className="x-story">
      {lines.map((l, i) => (
        <li key={i} className="x-story-line">
          <div className="x-story-head"><span className={`chip x-k-${l.kind}`}>{KIND_CHIP[l.kind] ?? l.kind}</span><strong>{l.title}</strong>{l.event !== undefined ? <span className="faint">event {l.event + 1}</span> : null}</div>
          <Facts facts={l.facts} />
        </li>
      ))}
    </ol>
  );
}

export function Signatures({ list, empty }: { list: Signature[]; empty: ReactNode }) {
  if (list.length === 0) return <>{empty}</>;
  return (
    <div className="x-scroll">
      <table>
        <thead><tr><th>Signature</th><th className="r">Slot</th><th>When</th><th>Result</th></tr></thead>
        <tbody>{list.map((s) => (
          <tr key={s.signature}>
            <td><Link className="addr" href={`/tx/${s.signature}`} title={s.signature}>{short(s.signature, 8)}</Link>{s.memo ? <div className="faint">{s.memo}</div> : null}</td>
            <td className="r tabular">{s.slot.toLocaleString('en-US')}</td>
            <td className="faint">{ago(s.blockTime)}</td>
            <td>{s.ok ? <span className="chip ok">ok</span> : <span className="chip bad">failed</span>}</td>
          </tr>
        ))}</tbody>
      </table>
    </div>
  );
}

export function SearchBox({ q }: { q?: string }) {
  return (
    <form className="x-search" action="/explorer" method="get" role="search">
      <input className="input" name="q" defaultValue={q} placeholder="Signature, mint, item, template (template 7), passport, wallet or program" aria-label="Search" autoComplete="off" spellCheck={false} />
      <button className="btn primary" type="submit">Search</button>
    </form>
  );
}
