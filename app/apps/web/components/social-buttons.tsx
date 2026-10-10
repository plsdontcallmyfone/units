'use client';
// Changed by Hookwars: new file, the social actions: follow, react, post, reply, hide. Each one is a
// memo the connected wallet signs (11 4.2); it goes through /api/v1/social/<action>/prepare and
// /api/v1/submit like every other action on the site.
import { useEffect, useState } from 'react';
import type { VersionedTransaction } from '@solana/web3.js';
import type { PreparedTx } from '@hookwars/shared';
import { signAndSend } from '@/lib/sign';
import s from './social.module.css';

export function useWallet(): { wallet: string | null; has: boolean | null; connect: () => Promise<string | null> } {
  const [wallet, setWallet] = useState<string | null>(null);
  const [has, setHas] = useState<boolean | null>(null);
  useEffect(() => { setHas(Boolean(window.solana)); if (window.solana?.publicKey) setWallet(window.solana.publicKey.toBase58()); }, []);
  const connect = async () => {
    if (!window.solana) return null;
    try { const r = await window.solana.connect(); const w = r.publicKey.toBase58(); setWallet(w); return w; } catch { return null; }
  };
  return { wallet, has, connect };
}

/** Prepare, sign, send. Returns the signatures, or throws with the backend's sentence. */
async function send(route: string, body: Record<string, unknown>, wallet: string): Promise<string[]> {
  const w = window.solana;
  if (!w?.signAllTransactions) throw new Error('This wallet cannot sign transactions here.');
  const r = await fetch(`/api/v1/social/${route}/prepare`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ ...body, owner: wallet }) });
  const b = await r.json() as { error?: string; transactions?: PreparedTx[] };
  if (!r.ok || !b.transactions) throw new Error(b.error ?? `HTTP ${r.status}`);
  const signer = { signAllTransactions: <T extends VersionedTransaction>(txs: T[]) => w.signAllTransactions!(txs) };
  return signAndSend(signer, b.transactions, {}, () => {});
}

function useAction() {
  const { wallet, has, connect } = useWallet();
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState<{ ok: boolean; text: string } | null>(null);
  const run = async (route: string, body: Record<string, unknown>, done?: () => void) => {
    const w = wallet ?? await connect();
    if (!w) { setNote({ ok: false, text: has === false ? 'No Solana wallet in this browser.' : 'Connect a wallet first.' }); return; }
    setBusy(true); setNote({ ok: true, text: 'Sign in your wallet.' });
    try {
      await send(route, body, w);
      setNote({ ok: true, text: 'Sent. It shows here once the indexer reads it.' });
      done?.();
    } catch (e) { setNote({ ok: false, text: e instanceof Error ? e.message : String(e) }); } finally { setBusy(false); }
  };
  return { wallet, has, connect, busy, note, run };
}

const Note = ({ note }: { note: { ok: boolean; text: string } | null }) => (note ? <p className={note.ok ? 'muted' : 'bad'} role="status" style={{ fontSize: 13, margin: '6px 0 0' }}>{note.text}</p> : null);

/** Follow or unfollow an address (a wallet or a passport). The current state is read from the signed follows. */
export function FollowButton({ target }: { target: string }) {
  const a = useAction();
  const [following, setFollowing] = useState<boolean | null>(null);
  useEffect(() => {
    if (!a.wallet) return;
    fetch(`/api/v1/social/follows/${a.wallet}`).then((r) => (r.ok ? r.json() : null)).then((b: { followingList?: { target: string }[] } | null) => {
      setFollowing(b?.followingList ? b.followingList.some((f) => f.target === target) : null);
    }).catch(() => setFollowing(null));
  }, [a.wallet, target]);
  if (a.wallet === target) return null;
  return (
    <div className={s.inline}>
      <button type="button" className={`btn sm ${following ? '' : 'primary'}`} disabled={a.busy}
        onClick={() => a.run(following ? 'unfollow' : 'follow', { target }, () => setFollowing(!following))}>
        {a.busy ? 'Working...' : following ? 'Unfollow' : 'Follow'}
      </button>
      <Note note={a.note} />
    </div>
  );
}

const LABEL: Record<string, string> = { like: 'Like', useful: 'Useful', disagree: 'Disagree' };

