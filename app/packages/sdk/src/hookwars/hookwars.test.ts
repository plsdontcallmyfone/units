import { describe, expect, it } from 'vitest';
import { Keypair, PublicKey } from '@solana/web3.js';
import { FIXED_ADDRESSES, HOOK_SIGNERS, PROGRAM_IDS } from '@hookwars/shared';
import { encodeHookAccountList } from '../hooks.ts';
import {
  ARMORY_ID, ITEMS_ID, LAUNCH_ID, MAX_SLOTS, PARAM_FIELDS, TOKEN_ID, WAR_ID, activeSlots, claimQuest, coderOf, decodeCpiEvent, decodePoolObservations, POOL_LEN, OBS_HEADER_LEN,
  decodeRaidLedger, decodeSlotMint, decodeWarState, discriminator, encode, encodeEventBody, EVENT_IX_TAG, EVENT_SPECS, forge, holdingCodec,
  idlAccountCodec, isCalled, itemCodec, itemLogEvents, lockedAt, logsTruncated, resolveSlices, settleEquip, sliceAccounts, swapRoute, SWAP_ID, slotRegistryAddress, structFields,
  tokenHookSigner, tyOf, vote, IDLS, type SlotData, type SlotMintData,
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
  kind: 0, equipRule: 0, bounds: { maxCutBps: 0, mayRefuse: false, mayWriteData: false, mayAnswerTouch: false, mayBurn: false }, dataOffset: 0, dataLen: 0,
  item: PublicKey.default, program: PublicKey.default, flags: 0, poolFlags: 0, equipVault: PublicKey.default, signerBump: 0, launchSignerBump: 0, dataEpoch: 0, extraCount: 0,
});

function mintBytes(m: SlotMintData, pad = 0): Buffer {
  return Buffer.concat([discriminator('account', 'Mint'), encode({ struct: structFields('token', 'Mint') }, m), Buffer.alloc(pad)]);
}

