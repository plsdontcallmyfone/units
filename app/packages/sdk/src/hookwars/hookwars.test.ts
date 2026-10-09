import { describe, expect, it } from 'vitest';
import { Keypair, PublicKey } from '@solana/web3.js';
import { FIXED_ADDRESSES, HOOK_SIGNERS, PROGRAM_IDS } from '@hookwars/shared';
import { encodeHookAccountList } from '../hooks.ts';
import {
  ARMORY_ID, ITEMS_ID, LAUNCH_ID, MAX_SLOTS, MINT_FIELDS, PARAM_FIELDS, SLOT, TOKEN_ID, WAR_ID, activeSlots, decodeCpiEvent, decodeObservations,
  decodeRaidLedger, decodeSlotMint, decodeWarState, discriminator, encode, encodeEventBody, EVENT_IX_TAG, holdingCodec, hookwarsIx,
  isCalled, itemCodec, itemLogEvents, lockedAt, logsTruncated, resolveSlices, sliceAccounts, slotRegistryAddress, tokenHookSigner,
  type SlotData, type SlotMintData,
} from './index.ts';

const pda = (seeds: (string | PublicKey)[], program: PublicKey) =>
  PublicKey.findProgramAddressSync(seeds.map((x) => (typeof x === 'string' ? Buffer.from(x) : x.toBuffer())), program);

describe('fixed addresses', () => {
  it('each is derived again from the program ids', () => {
    const cases: [string, (string | PublicKey)[], string][] = [
      [FIXED_ADDRESSES.armoryEventAuthority, ['__event_authority'], PROGRAM_IDS.armory],
      [FIXED_ADDRESSES.armorySigner, ['armory'], PROGRAM_IDS.armory],
      [FIXED_ADDRESSES.itemsEventAuthority, ['__event_authority'], PROGRAM_IDS.items],
      [FIXED_ADDRESSES.warEventAuthority, ['__event_authority'], PROGRAM_IDS.war],
      [FIXED_ADDRESSES.warConfig, ['war-config'], PROGRAM_IDS.war],
      [FIXED_ADDRESSES.warSigner, ['war-signer'], PROGRAM_IDS.war],
      [FIXED_ADDRESSES.lootSigner, ['loot-signer'], PROGRAM_IDS.war],
      [FIXED_ADDRESSES.prizeVault, ['prize-vault'], PROGRAM_IDS.war],
      [FIXED_ADDRESSES.launchConfig, ['config'], PROGRAM_IDS.launch],
      [FIXED_ADDRESSES.bridgedSolMint, ['wrapped', new PublicKey('So11111111111111111111111111111111111111112')], PROGRAM_IDS.bridge],
      [HOOK_SIGNERS.tokenForItems, ['hook-authority', new PublicKey(PROGRAM_IDS.items)], PROGRAM_IDS.token],
      [HOOK_SIGNERS.launchForItems, ['hook-authority', new PublicKey(PROGRAM_IDS.items)], PROGRAM_IDS.launch],
      [HOOK_SIGNERS.tokenForKit, ['hook-authority', new PublicKey(PROGRAM_IDS.kit)], PROGRAM_IDS.token],
    ];
    for (const [want, seeds, program] of cases) expect(pda(seeds, new PublicKey(program))[0].toBase58()).toBe(want);
  });
  it('program ids are the Hookwars keys, never Bordrless', () => {
    expect([TOKEN_ID, LAUNCH_ID, ARMORY_ID, ITEMS_ID, WAR_ID].map(String)).toEqual([PROGRAM_IDS.token, PROGRAM_IDS.launch, PROGRAM_IDS.armory, PROGRAM_IDS.items, PROGRAM_IDS.war]);
    expect(Object.values(PROGRAM_IDS)).not.toContain('2XoEWp8cF3kRXg74eVwPAyTFhVCAztn3V88komxAvr22');
  });
});

const k = () => Keypair.generate().publicKey;
const emptySlot = (): SlotData => ({
  kind: 0, equipRule: 0, bounds: { maxCutBps: 0, mayRefuse: false, mayWriteData: false, mayAnswerTouch: false }, dataOffset: 0, dataLen: 0,
  item: PublicKey.default, program: PublicKey.default, flags: 0, poolFlags: 0, equipVault: PublicKey.default, signerBump: 0, launchSignerBump: 0, dataEpoch: 0, extraCount: 0,
});

