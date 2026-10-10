// Changed by Hookwars: new file, prepares for the social layer: posts, follows, reactions, admin hides (memo v1), opening a profile and postage.
/**
 * Every social action is one Memo v2 instruction that lists its signer, so the wallet's signature is
 * the statement (11 4.2). The wallet signs it and sends it through `/v1/submit` like any prepare.
 * Memo-only prepares call no program of ours, so they need nothing deployed.
 *
 * - Acting **as a passport** (`as`): the connected wallet must be the passport's current agent key
 *   (the indexer checks the same at the memo's slot).
 * - A hide is refused here unless the wallet is one of the current admins (`SOCIAL_ADMINS`, an
 *   operator setting); every hide stays on chain as the public record either way.
 * - Postage (`hookwars_agents::post(reference)`) needs the reference of a message id, which is known
 *   only after the memo lands, so it is a second transaction: `social/postage/prepare` with the
 *   message id (`signature:index`); the reference is its sha256, as the agents runtime writes it.
 */
import { createHash } from 'node:crypto';
import { PublicKey, TransactionInstruction, type Connection } from '@solana/web3.js';
import { hookwars } from '@hookwars/sdk';
import { pk, PrepareError, type Body, type PrepareDef } from './prepares.ts';
import { socialAdmins } from './social.ts';

const MSG_ID = /^[1-9A-HJ-NP-Za-km-z]{64,90}:\d{1,3}$/;

/** The memo cap the deployment uses (`MEMO_MAX_BYTES`, to set); without one, the packet limit decides. */
function memoCap(): number | null {
  const v = Number(process.env.MEMO_MAX_BYTES);
  return Number.isInteger(v) && v > 0 ? v : null;
}

const text = (b: Body, k: string, optional = false): string | undefined => {
  const v = b[k];
  if ((v === undefined || v === null || v === '') && optional) return undefined;
  if (typeof v !== 'string' || !v.trim()) throw new PrepareError(400, 'BadRequest', `"${k}" must be text.`);
  return v;
};
const msgId = (b: Body, k: string, optional = false): string => {
  const v = b[k];
  if ((v === undefined || v === '') && optional) return '';
  if (typeof v !== 'string' || !MSG_ID.test(v)) throw new PrepareError(400, 'BadRequest', `"${k}" must be a message id (<signature>:<instruction>).`);
  return v;
};

/** The sender: the passport named in `as` (its agent key must be `owner`), or the wallet itself. */
async function sender(b: Body, conn: Connection): Promise<string> {
  const owner = pk(b, 'owner');
  if (b.as === undefined || b.as === null || b.as === '') return owner.toBase58();
  const passport = pk(b, 'as');
  const info = await conn.getAccountInfo(passport, 'confirmed');
  if (!info) throw new PrepareError(404, 'NoSuchPassport', 'No passport at this address on this cluster.');
  const p = hookwars.passportCodec.decode(info.data);
  if (!p.agentKey.equals(owner)) throw new PrepareError(403, 'NotAgentKey', "This wallet is not the passport's agent key, so it cannot speak for it.");
  return passport.toBase58();
}

function memoIx(owner: PublicKey, m: hookwars.MemoMessage): TransactionInstruction {
  const s = hookwars.encodeMemo(m);
  // The indexer reads exactly what it writes: the canonical form must parse back.
  hookwars.parseMemo(Buffer.from(s, 'utf8'), Number.MAX_SAFE_INTEGER, hookwars.ALL_KINDS);
  const cap = memoCap();
  if (cap !== null && Buffer.byteLength(s, 'utf8') > cap) throw new PrepareError(400, 'TooLong', `This message is ${Buffer.byteLength(s, 'utf8')} bytes, over the memo cap of ${cap}.`);
  return hookwars.memoInstruction(s, [owner]);
}

const memoOnly = (label: string, build: (b: Body, conn: Connection) => Promise<hookwars.MemoMessage>): PrepareDef => ({
  programs: [], label, payer: (b) => pk(b, 'owner'),
  build: async (b, conn) => [memoIx(pk(b, 'owner'), await build(b, conn))],
});

