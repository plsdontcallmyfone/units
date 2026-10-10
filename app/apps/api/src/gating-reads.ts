// Changed by Hookwars: new file (gating, docs/spec/18-gating.md): template supply and external gate
// reads. Chain state only: nothing here is estimated, and a missing account reads as null. Kept apart
// from the prepares (gating.ts) so the server can import it without entering the prepares' import cycle.
import { PublicKey, type Connection } from '@solana/web3.js';
import { hookwars } from '@hookwars/sdk';
import { PrepareError } from './prepares.ts';

const json = (v: unknown): unknown => JSON.parse(JSON.stringify(v, (_k, x) => (typeof x === 'bigint' ? x.toString() : x instanceof PublicKey ? x.toBase58() : x)));

// ---------------------------------------------------------------- reads --

/** A template's supply: tracked or not, the cap, the counters and what is left (spec 18 section 1). */
export async function templateSupply(conn: Connection, templateId: number): Promise<unknown> {
  const [t, s] = await conn.getMultipleAccountsInfo([hookwars.templateAddress(templateId), hookwars.supplyAddress(templateId)], 'confirmed');
  if (!t) return null;
  if (!s) return { templateId, tracked: false };
  const v = hookwars.decodeSupply(s.data);
  return json({
    templateId,
    tracked: true,
    maxSupply: v.maxSupply === 0 ? null : v.maxSupply,
    lootReserve: v.lootReserve,
    issued: v.issued,
    drops: v.drops,
    forged: v.forged,
    burned: v.burned,
    made: v.issued + v.drops,
    circulating: hookwars.circulating(v),
    authorRoom: hookwars.authorRoom(v),
    dropRoom: hookwars.dropRoom(v),
    minterRule: v.minterRule === hookwars.MINTER_RULE.authorOnly ? 'authorOnly' : 'openUntilCap',
    minter: v.minter,
  });
}

/** Every tracked template's supply (one row each). */
export async function supplies(conn: Connection): Promise<unknown> {
  const rows = await conn.getProgramAccounts(hookwars.ARMORY_ID, { commitment: 'confirmed', filters: [
    { memcmp: { offset: 0, bytes: Buffer.from(hookwars.idlAccountCodec('armory', 'Supply').disc).toString('base64'), encoding: 'base64' } },
  ] });
  return json(rows.map((r) => {
    const v = hookwars.decodeSupply(r.account.data);
    return { templateId: v.templateId, maxSupply: v.maxSupply === 0 ? null : v.maxSupply, made: v.issued + v.drops, circulating: hookwars.circulating(v), authorRoom: hookwars.authorRoom(v) };
  }).sort((a, b) => a.templateId - b.templateId));
}

/** A Token-2022 mint's gate: whether its transfer hook is the gate, its registration, and each
 * binding with its proof's state now (spec 18 section 3). */
export async function gateOf(conn: Connection, mintStr: string): Promise<unknown> {
  let mint: PublicKey;
  try { mint = new PublicKey(mintStr); } catch { throw new PrepareError(400, 'BadRequest', 'Not a mint address.'); }
  const [mintInfo, gateInfo] = await conn.getMultipleAccountsInfo([mint, hookwars.mintGateAddress(mint)], 'confirmed');
  if (!mintInfo) return null;
  const isT22 = mintInfo.owner.equals(hookwars.TOKEN_2022_ID);
  const hookProgram = isT22 ? transferHookProgram(mintInfo.data) : null;
  const base = { mint, token2022: isT22, hookProgram, hookedByGate: hookProgram?.equals(hookwars.GATE_ID) ?? false };
  if (!gateInfo) return json({ ...base, registered: false });
  const g = hookwars.decodeMintGate(gateInfo.data);
  const proofs = g.bindings.length ? await conn.getMultipleAccountsInfo(g.bindings.map((b) => b.proof), 'confirmed') : [];
  const now = BigInt(Math.floor(Date.now() / 1000));
  const bindings = g.bindings.map((b, i) => ({
    ...b,
    kind: b.kind === hookwars.GATE_BINDING_KIND.licence ? 'licence' : 'owned',
    live: proofLive(b, proofs[i]?.data ?? null, now),
  }));
  return json({ ...base, registered: true, authority: g.authority, venue: g.venue.equals(PublicKey.default) ? null : g.venue, strict: g.strict, bindings });
}

/** The `TransferHook` extension's program, from a Token-2022 mint's bytes. */
export function transferHookProgram(d: Buffer): PublicKey | null {
  if (d.length <= 165 || d[165] !== 1) return null;
  let at = 166;
  while (at + 4 <= d.length) {
    const ty = d.readUInt16LE(at); const len = d.readUInt16LE(at + 2); const start = at + 4;
    if (ty === 0 || start + len > d.length) return null;
    if (ty === 14 && len >= 64) {
      const k = new PublicKey(d.subarray(start + 32, start + 64));
      return k.equals(PublicKey.default) ? null : k;
    }
    at = start + len;
  }
  return null;
}

function proofLive(b: hookwars.GateBinding, data: Buffer | null, now: bigint): boolean | null {
  if (!data) return false;
  if (b.kind === hookwars.GATE_BINDING_KIND.licence) {
    // License: disc, bump, item, token_mint, payer, price_paid, starts_at, ends_at (121), revoked_at (129).
    if (data.length < 137) return null;
    const ends = data.readBigInt64LE(121); const revoked = data.readBigInt64LE(129);
    return now < ends && (revoked === 0n || now < revoked);
  }
  try { return hookwars.holdingCodec.decode(data).amount === 1n; } catch { return null; }
}