function mintBytes(m: SlotMintData, pad = 0): Buffer {
  return Buffer.concat([discriminator('account', 'Mint'), encode({ struct: MINT_FIELDS }, m), Buffer.alloc(pad)]);
}

describe('token Mint with slots (M1 layout)', () => {
  const kitSlot: SlotData = { ...emptySlot(), kind: 5, equipRule: 0, dataLen: 32, program: k(), flags: 1 | 64 | 128, extraCount: 0 };
  const raidSlot: SlotData = { ...emptySlot(), kind: 4, equipRule: 1, dataOffset: 32, dataLen: 12, item: k(), program: ITEMS_ID, flags: 1, extraCount: 2, dataEpoch: 1, bounds: { maxCutBps: 300, mayRefuse: false, mayWriteData: true, mayAnswerTouch: true } };
  const warSlot: SlotData = { ...emptySlot(), kind: 6, item: k(), program: ITEMS_ID };
  const m: SlotMintData = {
    version: 1, decimals: 6, supply: 1_000_000_000_000_000n, maxSupply: 0n, mintAuthority: null, freezeAuthority: null, hookAuthority: null, metadataAuthority: k(),
    hookProgram: null, hookFlags: 0, name: 'Raider', symbol: 'RAID', uri: 'https://example.invalid/r.json', createdAt: 1_700_000_000n, creator: k(),
    hookSignerBump: 0, reserved: Buffer.alloc(31), slotAuthority: k(), slotCount: 3, slots: [kitSlot, raidSlot, warSlot, emptySlot()],
  };
  it('round-trips and lists active slots', () => {
    const d = decodeSlotMint(mintBytes(m, 300));
    expect(d.symbol).toBe('RAID');
    expect(d.slots).toHaveLength(MAX_SLOTS);
    expect(activeSlots(d)).toHaveLength(3);
    expect(d.slots[1]!.bounds.maxCutBps).toBe(300);
    expect(d.slots[1]!.item.equals(raidSlot.item)).toBe(true);
  });
  it('a slot is 113 bytes', () => {
    expect(encode(SLOT, emptySlot()).length).toBe(113);
  });
  it('calls slots as M1 does', () => {
    expect(isCalled(kitSlot, 'transfer')).toBe(true);
    expect(isCalled(raidSlot, 'transfer')).toBe(true);
    expect(isCalled(raidSlot, 'protocolTransfer')).toBe(false);
    expect(isCalled(kitSlot, 'protocolTransfer')).toBe(true);
    expect(isCalled(warSlot, 'transfer')).toBe(false);
  });
  it('resolves slices [program, signer, extras] in slot order', () => {
    const mint = k(), source = k(), destination = k(), authority = k();
    const extra1 = k(), extra2 = k();
    const reg = encodeHookAccountList({ version: 1, accounts: [{ writable: true, source: { kind: 'key', key: extra1 } }, { writable: false, source: { kind: 'key', key: extra2 } }] });
    const regs = new Map([[slotRegistryAddress(raidSlot, mint).toBase58(), reg]]);
    const s = resolveSlices(m, { mint, source, destination, authority }, regs);
    expect(s.slices.map((x) => x.slot)).toEqual([0, 1]);
    const metas = sliceAccounts(s);
    expect(metas.map((x) => x.pubkey.toBase58())).toEqual([
      kitSlot.program.toBase58(), tokenHookSigner(kitSlot.program).toBase58(),
      ITEMS_ID.toBase58(), tokenHookSigner(ITEMS_ID).toBase58(), extra1.toBase58(), extra2.toBase58(),
    ]);
    expect(() => resolveSlices(m, { mint, source, destination, authority }, new Map())).toThrow(/registry is missing/);
  });
});