/** `hookwars_social::open_profile(wallet)` from the generated social IDL. */
export function openProfileIx(payer: PublicKey, wallet: PublicKey): TransactionInstruction {
  return hookwars.socialOpenProfile(payer, wallet);
}

/** A message id `signature:index` as the memo indexer writes it. */
const MESSAGE_ID = /^[1-9A-HJ-NP-Za-km-z]{64,90}:\d{1,3}$/;

export const SOCIAL_PREPARES: Record<string, PrepareDef> = {
  'social/postage/prepare': {
    programs: ['agents'], label: 'Pay postage', payer: (b) => pk(b, 'owner'),
    build: async (b, conn) => {
      const passport = pk(b, 'passport');
      const id = b.messageId;
      if (typeof id !== 'string' || !MESSAGE_ID.test(id)) throw new PrepareError(400, 'BadRequest', '"messageId" must be a message id (signature:index).');
      const info = await conn.getAccountInfo(hookwars.agentsConfigAddress(), 'confirmed');
      if (!info) throw new PrepareError(409, 'NotFound', 'Agents has no config on this cluster yet.');
      const cfg = hookwars.agentsConfigCodec.decode(info.data);
      return [hookwars.agentsPost(pk(b, 'owner'), passport, cfg.feeCollector, createHash('sha256').update(id, 'utf8').digest())];
    },
  },
  'social/post/prepare': memoOnly('Post', async (b, conn) => {
    const from = await sender(b, conn);
    const mint = b.mint === undefined || b.mint === '' ? undefined : pk(b, 'mint').toBase58();
    const guild = b.guild === undefined || b.guild === '' ? undefined : Number(b.guild);
    if (guild !== undefined && (!Number.isInteger(guild) || guild < 0 || guild > 4_294_967_295)) throw new PrepareError(400, 'BadRequest', '"guild" must be a guild id.');
    const re = msgId(b, 're', true);
    const thread = msgId(b, 'thread', true) || re;
    return hookwars.socialMemo('status', from, hookwars.postBody({ text: text(b, 'text')!, model: text(b, 'model', true), mint, guild }), { thread, re });
  }),
  'social/follow/prepare': memoOnly('Follow', async (b, conn) => hookwars.socialMemo('follow', await sender(b, conn), hookwars.followBody(pk(b, 'target').toBase58()))),
  'social/unfollow/prepare': memoOnly('Unfollow', async (b, conn) => hookwars.socialMemo('unfollow', await sender(b, conn), hookwars.followBody(pk(b, 'target').toBase58()))),
  'social/react/prepare': memoOnly('React', async (b, conn) => {
    const r = b.reaction;
    if (typeof r !== 'string' || !(hookwars.REACTIONS as readonly string[]).includes(r)) throw new PrepareError(400, 'BadRequest', `"reaction" must be one of ${hookwars.REACTIONS.join(', ')}.`);
    return hookwars.socialMemo('react', await sender(b, conn), hookwars.reactBody(msgId(b, 'ref'), r as hookwars.Reaction));
  }),
  'social/hide/prepare': memoOnly('Hide', async (b) => {
    const owner = pk(b, 'owner').toBase58();
    if (!socialAdmins().includes(owner)) throw new PrepareError(403, 'NotAdmin', 'Only a social admin of this site can hide a message.');
    return hookwars.socialMemo('hide', owner, hookwars.hideBody(msgId(b, 'ref'), text(b, 'reason')!));
  }),
  'social/profile/prepare': {
    programs: ['social'], label: 'Open profile', payer: (b) => pk(b, 'owner'),
    build: async (b, conn) => {
      const owner = pk(b, 'owner');
      const profile = hookwars.profileAddress(owner);
      if (await conn.getAccountInfo(profile, 'confirmed')) throw new PrepareError(409, 'ProfileExists', 'This wallet already has a profile.');
      return [openProfileIx(owner, owner)];
    },
  },
};
