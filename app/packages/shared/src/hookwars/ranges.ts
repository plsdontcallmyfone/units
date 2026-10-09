/**
 * Decoding a holding's 64 bytes of hook data by the mint's slot table (docs/spec/06-app.md 2.2,
 * 00 4.4, 01 3.4, 04 2.4). A Locked range (the kit, bytes 0..32) has no epoch byte. An item range's
 * first byte is the token program's epoch byte: when it differs from the slot's current
 * `data_epoch`, the range is stale and reads as empty. The bytes after it are the template's layout.
 */
export interface SlotRangeSpec { slot: number; kind: number; offset: number; len: number; dataEpoch: number; templateId: number | null }

export type DecodedRange =
  | { slot: number; type: 'empty' }
  | { slot: number; type: 'stale' }
  | { slot: number; type: 'kit'; snapshot: bigint; owed: bigint; earlyLocked: bigint }
  | { slot: number; type: 'raid'; seasonId: number; raidPoints: number; tickets: number }
  | { slot: number; type: 'shield'; origin: number; originAt: number }
  | { slot: number; type: 'halfLife'; since: number }
  | { slot: number; type: 'cooldown'; lastBuy: number }
  | { slot: number; type: 'dailySellCap'; day: number; base: bigint }
  | { slot: number; type: 'flashGuard'; buySlotLow: number }
  | { slot: number; type: 'loyalty'; joinedEpoch: number }
  | { slot: number; type: 'streak'; lastDay: number; streak: number; flags: number }
  | { slot: number; type: 'rankBadge'; volumeUnits: number; rank: number }
  | { slot: number; type: 'guildTag'; guild: number }
  | { slot: number; type: 'patience'; since: number }
  | { slot: number; type: 'unknown'; tag: number; bytes: Uint8Array };

/** On-chain value of slot_kind::LOCKED (crates/bordrless-hook). */
export const SLOT_KIND_LOCKED = 5;

const u32 = (b: Uint8Array, o: number): number => ((b[o] ?? 0) | ((b[o + 1] ?? 0) << 8) | ((b[o + 2] ?? 0) << 16)) + (b[o + 3] ?? 0) * 0x1000000;
const u16 = (b: Uint8Array, o: number): number => (b[o] ?? 0) | ((b[o + 1] ?? 0) << 8);
const u64 = (b: Uint8Array, o: number): bigint => {
  let v = 0n;
  for (let i = 7; i >= 0; i--) v = (v << 8n) | BigInt(b[o + i] ?? 0);
  return v;
};

export function decodeRange(data: Uint8Array, spec: SlotRangeSpec, currentSeason: number | null): DecodedRange {
  const bytes = data.subarray(spec.offset, spec.offset + spec.len);
  if (spec.kind === SLOT_KIND_LOCKED) {
    if (bytes.every((x) => x === 0)) return { slot: spec.slot, type: 'empty' };
    // Upstream kit layout, hooks-v2 4.4: snapshot u128 (only the low 64 bits kept for display),
    // owed u64 at 16, early_locked u64 at 24.
    return { slot: spec.slot, type: 'kit', snapshot: u64(bytes, 0), owed: u64(bytes, 16), earlyLocked: u64(bytes, 24) };
  }
  if (spec.len === 0) return { slot: spec.slot, type: 'empty' };
  const epoch = bytes[0] ?? 0;
  const body = bytes.subarray(1);
  if (body.every((x) => x === 0)) return { slot: spec.slot, type: 'empty' };
  if (epoch !== spec.dataEpoch) return { slot: spec.slot, type: 'stale' };
  const tag = body[0] ?? 0;
  switch (tag) {
    case 0x01: {
      const seasonId = u32(body, 1);
      const raidPoints = currentSeason !== null && seasonId !== currentSeason ? 0 : u32(body, 5);
      return { slot: spec.slot, type: 'raid', seasonId, raidPoints, tickets: u16(body, 9) };
    }
    case 0x02:
      return { slot: spec.slot, type: 'shield', origin: body[1] ?? 0, originAt: u32(body, 2) };
    case 0x07:
      return { slot: spec.slot, type: 'halfLife', since: u32(body, 1) };
    // Arsenal layouts (programs/hookwars_items/src/templates/*.rs `TAG` and `read`).
    case 0x11:
      return { slot: spec.slot, type: 'cooldown', lastBuy: u32(body, 1) };
    case 0x12:
      return { slot: spec.slot, type: 'dailySellCap', day: u16(body, 1), base: u64(body, 3) };
    case 0x14:
      return { slot: spec.slot, type: 'flashGuard', buySlotLow: u32(body, 1) };
    case 0x18:
      return { slot: spec.slot, type: 'loyalty', joinedEpoch: u32(body, 1) };
    case 0x1a:
      return { slot: spec.slot, type: 'streak', lastDay: u16(body, 1), streak: u16(body, 3), flags: body[5] ?? 0 };
    case 0x23:
      return { slot: spec.slot, type: 'rankBadge', volumeUnits: u32(body, 1), rank: body[5] ?? 0 };
    case 0x27:
      return { slot: spec.slot, type: 'guildTag', guild: u16(body, 1) };
    case 0x28:
      return { slot: spec.slot, type: 'patience', since: u32(body, 1) };
    default:
      return { slot: spec.slot, type: 'unknown', tag, bytes: body };
  }
}

export function decodeHookData(data: Uint8Array, specs: SlotRangeSpec[], currentSeason: number | null): DecodedRange[] {
  return specs.map((s) => decodeRange(data, s, currentSeason));
}