describe('token Mint with slots (IDL)', () => {
  const kitSlot: SlotData = { ...emptySlot(), kind: 5, equipRule: 0, dataLen: 32, program: k(), flags: 1 | 64 | 128, extraCount: 0 };
  const raidSlot: SlotData = { ...emptySlot(), kind: 4, equipRule: 1, dataOffset: 32, dataLen: 12, item: k(), program: ITEMS_ID, flags: 1, extraCount: 2, dataEpoch: 1, bounds: { maxCutBps: 300, mayRefuse: false, mayWriteData: true, mayAnswerTouch: true, mayBurn: false } };
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
  it('a slot is 114 bytes (01 M1 notes, plus SlotBounds.may_burn from M3b)', () => {
    expect(encode(tyOf(IDLS.token!, { defined: { name: 'Slot' } }), emptySlot()).length).toBe(114);
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

describe('IDL accounts', () => {
  it('PARAM_FIELDS and MAX_SLOTS come from the IDLs', () => {
    expect(PARAM_FIELDS).toBe(11);
    expect(MAX_SLOTS).toBe(4);
  });
  it('Item round-trips with PARAM_FIELDS params', () => {
    const c = itemCodec();
    const item = {
      version: 1, bump: 254, itemMint: k(), templateId: 1, params: Array.from({ length: PARAM_FIELDS }, (_, i) => i),
      manifest: { kind: 4, tokenFlags: 257, poolFlags: 7, maxCutBuyBps: 100, maxCutSellBps: 0, maxCutTransferBps: 0, maxDiscountBps: 500, mayRefuse: false, mayBurn: false, dataBytes: 11, readsOtherPools: 1 },
      author: k(), royaltyBps: 250, level: 1, source: 0, equippedCount: 0, royaltyOwnerBump: 253, createdAt: 5n, reserved: Buffer.alloc(32),
    };
    expect(c.decode(c.encode(item))).toMatchObject({ templateId: 1, royaltyBps: 250, level: 1 });
  });
  it('every account of the IDL programs has a codec that round-trips zeros', () => {
    for (const [program, idl] of Object.entries(IDLS)) {
      for (const a of idl.accounts ?? []) {
        const codec = idlAccountCodec<Record<string, unknown>>(program, a.name);
        const zero = coderOf(program).decodeAccount(a.name, Buffer.concat([Buffer.from(a.discriminator), Buffer.alloc(20_000)]));
        expect(codec.decode(codec.encode(zero as Record<string, unknown>))).toBeTruthy();
      }
    }
  });
  it('WarState decodes through the IDL with MAX_CAPTURED entries', () => {
    const zero = coderOf('war').decodeAccount<Record<string, unknown>>('WarState', Buffer.concat([discriminator('account', 'WarState'), Buffer.alloc(4_000)]));
    const data = idlAccountCodec<Record<string, unknown>>('war', 'WarState').encode(zero);
    expect(decodeWarState(data).captured.length).toBeGreaterThan(0);
  });
  it('spec-layout rings and tables are sized from the data', () => {
    // Changed by Hookwars: the ring is the tail of the pool account (M3a), read from POOL_LEN.
    const head = Buffer.alloc(OBS_HEADER_LEN);
    discriminator('account', 'Observations').copy(head, 0);
    head[8] = 1; head.writeUInt16LE(7, 86);
    const POOL = Buffer.concat([Buffer.alloc(POOL_LEN), head, Buffer.alloc(48 * 7)]);
    expect(decodePoolObservations(POOL).entries).toHaveLength(7);
    expect(() => decodePoolObservations(Buffer.alloc(POOL_LEN + OBS_HEADER_LEN))).toThrow();
    const RL = Buffer.concat([discriminator('account', 'RaidLedger'), Buffer.alloc(46), Buffer.alloc(56 * 5), Buffer.alloc(81), Buffer.alloc(32)]);
    expect(decodeRaidLedger(RL).inbound).toHaveLength(5);
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
  it('IDL builders use the IDL discriminator, arguments and account order', () => {
    const owner = k(), mint = k();
    const ix = claimQuest(owner, mint, 3, 1, 9, 1);
    const def = coderOf('war').instruction('claim_quest');
    expect(ix.programId.equals(WAR_ID)).toBe(true);
    expect(ix.data.subarray(0, 8)).toEqual(Buffer.from(def.discriminator));
    expect([...ix.data.subarray(8)]).toEqual([1, 9, 0, 0, 0, 1]);
    const idlCount = coderOf('war').accountsOf('claim_quest').length;
    expect(ix.keys.length).toBe(idlCount + 2); // the token program and its event authority (client.rs)
    expect(ix.keys[0]!.pubkey.equals(owner) && ix.keys[0]!.isSigner).toBe(true);
    // forge_counter is optional and None for the Raid quest: the program id stands in.
    const fc = coderOf('war').accountsOf('claim_quest').findIndex((a) => a.name === 'forge_counter');
    expect(ix.keys[fc]!.pubkey.equals(WAR_ID)).toBe(true);
  });
  it('PDAs come from the IDL seeds', () => {
    const voter = k(), mint = k();
    const ix = vote(voter, mint, 2, 0n, true, 10n);
    const names = coderOf('armory').accountsOf('vote').map((a) => a.name);
    const slotAuth = PublicKey.findProgramAddressSync([Buffer.from('slots'), mint.toBuffer()], ARMORY_ID)[0];
    expect(ix.keys[names.indexOf('slot_authority')]!.pubkey.equals(slotAuth)).toBe(true);
    const ea = PublicKey.findProgramAddressSync([Buffer.from('__event_authority')], ARMORY_ID)[0];
    expect(ix.keys[names.indexOf('event_authority')]!.pubkey.equals(ea)).toBe(true);
  });
  it('forge derives the new item from items_minted', () => {
    const forger = k();
    const ix = forge(forger, { item: k(), itemMint: k() }, { item: k(), itemMint: k() }, 1, 7n);
    const names = coderOf('armory').accountsOf('forge').map((a) => a.name);
    const itemMint = PublicKey.findProgramAddressSync([Buffer.from('item-mint'), Buffer.from([7, 0, 0, 0, 0, 0, 0, 0])], ARMORY_ID)[0];
    expect(ix.keys[names.indexOf('item_mint')]!.pubkey.equals(itemMint)).toBe(true);
  });
  it('settle_equip and swap_route come from the items and DEX IDLs', () => {
    const dest = k();
    const ix = settleEquip(k(), k(), 1, { key: k(), tokenCuts: true, composite: false }, k(), [[k(), dest]]);
    expect(ix.programId.equals(ITEMS_ID)).toBe(true);
    expect(ix.data.subarray(0, 8)).toEqual(discriminator('global', 'settle_equip'));
    expect(ix.keys.some((m) => m.pubkey.equals(dest))).toBe(true);
  });
  it('swap_route lays out each hop as nine fixed accounts then its slices', () => {
    const hop = { pool: k(), baseMint: k(), quoteMint: k(), traderBase: k(), traderQuote: k(), hookProgram: null, baseMintWritable: true, quoteMintWritable: false, direction: 0 as const, inSlice: [{ pubkey: k(), isSigner: false, isWritable: false }], outSlice: [], poolExtras: [] };
    const ix = swapRoute(k(), 10n, 1n, [hop, { ...hop, pool: k() }]);
    expect(ix.programId.equals(SWAP_ID)).toBe(true);
    expect(ix.data.subarray(0, 8)).toEqual(discriminator('global', 'swap_route'));
    const fixed = coderOf('swap').accountsOf('swap_route').length;
    expect(ix.keys.length).toBe(fixed + 2 * 10);
  });
  it('events of the IDL programs come from the IDLs', () => {
    const war = EVENT_SPECS.war!.map(([n]) => n);
    expect(war).toContain('BountyClaimed');
    expect(EVENT_SPECS.items!.map(([n]) => n)).toEqual(expect.arrayContaining(['EquipInitialized', 'EquipClosed', 'RaidMarked']));
  });
});

describe('transaction events', () => {
  it('decodes self-CPI and item log events in one sequence', async () => {
    const { decodeTransactionEvents } = await import('./tx.ts');
    const { eventAuthority } = await import('../addresses.ts');
    const bs58 = (await import('bs58')).default;
    const war = WAR_ID.toBase58();
    const ea = eventAuthority(WAR_ID).toBase58();
    const body = encodeEventBody('war', 'WarFunded', { mint: k(), amount: 5n, balance: 5n, fundedTotal: 5n });
    const tx = {
      accountKeys: ['payer', war, ea],
      inner: [{ programIdIndex: 1, accounts: [2], data: bs58.encode(Buffer.concat([EVENT_IX_TAG, body])) }, { programIdIndex: 1, accounts: [0], data: bs58.encode(Buffer.concat([EVENT_IX_TAG, body])) }],
      logs: [],
    };
    const out = decodeTransactionEvents(tx);
    expect(out.events.map((e) => [e.ordinal, e.name, e.via])).toEqual([[0, 'WarFunded', 'cpi']]);
    expect(out.events[0]!.data.amount).toBe('5');
  });
});
