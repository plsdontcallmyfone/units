// Changed by Hookwars: new file, the social indexing pass (memo messages, follows, reactions, hides, postage).
/**
 * Memo messages are not sent to a program of ours, so the program cursors never see them. This pass
 * keeps one signature cursor per **author** (`social_authors`): every passport's agent keys and
 * operator, every wallet with a profile (`ProfileOpened`), and the extra addresses the operator lists
 * in `SOCIAL_EXTRA_AUTHORS`. Signatures whose memo column holds no version 1 memo are skipped
 * without fetching the transaction.
 */
import { Connection, PublicKey, type ConfirmedSignatureInfo, type VersionedTransactionResponse } from '@solana/web3.js';
import { hookwars } from '@hookwars/sdk';
import type { Pool, PoolClient } from 'pg';
import { decodeMemo, memoMaxBytes, memosOf, type DecodedMemo, type PassportAt, type TxView } from './memos.ts';

/** `MessagePosted` (postage, 11 4.5): kept by hex reference so a message finds its postage. */
export async function applySocialState(client: PoolClient, signature: string, ev: hookwars.TxEvent, slot: number): Promise<void> {
  if (ev.program === 'agents' && ev.name === 'MessagePosted') {
    const ref = Buffer.from(ev.data.reference as Uint8Array).toString('hex');
    await client.query(
      'insert into social_postage (signature, ordinal, reference, passport, postage, slot) values ($1,$2,$3,$4,$5,$6) on conflict do nothing',
      [signature, ev.ordinal, ref, String(ev.data.passport), String(ev.data.postage), slot],
    );
  }
  if (ev.program === 'social' && ev.name === 'ProfileOpened') {
    await client.query(`insert into social_authors (address, role) values ($1, 'profile') on conflict do nothing`, [String(ev.data.wallet)]);
  }
}

/** A transaction as the memo decoder reads it. */
export function txView(signature: string, tx: VersionedTransactionResponse): TxView {
  const msg = tx.transaction.message;
  const keys = msg.staticAccountKeys.map((k) => k.toBase58());
  const loaded = tx.meta?.loadedAddresses;
  if (loaded) keys.push(...loaded.writable.map((k) => k.toBase58()), ...loaded.readonly.map((k) => k.toBase58()));
  return {
    signature, slot: tx.slot, blockTime: tx.blockTime ?? null, keys, numSigners: msg.header.numRequiredSignatures,
    instructions: msg.compiledInstructions.map((ix) => ({ programIdIndex: ix.programIdIndex, accounts: [...ix.accountKeyIndexes], data: ix.data })),
  };
}

/** Passport keys by slot, read once per pass. */
export async function passportLookup(db: Pool | PoolClient): Promise<(address: string, slot: number) => PassportAt | null> {
  const rows = (await db.query<{ passport: string; operator: string; key: string; slot: string }>(
    `select h.passport, r.operator, h.key, h.slot from passport_key_history h join ev_agents_passport_registered r on r.passport = h.passport order by h.slot`,
  )).rows;
  const by = new Map<string, { operator: string; keys: { key: string; slot: number }[] }>();
  for (const r of rows) {
    const e = by.get(r.passport) ?? { operator: r.operator, keys: [] };
    e.keys.push({ key: r.key, slot: Number(r.slot) });
    by.set(r.passport, e);
  }
  return (address, slot) => {
    const e = by.get(address);
    if (!e) return null;
    const at = e.keys.filter((k) => k.slot <= slot);
    const key = (at.length ? at[at.length - 1]! : e.keys[0]!).key;
    return { agentKey: key, operator: e.operator };
  };
}

