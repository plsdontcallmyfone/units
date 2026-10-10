// Changed by Hookwars: D-4 (17-randomness). Loot rolls and reveals. With Switchboard as the war
// config's randomness program, a roll is Switchboard's `randomness_commit` then `roll` in one
// transaction, and a reveal is Switchboard's `randomness_reveal` then `reveal` in one transaction
// (the war program reads the value only in its reveal slot). The adapter path (the test stub) keeps
// the old shape.
import { PublicKey, type Connection, type TransactionInstruction } from '@solana/web3.js';
import { hookwars } from '@hookwars/sdk';
import { PrepareError, big, pk, raidContext, type PrepareDef } from './prepares.ts';
import { randomnessGateway } from './switchboard.ts';

async function warConfig(conn: Connection) {
  const info = await conn.getAccountInfo(hookwars.WAR_CONFIG, 'confirmed');
  if (!info) throw new PrepareError(409, 'NoWarConfig', 'The war program has no config on this cluster yet.');
  return hookwars.warConfigCodec.decode(info.data);
}

/** The adapter layout (`hookwars_war::oracle::Randomness`): `fulfilled` at 48, `value` at 57. */
function adapterValue(data: Buffer): Uint8Array | null {
  if (data.length < 89 || data[48] !== 1) return null;
  return Uint8Array.from(data.subarray(57, 89));
}

export const LOOT_PREPARES: Record<string, PrepareDef> = {
  'rolls/randomness/prepare': {
    programs: ['war'], label: 'Create a randomness account', payer: (b) => pk(b, 'owner'),
    build: async () => { throw new PrepareError(400, 'UseStagedRoute', 'Creating a randomness account needs its key to sign; it is prepared by the staged route.'); },
    staged: async (b, conn) => {
      const cfg = await warConfig(conn);
      if (!hookwars.isSwitchboard(cfg.randomnessProgram)) {
        throw new PrepareError(409, 'NotSwitchboard', 'The war program on this cluster does not use Switchboard randomness.');
      }
      const ix = await randomnessGateway().create(conn, cfg.randomnessProgram, pk(b, 'randomness'), pk(b, 'owner'));
      return [{ label: 'Create a randomness account', ixs: [ix], extraSigners: ['randomness'], simulate: false, tables: [] }];
    },
  },
  'rolls/prepare': {
    programs: ['war', 'token', 'items'], label: 'Roll', payer: (b) => pk(b, 'owner'),
    build: async (b, conn) => {
      const owner = pk(b, 'owner'); const mint = pk(b, 'mint'); const oracleAccount = pk(b, 'oracleAccount');
      const cfg = await warConfig(conn);
      const { raid, extras } = await raidContext(conn, mint, owner);
      const roll = hookwars.roll(owner, mint, big(b, 'nonce'), raid.slot, { program: cfg.randomnessProgram, account: oracleAccount }, extras);
      if (!hookwars.isSwitchboard(cfg.randomnessProgram)) return [roll];
      const info = await conn.getAccountInfo(oracleAccount, 'confirmed');
      if (!info || !info.owner.equals(cfg.randomnessProgram)) {
        throw new PrepareError(409, 'NoRandomnessAccount', 'Create the randomness account first (rolls/randomness/prepare).');
      }
      if (!hookwars.decodeSbRandomness(info.data).authority.equals(owner)) {
        throw new PrepareError(409, 'RandomnessAuthority', 'The randomness account belongs to another wallet.');
      }
      // Switchboard's commit first: the roll checks it was committed in the previous slot.
      return [await randomnessGateway().commit(conn, cfg.randomnessProgram, oracleAccount, owner), roll];
    },
  },
  'rolls/reveal/prepare': {
    programs: ['war', 'armory', 'items', 'token'], label: 'Reveal loot', payer: (b) => pk(b, 'revealer'),
    build: async (b, conn) => {
      const revealer = pk(b, 'revealer'); const owner = pk(b, 'owner'); const mint = pk(b, 'mint'); const nonce = big(b, 'nonce');
      const rollKey = hookwars.rollAddress(hookwars.holdingAddr(mint, owner), nonce);
      const rollInfo = await conn.getAccountInfo(rollKey, 'confirmed');
      if (!rollInfo) throw new PrepareError(404, 'NoSuchRoll', 'There is no open roll with this nonce.');
      const roll = hookwars.rollRequestCodec.decode(rollInfo.data);
      const [tableInfo, armoryInfo, oracleInfo] = await conn.getMultipleAccountsInfo(
        [hookwars.lootTableAddress(roll.season), hookwars.armoryConfigAddress(), roll.oracleAccount], 'confirmed');
      if (!tableInfo) throw new PrepareError(409, 'NoLootTable', 'The roll\'s season has no loot table.');
      if (!armoryInfo) throw new PrepareError(409, 'NoArmoryConfig', 'The armory has no config on this cluster yet.');
      const front: TransactionInstruction[] = [];
      let value: Uint8Array | null;
      if (hookwars.isSwitchboard(roll.oracleProgram)) {
        const r = await randomnessGateway().reveal(conn, roll.oracleProgram, roll.oracleAccount, revealer);
        front.push(r.ix); value = r.value;
      } else {
        value = oracleInfo ? adapterValue(oracleInfo.data) : null;
      }
      if (!value) throw new PrepareError(409, 'RandomnessNotReady', 'The randomness for this roll is not ready yet.');
      const drawn = hookwars.drawLoot(hookwars.activeLoot(hookwars.decodeLootTable(tableInfo.data)), value);
      if (!drawn) throw new PrepareError(409, 'NoLootTable', 'The roll\'s loot table has no weight.');
      const itemsMinted = hookwars.armoryConfigCodec.decode(armoryInfo.data).itemsMinted;
      const extras = hookwars.mintLootExtras(revealer, roll.owner, drawn.templateId, drawn.params, itemsMinted);
      return [...front, hookwars.reveal(revealer, roll.owner, mint, nonce, roll.season, roll.oracleAccount, extras)];
    },
  },
};

/** The roll a reveal would close, for callers that only hold the roll's address. */
export const rollKeyOf = (mint: PublicKey, owner: PublicKey, nonce: bigint): PublicKey =>
  hookwars.rollAddress(hookwars.holdingAddr(mint, owner), nonce);
