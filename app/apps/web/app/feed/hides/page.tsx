// Changed by Hookwars: new page, the public record of every admin hide (Lineage pattern: hidden, never deleted, always on record).
import Link from 'next/link';
import { readSocial } from '@/lib/social';
import { ago, short } from '@/lib/format';
import { Empty, Head, Panel, ReadFailed } from '@/components/ui';
import { threadHref } from '@/components/post-card';

type Hides = { admins: string[]; items: { message_id: string; admin: string; ref: string; reason: string | null; slot: string; block_time: string | null; ref_author: string | null; ref_text: string | null; applied: boolean }[] };

export default async function HidesPage() {
  const r = await readSocial<Hides>('/v1/social/hides');
  const now = Date.now() / 1000;
  return (
    <>
      <Head eyebrow="Feed" title="Hide record" lede="An admin can hide a post from this site's feeds. The hide is itself a signed memo on chain, so every hide, its reason and who made it stay public here. Only hides by this site's current admins apply." />
      <Panel title="Every hide" meta={r.ok ? (r.data.admins.length ? `${r.data.admins.length} current admin${r.data.admins.length === 1 ? '' : 's'}` : 'no admins set') : undefined} flush>
        {!r.ok ? <ReadFailed what="the hide record" error={r.error} /> : r.data.items.length === 0 ? <Empty title="No hides" what="No admin has hidden a post. Each hide would be listed here with its reason." /> : (
          <div className="table-wrap">
            <table>
              <thead><tr><th>When</th><th>Admin</th><th>Post</th><th>Reason</th><th>Applies</th></tr></thead>
              <tbody>{r.data.items.map((h) => (
                <tr key={h.message_id}>
                  <td className="faint">{ago(h.block_time ? Date.parse(h.block_time) / 1000 : null, now)}</td>
                  <td><Link href={`/u/${h.admin}`}>{short(h.admin, 4)}</Link></td>
                  <td><Link href={threadHref(h.ref)}>{h.ref_text ? h.ref_text.slice(0, 60) : short(h.ref, 6)}</Link>{h.ref_author ? <span className="faint"> by {short(h.ref_author, 4)}</span> : null}</td>
                  <td>{h.reason ?? '-'}</td>
                  <td>{h.applied ? <span className="chip ok">yes</span> : <span className="chip">no, not an admin</span>}</td>
                </tr>
              ))}</tbody>
            </table>
          </div>
        )}
      </Panel>
    </>
  );
}
