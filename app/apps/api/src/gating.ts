// Changed by Hookwars: new file (gating, docs/spec/18-gating.md). Template supply reads, the
// external gate's reads and prepares, and the premium (Licensed) item flow. Chain state only:
// nothing here is estimated, and a missing account reads as null.
import { PublicKey, type Connection } from '@solana/web3.js';
import { hookwars } from '@hookwars/sdk';
import { createExtras } from './economy-prepares.ts';
import { big, int, pk, PrepareError, type Body, type PrepareDef } from './prepares.ts';

const one = (label: string, programs: PrepareDef['programs'], build: PrepareDef['build']): PrepareDef => ({ programs, label, payer: (b) => pk(b, 'owner'), build });
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

// ---------------------------------------------------------------- prepares --

/** The items program's further accounts a bind carries, per template the app offers on external
 * tokens: the venue readers take the mint's `MintGate`; the item's `Wear` when it wears. */
function bindExtras(templateId: number, mint: PublicKey, hasWear: boolean, item: PublicKey): PublicKey[] {
  const venueReaders = [17, 18, 19, 20, 22, 26];
  let extras: PublicKey[];
  if (venueReaders.includes(templateId)) extras = [hookwars.mintGateAddress(mint)];
  else if (templateId === 39) extras = [];
  else throw new PrepareError(409, 'NotGateable', 'The app binds Cooldown, Daily Sell Cap, Max Transaction, Flash Guard, Dust Guard, Streak and Guild Tag on external tokens.');
  if (hasWear) extras.push(hookwars.wearAddress(item));
  return extras;
}

export const GATING_PREPARES: Record<string, PrepareDef> = {
  'gate/register/prepare': one('Register the mint with the gate', ['gate'], async (b) => {
    const owner = pk(b, 'owner');
    const venue = b.venue === undefined || b.venue === null ? PublicKey.default : pk(b, 'venue');
    const strict = b.strict === undefined || b.strict === null ? null : Boolean(b.strict);
    return [hookwars.gateRegisterMint(owner, owner, pk(b, 'mint'), venue, strict)];
  }),
  'gate/venue/prepare': one('Set the venue', ['gate'], async (b) => {
    const venue = b.venue === undefined || b.venue === null ? PublicKey.default : pk(b, 'venue');
    return [hookwars.gateSetVenue(pk(b, 'owner'), pk(b, 'mint'), venue, Boolean(b.strict))];
  }),
  'gate/bind/prepare': one('Bind an item', ['gate'], async (b, conn) => {
    const owner = pk(b, 'owner'); const mint = pk(b, 'mint'); const itemMint = pk(b, 'itemMint');
    const slot = int(b, 'slot', 0, 3);
    const kind = b.kind === 'owned' ? hookwars.GATE_BINDING_KIND.owned : hookwars.GATE_BINDING_KIND.licence;
    const itemKey = hookwars.itemAddress(itemMint);
    const info = await conn.getAccountInfo(itemKey, 'confirmed');
    if (!info) throw new PrepareError(404, 'NoSuchItem', 'No armory item has this mint.');
    const it = hookwars.itemCodec().decode(info.data) as { templateId: number; hasWear: boolean };
    const proof = kind === hookwars.GATE_BINDING_KIND.licence
      ? hookwars.gateLicenceProof(itemKey, mint, hookwars.MARKET_ID)
      : hookwars.gateVaultHolding(itemMint, mint);
    if (!(await conn.getAccountInfo(proof, 'confirmed'))) {
      throw new PrepareError(409, 'NoProof', kind === hookwars.GATE_BINDING_KIND.licence
        ? 'This mint holds no licence for the item; buy one on the item page first.'
        : 'The gate vault does not hold this item; send it to the vault first.');
    }
    return [hookwars.gateBind(owner, mint, slot, kind, itemKey, proof, [], bindExtras(it.templateId, mint, it.hasWear, itemKey))];
  }),
  'gate/unbind/prepare': one('Unbind an item', ['gate'], async (b, conn) => {
    const mint = pk(b, 'mint'); const slot = int(b, 'slot', 0, 3);
    const info = await conn.getAccountInfo(hookwars.mintGateAddress(mint), 'confirmed');
    if (!info) throw new PrepareError(404, 'NotRegistered', 'This mint is not registered with the gate.');
    const g = hookwars.decodeMintGate(info.data);
    const bnd = g.bindings.find((x) => x.slot === slot);
    if (!bnd) throw new PrepareError(409, 'NotBound', 'Nothing is bound in that slot.');
    return [hookwars.gateUnbind(pk(b, 'owner'), mint, slot, bnd.proof)];
  }),
  'gate/holder/prepare': one('Open holder memory', ['gate'], async (b) => [hookwars.gateOpenHolder(pk(b, 'owner'), pk(b, 'mint'), b.wallet === undefined ? pk(b, 'owner') : pk(b, 'wallet'))]),
  // A premium (Licensed by default) item: created and given its licence terms in one transaction,
  // so it is never listed without terms (spec 18 section 2).
  'items/premium/prepare': one('Create a licensed item', ['armory', 'items', 'token'], async (b, conn) => {
    const owner = pk(b, 'owner'); const templateId = int(b, 'templateId', 1, 65_535);
    const cfgInfo = await conn.getAccountInfo(hookwars.armoryConfigAddress(), 'confirmed');
    if (!cfgInfo) throw new PrepareError(409, 'NoArmoryConfig', 'The armory has no config on this cluster yet.');
    const itemsMinted = hookwars.armoryConfigCodec.decode(cfgInfo.data).itemsMinted;
    const itemMint = hookwars.itemMintAddress(itemsMinted);
    const p = b.params;
    if (!Array.isArray(p) || p.length > 11 || p.some((x) => !Number.isInteger(x) || x < 0 || x > 4_294_967_295)) throw new PrepareError(400, 'BadRequest', '"params" must be a list of at most 11 integers.');
    const params = [...(p as number[]), ...Array<number>(11 - p.length).fill(0)];
    const terms = { priceLamports: big(b, 'priceLamports'), termSecs: int(b, 'termSecs', 1, 4_294_967_295), per: int(b, 'per', 0, 1), maxLive: int(b, 'maxLive', 1, 65_535) };
    const extras = await createExtras(conn, owner, templateId);
    return [
      hookwars.createItemEco(owner, templateId, params, int(b, 'royaltyBps', 0, 10_000), itemsMinted, extras),
      hookwars.setAccess(owner, { itemMint, templateId, holder: owner, mode: 2, exclusive: Boolean(b.exclusive), licenceTerms: terms }),
    ];
  }),
  'items/minter/prepare': one('Hand over the minter', ['armory'], async (b) => [hookwars.handOverMinter(pk(b, 'owner'), int(b, 'templateId', 1, 65_535), pk(b, 'newMinter'))]),
};
