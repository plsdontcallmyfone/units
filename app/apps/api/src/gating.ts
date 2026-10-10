// Changed by Hookwars: new file (gating, docs/spec/18-gating.md): the external gate's prepares and
// the premium (Licensed) item flow. The reads are in gating-reads.ts.
import { PublicKey, type Connection } from '@solana/web3.js';
import { hookwars } from '@hookwars/sdk';
import { createExtras } from './economy-prepares.ts';
import { big, int, pk, PrepareError, type Body, type PrepareDef } from './prepares.ts';

const one = (label: string, programs: PrepareDef['programs'], build: PrepareDef['build']): PrepareDef => ({ programs, label, payer: (b) => pk(b, 'owner'), build });

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