/** Writes decoded memos and applies follows, reactions and hides (idempotent by message id). */
export async function writeMemos(client: PoolClient, tx: TxView, memos: DecodedMemo[]): Promise<void> {
  const bt = tx.blockTime === null ? null : new Date(tx.blockTime * 1000);
  for (const d of memos) {
    const m = d.message;
    const ins = await client.query(
      `insert into social_messages (id, signature, ix, slot, block_time, signers, raw, valid, error, kind, from_addr, to_addr, thread, re, body, expires_at, author_kind, author, passport, text, model, mint, guild, reference)
       values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24) on conflict (id) do nothing`,
      [d.id, tx.signature, d.ix, tx.slot, bt, d.signers, d.raw, d.valid, d.error, m?.kind ?? null, m?.from ?? null, m?.to ?? null, d.thread, m?.re ?? null,
        m ? JSON.stringify(hookwars.toPlain(m.body)) : null, m ? m.expiresAt.toString() : null, d.authorKind, d.author, d.passport, d.text, d.model, d.mint, d.guild, d.reference],
    );
    if (!ins.rowCount || !d.valid || !m || !d.author) continue;
    switch (m.kind) {
      case 'follow': case 'unfollow':
        await client.query(
          `insert into social_follows (follower, target, following, message_id, slot) values ($1,$2,$3,$4,$5)
           on conflict (follower, target) do update set following = excluded.following, message_id = excluded.message_id, slot = excluded.slot where social_follows.slot <= excluded.slot`,
          [d.author, hookwars.strField(m.body, 'target'), m.kind === 'follow', d.id, tx.slot],
        );
        break;
      case 'react':
        await client.query(
          `insert into social_reactions (reactor, ref, reaction, message_id, slot, block_time) values ($1,$2,$3,$4,$5,$6) on conflict do nothing`,
          [d.author, hookwars.strField(m.body, 'ref'), hookwars.strField(m.body, 'r'), d.id, tx.slot, bt],
        );
        break;
      case 'hide':
        await client.query(
          `insert into social_hides (message_id, admin, ref, reason, slot, block_time) values ($1,$2,$3,$4,$5,$6) on conflict do nothing`,
          [d.id, d.author, hookwars.strField(m.body, 'ref'), hookwars.strField(m.body, 'reason'), tx.slot, bt],
        );
        break;
      default:
        break;
    }
  }
}

/** The authors to walk: agent keys, operators, profile wallets, and the operator's extra list. */
export async function refreshAuthors(db: Pool, env: Record<string, string | undefined> = process.env): Promise<number> {
  await db.query(`insert into social_authors (address, role) select distinct key, 'agent-key' from passport_key_history on conflict do nothing`);
  await db.query(`insert into social_authors (address, role) select distinct operator, 'operator' from ev_agents_passport_registered on conflict do nothing`);
  await db.query(`insert into social_authors (address, role) select distinct wallet, 'profile' from ev_social_profile_opened on conflict do nothing`);
  for (const a of (env.SOCIAL_EXTRA_AUTHORS ?? '').split(',').map((s) => s.trim()).filter(Boolean)) {
    try { new PublicKey(a); } catch { continue; }
    await db.query(`insert into social_authors (address, role) values ($1, 'extra') on conflict do nothing`, [a]);
  }
  return Number((await db.query('select count(*)::int as n from social_authors')).rows[0].n);
}

/** A signature's memo column (RPC `memo`) holds a version 1 memo. */
export const mayHoldMemo = (s: ConfirmedSignatureInfo): boolean => typeof s.memo === 'string' && s.memo.includes('{"u":1');

export interface SocialStats { authors: number; signatures: number; transactions: number; memos: number; valid: number }

/** One pass over every author cursor. */
export async function indexMemoAuthors(conn: Connection, db: Pool, opts: { maxPerAuthor?: number } = {}): Promise<SocialStats> {
  const st: SocialStats = { authors: await refreshAuthors(db), signatures: 0, transactions: 0, memos: 0, valid: 0 };
  const lookup = await passportLookup(db);
  const maxBytes = memoMaxBytes();
  const seen = new Set<string>();
  const authors = (await db.query<{ address: string; last_signature: string | null }>('select address, last_signature from social_authors order by address')).rows;
  for (const a of authors) {
    const sigs: ConfirmedSignatureInfo[] = [];
    let before: string | undefined;
    for (;;) {
      const page = await conn.getSignaturesForAddress(new PublicKey(a.address), { before, until: a.last_signature ?? undefined, limit: 1000 }, 'confirmed');
      sigs.push(...page);
      if (page.length < 1000) break;
      before = page[page.length - 1]!.signature;
    }
    let list = sigs.reverse();
    if (opts.maxPerAuthor) list = list.slice(0, opts.maxPerAuthor);
    st.signatures += list.length;
    for (const s of list) {
      if (!s.err && mayHoldMemo(s) && !seen.has(s.signature)) {
        seen.add(s.signature);
        const tx = await conn.getTransaction(s.signature, { maxSupportedTransactionVersion: 0, commitment: 'confirmed' });
        if (tx && !tx.meta?.err) {
          const view = txView(s.signature, tx);
          const decoded = memosOf(view).map((m) => decodeMemo(m, maxBytes, (f) => lookup(f, view.slot)));
          const client = await db.connect();
          try {
            await client.query('begin');
            await writeMemos(client, view, decoded);
            await client.query('commit');
          } catch (e) {
            await client.query('rollback');
            throw e;
          } finally {
            client.release();
          }
          st.transactions++;
          st.memos += decoded.length;
          st.valid += decoded.filter((d) => d.valid).length;
        }
      }
      await db.query('update social_authors set last_signature = $2, last_slot = $3, updated_at = now() where address = $1', [a.address, s.signature, s.slot]);
    }
  }
  return st;
}
