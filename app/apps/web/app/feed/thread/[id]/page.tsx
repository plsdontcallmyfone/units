// Changed by Hookwars: new page, one thread: the first message and every reply, oldest first.
import Link from 'next/link';
import { readSocial, type Post } from '@/lib/social';
import { short } from '@/lib/format';
import { Empty, Head, Panel, ReadFailed } from '@/components/ui';
import { PostCard } from '@/components/post-card';
import { Composer, HideButton } from '@/components/social-buttons';

type Thread = { thread: string; focus: string; messages: Post[]; hidden: { message_id: string; admin: string; reason: string }[]; raw: { raw: string; error: string } | null };

export default async function ThreadPage({ params }: { params: Promise<{ id: string }> }) {
  const id = decodeURIComponent((await params).id);
  const r = await readSocial<Thread | null>(`/v1/social/threads/${encodeURIComponent(id)}`);
  const now = Date.now() / 1000;
  const title = `Thread ${short(id.split(':')[0], 5)}`;
  if (!r.ok) {
    return <><Head eyebrow="Feed" title={title} /><Panel title="Thread">{/No such|holds no/.test(r.error) ? <Empty title="No such message" what="The indexer holds no message with this id. It may not be read yet." next={<Link href="/feed">Back to the feed</Link>} /> : <ReadFailed what="this thread" error={r.error} />}</Panel></>;
  }
  const t = r.data;
  if (!t) return <><Head eyebrow="Feed" title={title} /><Panel title="Thread"><Empty title="No such message" what="The indexer holds no message with this id." /></Panel></>;
  return (
    <>
      <Head eyebrow="Feed" title={title} lede="Every message here is a signed memo. A reply names the first message of its thread and the message it answers." right={<Link className="btn sm" href="/feed">Feed</Link>} />
      <div className="grid cols-main">
        <Panel title="Messages" meta={`${t.messages.length} shown`}>
          {t.raw ? <Empty title="This memo did not parse" what={`It is kept as written and not threaded (${t.raw.error}).`} /> : null}
          {t.hidden.length ? <p className="muted" style={{ fontSize: 13 }}>An admin hid this message: {t.hidden.map((h) => h.reason).join('; ')}. The hide is in the <Link href="/feed/hides">public record</Link>.</p> : null}
          {t.messages.length === 0 ? <Empty title="No messages shown" what="Every message in this thread is hidden or below the proof floor." /> : t.messages.map((m) => (
            <div key={m.id}>
              <PostCard p={m} now={now} focus={m.id === t.focus} />
              <HideButton id={m.id} />
            </div>
          ))}
        </Panel>
        <Panel title="Reply">
          <Composer re={t.focus} thread={t.thread} tags={false} />
        </Panel>
      </div>
    </>
  );
}
