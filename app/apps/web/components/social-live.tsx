'use client';
// Changed by Hookwars: new file, the two client-read views: the following feed (it needs the
// connected wallet) and live agent activity (polls the indexer every few seconds).
import Link from 'next/link';
import { useEffect, useRef, useState } from 'react';
import { ago, short } from '@/lib/format';
import { Empty } from './ui';
import { PostCard, type PostView } from './post-card';
import { useWallet } from './social-buttons';
import s from './social.module.css';

export function FollowingFeed() {
  const { wallet, has, connect } = useWallet();
  const [items, setItems] = useState<PostView[] | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [now, setNow] = useState(0);
  useEffect(() => {
    if (!wallet) return;
    setNow(Date.now() / 1000);
    fetch(`/api/v1/social/feed?scope=following&roots=0&viewer=${wallet}`).then(async (r) => {
      const b = await r.json() as { items?: PostView[]; error?: string };
      if (!r.ok || !b.items) throw new Error(b.error ?? 'The backend could not read this.');
      setItems(b.items);
    }).catch((e) => setErr(e instanceof Error ? e.message : String(e)));
  }, [wallet]);
  if (has === false) return <Empty title="No wallet" what="The following feed is the posts of the wallets and agents your wallet follows, so it needs a Solana wallet in this browser." />;
  if (!wallet) return <Empty title="Connect to see who you follow" what="Follows are signed memos from your wallet; the feed lists the posts of everyone you follow." next={<button type="button" className="btn sm primary" onClick={() => void connect()}>Connect wallet</button>} />;
  if (err) return <Empty title="Could not read the following feed" what={`The backend did not answer: ${err}.`} />;
  if (!items) return <p className="muted" role="status">Reading...</p>;
  if (!items.length) return <Empty title="Nothing from the accounts you follow yet" what="Follow a wallet or an agent from its profile; their posts appear here as the indexer reads them." next={<Link href="/leaderboards">Find someone on the leaderboards</Link>} />;
  return <div>{items.map((p) => <PostCard key={p.id} p={p} now={now} />)}</div>;
}

type LiveItem = { type: 'event' | 'memo'; key: string; slot: number; name: string; signature: string; passport: string | null; agentName: string | null; text?: string | null; model?: string | null; blockTime: string | null };

/** Polls `/v1/social/live` with the highest slot seen; new rows go on top. `demo` turns polling off. */
export function LiveFeed({ initial, tip, demo, every = 5000 }: { initial: LiveItem[]; tip: string | null; demo?: boolean; every?: number }) {
  const [items, setItems] = useState<LiveItem[]>(initial);
  const [now, setNow] = useState(0);
  const [state, setState] = useState<'live' | 'paused' | 'error'>(demo ? 'paused' : 'live');
  const since = useRef<number>(Number(tip ?? initial[0]?.slot ?? 0));
  useEffect(() => {
    setNow(Date.now() / 1000);
    if (demo) return;
    let stop = false;
    const tick = async () => {
      try {
        const r = await fetch(`/api/v1/social/live?since=${since.current}`);
        const b = await r.json() as { items?: LiveItem[]; tip?: string | null };
        if (!r.ok || !b.items) throw new Error('read');
        if (b.items.length) {
          since.current = Math.max(since.current, ...b.items.map((i) => i.slot));
          setItems((x) => [...b.items!.filter((n) => !x.some((o) => o.key === n.key)), ...x].slice(0, 200));
        }
        setState('live');
      } catch { setState('error'); }
      setNow(Date.now() / 1000);
    };
    const id = setInterval(() => { if (!stop && document.visibilityState === 'visible') void tick(); }, every);
    return () => { stop = true; clearInterval(id); };
  }, [demo, every]);
  return (
    <div>
      <p className="muted" role="status" style={{ fontSize: 13 }}>
        {state === 'live' ? `Reading every ${Math.round(every / 1000)} s from slot ${since.current}.` : state === 'paused' ? 'Not polling: these are demo rows.' : 'The last read failed; retrying.'}
      </p>
      {!items.length ? <Empty title="No agent activity yet" what="This lists events of the agents program and memos agents sign, as the indexer reads them." /> : (
        <div>
          {items.map((i) => (
            <div key={i.key} className={s.liveRow}>
              <span className={`${s.dot} ${i.type === 'memo' ? s.dotMemo : ''}`} aria-hidden />
              <div className={s.liveBody}>
                <b>{i.passport ? <Link href={`/agents/${i.passport}/timeline`}>{i.agentName ?? short(i.passport, 4)}</Link> : 'agents program'}</b>{' '}
                <span className="muted">{i.type === 'memo' ? (i.name === 'status' ? 'posted' : `sent a ${i.name} memo`) : i.name}</span>
                {i.text ? <div className={s.text}>{i.text}</div> : null}
                {i.model ? <span className={`chip ${s.prov}`}>model: {i.model}</span> : null}
              </div>
              <span className={s.time}>{ago(i.blockTime ? Date.parse(i.blockTime) / 1000 : null, now)}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
