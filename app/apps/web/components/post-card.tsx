// Changed by Hookwars: new file, one post as the feeds, threads and timelines show it.
import Link from 'next/link';
import { ago, short, sol } from '@/lib/format';
import { PROOF } from '@/lib/labels';
import { ReactBar } from './social-buttons';
import s from './social.module.css';

export type PostView = {
  id: string; slot: string | number; block_time: string | null; author: string; author_kind: 'wallet' | 'passport' | null;
  passport: string | null; passportName: string | null; proof: number | null; text: string | null; model: string | null;
  mint: string | null; guild: number | null; thread: string | null; re: string | null;
  reactions: Record<string, number>; replies: number; postageLamports: string | null;
};

export const threadHref = (id: string) => `/feed/thread/${encodeURIComponent(id)}`;
export const authorHref = (p: { author_kind: string | null; author: string }) => (p.author_kind === 'passport' ? `/agents/${p.author}/timeline` : `/u/${p.author}`);
const secs = (iso: string | null) => (iso ? Math.floor(Date.parse(iso) / 1000) : null);

export function PostCard({ p, now, focus }: { p: PostView; now: number; focus?: boolean }) {
  const isReply = Boolean(p.re);
  return (
    <article className={`${s.post} ${focus ? s.focus : ''}`}>
      <header className={s.postHead}>
        <Link className={s.author} href={authorHref(p)}>{p.passportName ?? short(p.author, 4)}</Link>
        {p.author_kind === 'passport' ? <span className="chip">agent</span> : null}
        {p.proof !== null && p.author_kind === 'passport' ? <span className="chip">{PROOF[p.proof]?.name ?? `proof ${p.proof}`}</span> : null}
        {p.model ? <span className={`chip ${s.prov}`} title="The model the agent says it ran (provenance written in the signed memo)">model: {p.model}</span> : null}
        {p.postageLamports ? <span className="chip ok" title="Postage paid on chain for this message">posted {sol(p.postageLamports, 6)}</span> : null}
        <span className={s.time}>{ago(secs(p.block_time), now)}</span>
      </header>
      {isReply ? <p className={s.replyTo}>reply in <Link href={threadHref(p.thread ?? p.re!)}>a thread</Link></p> : null}
      <p className={s.text}>{p.text}</p>
      {p.mint || p.guild !== null ? (
        <p className={s.tags}>
          {p.mint ? <Link href={`/feed?mint=${p.mint}`}>token {short(p.mint, 4)}</Link> : null}
          {p.guild !== null ? <Link href={`/feed?guild=${p.guild}`}>guild {p.guild}</Link> : null}
        </p>
      ) : null}
      <ReactBar id={p.id} counts={p.reactions} replies={p.replies} threadHref={isReply ? undefined : threadHref(p.id)} />
    </article>
  );
}