/** The three reactions with their counts (counted from signed reactions, one per wallet each). */
export function ReactBar({ id, counts, replies, threadHref }: { id: string; counts: Record<string, number>; replies?: number; threadHref?: string }) {
  const a = useAction();
  return (
    <div>
      <div className={s.reacts}>
        {Object.keys(LABEL).map((k) => (
          <button key={k} type="button" className={s.react} disabled={a.busy} onClick={() => a.run('react', { ref: id, reaction: k })} aria-label={`${LABEL[k]} (${counts[k] ?? 0})`}>
            {LABEL[k]} <span className={s.count}>{counts[k] ?? 0}</span>
          </button>
        ))}
        {threadHref ? <a className={s.react} href={threadHref}>Replies <span className={s.count}>{replies ?? 0}</span></a> : null}
      </div>
      <Note note={a.note} />
    </div>
  );
}

/** Write a post, or a reply when `re` is set. Optional token and guild tags put it in those feeds. */
export function Composer({ re, thread, tags = true, placeholder }: { re?: string; thread?: string; tags?: boolean; placeholder?: string }) {
  const a = useAction();
  const [text, setText] = useState('');
  const [mint, setMint] = useState('');
  const [guild, setGuild] = useState('');
  const [as, setAs] = useState('');
  const submit = () => {
    if (!text.trim()) return;
    a.run('post', { text, ...(re ? { re, thread: thread ?? re } : {}), ...(mint ? { mint } : {}), ...(guild ? { guild } : {}), ...(as ? { as } : {}) }, () => setText(''));
  };
  return (
    <div className={s.composer}>
      <label className="field" style={{ width: '100%' }}>{re ? 'Reply' : 'Post'}
        <textarea className={s.textarea} value={text} rows={re ? 2 : 3} placeholder={placeholder ?? 'Say what happened. Posts are public memos signed by your wallet.'} onChange={(e) => setText(e.target.value)} />
      </label>
      {tags ? (
        <div className={s.tagRow}>
          <label className="field">Token<span className="faint"> (optional)</span><input value={mint} placeholder="mint address" onChange={(e) => setMint(e.target.value.trim())} /></label>
          <label className="field">Guild<span className="faint"> (optional)</span><input value={guild} inputMode="numeric" placeholder="guild id" onChange={(e) => setGuild(e.target.value.trim())} /></label>
          <label className="field">As passport<span className="faint"> (optional)</span><input value={as} placeholder="your agent's passport" onChange={(e) => setAs(e.target.value.trim())} /></label>
        </div>
      ) : null}
      <div className="action-row">
        {a.has === false ? <span className="muted">No Solana wallet in this browser.</span>
          : <button type="button" className="btn sm primary" disabled={a.busy || !text.trim()} onClick={submit}>{a.busy ? 'Working...' : a.wallet ? (re ? 'Reply' : 'Post') : 'Connect and post'}</button>}
        {a.wallet ? <span className="faint">as {a.wallet.slice(0, 4)}...{a.wallet.slice(-4)}</span> : null}
      </div>
      <Note note={a.note} />
    </div>
  );
}

/** Admin hide: refused by the backend unless the wallet is a current admin; the hide is a public memo. */
export function HideButton({ id }: { id: string }) {
  const a = useAction();
  const [reason, setReason] = useState('');
  return (
    <details className={s.hide}>
      <summary>Admin: hide</summary>
      <div className={s.inline}>
        <input value={reason} placeholder="reason, shown in the public record" onChange={(e) => setReason(e.target.value)} />
        <button type="button" className="btn sm" disabled={a.busy || !reason.trim()} onClick={() => a.run('hide', { ref: id, reason })}>Hide</button>
      </div>
      <Note note={a.note} />
    </details>
  );
}

/** Opens the wallet's profile (social `open_profile`), which starts its counters and levels. */
export function OpenProfile({ wallet }: { wallet: string }) {
  const a = useAction();
  if (a.wallet !== wallet) return <p className="faint" style={{ fontSize: 13, margin: 0 }}>The wallet itself can open its profile here once connected.</p>;
  return (
    <div className={s.inline}>
      <button type="button" className="btn sm primary" disabled={a.busy} onClick={() => a.run('profile', {})}>{a.busy ? 'Working...' : 'Open my profile'}</button>
      <Note note={a.note} />
    </div>
  );
}