describe('Holding with the vote lock', () => {
  it('decodes and applies locked_at', () => {
    const h = { version: 1, bump: 255, mint: k(), owner: k(), amount: 500n, delegate: null, delegatedAmount: 0n, frozen: false, hookData: Buffer.alloc(64), voteLocked: 200n, voteLockUntil: 1000n };
    const d = holdingCodec.decode(holdingCodec.encode(h));
    expect(d.amount).toBe(500n);
    expect(lockedAt(d, 999n)).toBe(200n);
    expect(lockedAt(d, 1000n)).toBe(0n);
  });
});

describe('spec-layout accounts', () => {
  it('Item round-trips with PARAM_FIELDS params', () => {
    const c = itemCodec();
    const item = {
      version: 1, bump: 254, itemMint: k(), templateId: 1, params: Array.from({ length: PARAM_FIELDS }, (_, i) => i),
      manifest: { kind: 4, tokenFlags: 257, poolFlags: 7, maxCutBuyBps: 100, maxCutSellBps: 0, maxCutTransferBps: 0, maxDiscountBps: 500, mayRefuse: false, mayBurn: false, dataBytes: 11, readsOtherPools: 1 },
      author: k(), royaltyBps: 250, level: 1, source: 0, equippedCount: 0, royaltyOwnerBump: 253, createdAt: 5n,
    };
    expect(c.decode(c.encode(item))).toMatchObject({ templateId: 1, royaltyBps: 250, level: 1 });
  });
  it('variable-length rings and tables are sized from the data', () => {
    const OBS = Buffer.concat([discriminator('account', 'Observations'), Buffer.alloc(1 + 1 + 32 + 16 + 8 + 2 + 2), Buffer.alloc(48 * 7)]);
    expect(decodeObservations(OBS).entries).toHaveLength(7);
    const RL = Buffer.concat([discriminator('account', 'RaidLedger'), Buffer.alloc(46), Buffer.alloc(56 * 5), Buffer.alloc(81), Buffer.alloc(32)]);
    expect(decodeRaidLedger(RL).inbound).toHaveLength(5);
    const head = 4 + 64 + 8 * 8 + 8 * 4 + 32;
    const WS = Buffer.concat([discriminator('account', 'WarState'), Buffer.alloc(head), Buffer.alloc(80 * 3), Buffer.alloc(4 + 36 + 36 + 64)]);
    expect(decodeWarState(WS).captured).toHaveLength(3);
  });
});

describe('events', () => {
  it('self-CPI event round trip', () => {
    const body = encodeEventBody('war', 'BountyClaimed', { mint: k(), owner: k(), points: 12, paid: 1000n });
    const ev = decodeCpiEvent('war', Buffer.concat([EVENT_IX_TAG, body]));
    expect(ev?.name).toBe('BountyClaimed');
    expect(ev?.data.paid).toBe(1000n);
  });
  it('item log events only inside hookwars_items frames', () => {
    const mint = k();
    const body = encodeEventBody('items', 'RaidMarked', { mint, rival: k(), trader: k(), volume: 5n, points: 2, lootTicket: true }).toString('base64');
    const logs = [
      `Program ${PROGRAM_IDS.swap} invoke [1]`,
      `Program ${PROGRAM_IDS.launch} invoke [2]`,
      `Program ${ITEMS_ID.toBase58()} invoke [3]`,
      `Program data: ${body}`,
      `Program ${ITEMS_ID.toBase58()} success`,
      `Program data: ${body}`,
      `Program ${PROGRAM_IDS.launch} success`,
      `Program ${PROGRAM_IDS.swap} success`,
    ];
    const evs = itemLogEvents(logs);
    expect(evs).toHaveLength(1);
    expect(evs[0]!.name).toBe('RaidMarked');
    expect((evs[0]!.data.mint as PublicKey).equals(mint)).toBe(true);
    expect(logsTruncated([...logs, 'Log truncated'])).toBe(true);
  });
});

describe('instructions', () => {
  it('anchor discriminator then args', () => {
    const ix = hookwarsIx('war', 'claim_quest', { questId: 1, period: 9 }, []);
    expect(ix.programId.equals(WAR_ID)).toBe(true);
    expect(ix.data.subarray(0, 8)).toEqual(discriminator('global', 'claim_quest'));
    expect([...ix.data.subarray(8)]).toEqual([1, 9, 0, 0, 0]);
    expect(() => hookwarsIx('war', 'nope', {}, [])).toThrow();
  });
});
