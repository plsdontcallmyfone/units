// Changed by Hookwars: enums with data (`tagged`), for the agents, market and social IDLs.
/**
 * A small Borsh codec driven by field schemas, used for the Hookwars accounts, events and
 * instruction arguments whose IDLs are not generated yet (the programs are being built on other
 * branches). Every schema names the spec section it follows; INTEGRATION.md lists each one to swap
 * for the generated IDL coder once the programs land. Layouts are Anchor's: an 8-byte discriminator
 * (`sha256("account:<Name>")`, `sha256("event:<Name>")`, `sha256("global:<name>")`) then Borsh.
 */
import { createHash } from 'node:crypto';
import { PublicKey } from '@solana/web3.js';

export type Ty =
  | 'u8' | 'u16' | 'u32' | 'u64' | 'u128' | 'i8' | 'i16' | 'i32' | 'i64' | 'i128' | 'bool' | 'pubkey' | 'string'
  | { option: Ty }
  | { vec: Ty }
  | { array: [Ty, number] }
  | { bytes: number }
  | { struct: Field[] }
  | { enum: string[] }
  /** An enum with data (Borsh: u8 variant index, then the variant's fields); values are `{ name, ...fields }` as `IdlCoder` decodes them. */
  | { tagged: [name: string, fields: Field[]][] };

export type Field = [name: string, ty: Ty];

export function discriminator(namespace: 'account' | 'event' | 'global', name: string): Buffer {
  return createHash('sha256').update(`${namespace}:${name}`).digest().subarray(0, 8);
}

export class Reader {
  off = 0;
  readonly buf: Buffer;
  constructor(buf: Buffer) { this.buf = buf; }
  need(n: number): void {
    if (this.off + n > this.buf.length) throw new RangeError(`read past end at ${this.off} (+${n} of ${this.buf.length})`);
  }
  read(ty: Ty): unknown {
    const b = this.buf;
    if (typeof ty === 'string') {
      switch (ty) {
        case 'u8': this.need(1); return b.readUInt8(this.off++);
        case 'i8': this.need(1); return b.readInt8(this.off++);
        case 'i16': { this.need(2); const v = b.readInt16LE(this.off); this.off += 2; return v; }
        case 'i32': { this.need(4); const v = b.readInt32LE(this.off); this.off += 4; return v; }
        case 'bool': this.need(1); return b.readUInt8(this.off++) !== 0;
        case 'u16': { this.need(2); const v = b.readUInt16LE(this.off); this.off += 2; return v; }
        case 'u32': { this.need(4); const v = b.readUInt32LE(this.off); this.off += 4; return v; }
        case 'u64': { this.need(8); const v = b.readBigUInt64LE(this.off); this.off += 8; return v; }
        case 'i64': { this.need(8); const v = b.readBigInt64LE(this.off); this.off += 8; return v; }
        case 'u128': { this.need(16); const lo = b.readBigUInt64LE(this.off); const hi = b.readBigUInt64LE(this.off + 8); this.off += 16; return (hi << 64n) | lo; }
        case 'i128': { this.need(16); const lo = b.readBigUInt64LE(this.off); const hi = b.readBigInt64LE(this.off + 8); this.off += 16; return (hi << 64n) | lo; }
        case 'pubkey': { this.need(32); const v = new PublicKey(b.subarray(this.off, this.off + 32)); this.off += 32; return v; }
        case 'string': { const n = this.read('u32') as number; this.need(n); const v = b.toString('utf8', this.off, this.off + n); this.off += n; return v; }
      }
    }
    if ('option' in ty) return (this.read('u8') as number) === 0 ? null : this.read(ty.option);
    if ('vec' in ty) { const n = this.read('u32') as number; const out: unknown[] = []; for (let i = 0; i < n; i++) out.push(this.read(ty.vec)); return out; }
    if ('array' in ty) { const out: unknown[] = []; for (let i = 0; i < ty.array[1]; i++) out.push(this.read(ty.array[0])); return out; }
    if ('bytes' in ty) { this.need(ty.bytes); const v = Buffer.from(b.subarray(this.off, this.off + ty.bytes)); this.off += ty.bytes; return v; }
    if ('enum' in ty) { const i = this.read('u8') as number; const name = ty.enum[i]; if (name === undefined) throw new RangeError(`bad enum index ${i}`); return name; }
    if ('tagged' in ty) {
      const i = this.read('u8') as number; const v = ty.tagged[i]; if (v === undefined) throw new RangeError(`bad enum index ${i}`);
      const o: Record<string, unknown> = { name: v[0] };
      for (const [n, t] of v[1]) o[n] = this.read(t);
      return o;
    }
    const o: Record<string, unknown> = {};
    for (const [n, t] of ty.struct) o[n] = this.read(t);
    return o;
  }
}

export class Writer {
  parts: Buffer[] = [];
  write(ty: Ty, v: unknown): void {
    const p = this.parts;
    if (typeof ty === 'string') {
      switch (ty) {
        case 'u8': { const x = Buffer.alloc(1); x.writeUInt8(Number(v)); p.push(x); return; }
        case 'i8': { const x = Buffer.alloc(1); x.writeInt8(Number(v)); p.push(x); return; }
        case 'i16': { const x = Buffer.alloc(2); x.writeInt16LE(Number(v)); p.push(x); return; }
        case 'i32': { const x = Buffer.alloc(4); x.writeInt32LE(Number(v)); p.push(x); return; }
        case 'bool': { const x = Buffer.alloc(1); x.writeUInt8(v ? 1 : 0); p.push(x); return; }
        case 'u16': { const x = Buffer.alloc(2); x.writeUInt16LE(Number(v)); p.push(x); return; }
        case 'u32': { const x = Buffer.alloc(4); x.writeUInt32LE(Number(v)); p.push(x); return; }
        case 'u64': { const x = Buffer.alloc(8); x.writeBigUInt64LE(BigInt(v as bigint)); p.push(x); return; }
        case 'i64': { const x = Buffer.alloc(8); x.writeBigInt64LE(BigInt(v as bigint)); p.push(x); return; }
        case 'u128': case 'i128': {
          const n = BigInt(v as bigint);
          const m = ty === 'i128' && n < 0n ? (1n << 128n) + n : n;
          const x = Buffer.alloc(16); x.writeBigUInt64LE(m & ((1n << 64n) - 1n)); x.writeBigUInt64LE(m >> 64n, 8); p.push(x); return;
        }
        case 'pubkey': p.push((v instanceof PublicKey ? v : new PublicKey(v as string)).toBuffer()); return;
        case 'string': { const s = Buffer.from(String(v), 'utf8'); this.write('u32', s.length); p.push(s); return; }
      }
    }
    if ('option' in ty) { if (v === null || v === undefined) this.write('u8', 0); else { this.write('u8', 1); this.write(ty.option, v); } return; }
    if ('vec' in ty) { const a = v as unknown[]; this.write('u32', a.length); for (const e of a) this.write(ty.vec, e); return; }
    if ('array' in ty) { const a = v as unknown[]; if (a.length !== ty.array[1]) throw new RangeError(`array length ${a.length} != ${ty.array[1]}`); for (const e of a) this.write(ty.array[0], e); return; }
    if ('bytes' in ty) { const x = Buffer.from(v as Uint8Array); if (x.length !== ty.bytes) throw new RangeError(`bytes ${x.length} != ${ty.bytes}`); p.push(x); return; }
    if ('enum' in ty) { const i = ty.enum.indexOf(String(v)); if (i < 0) throw new RangeError(`bad enum ${String(v)}`); this.write('u8', i); return; }
    if ('tagged' in ty) {
      const o = v as Record<string, unknown>;
      const i = ty.tagged.findIndex(([n]) => n === o.name);
      if (i < 0) throw new RangeError(`bad enum ${String(o.name)}`);
      this.write('u8', i);
      for (const [n, t] of ty.tagged[i]![1]) this.write(t, o[n]);
      return;
    }
    const o = v as Record<string, unknown>;
    for (const [n, t] of ty.struct) this.write(t, o[n]);
  }
  bytes(): Buffer { return Buffer.concat(this.parts); }
}

export function encode(ty: Ty, v: unknown): Buffer { const w = new Writer(); w.write(ty, v); return w.bytes(); }
export function decode<T = Record<string, unknown>>(ty: Ty, buf: Buffer, offset = 0): T { const r = new Reader(buf); r.off = offset; return r.read(ty) as T; }

/** Byte size of a fixed-size type (no strings, vecs or options). */
export function fixedSize(ty: Ty): number {
  if (typeof ty === 'string') {
    const sizes: Record<string, number> = { u8: 1, i8: 1, bool: 1, u16: 2, i16: 2, u32: 4, i32: 4, u64: 8, i64: 8, u128: 16, i128: 16, pubkey: 32 };
    const s = sizes[ty]; if (s === undefined) throw new Error(`${ty} has no fixed size`); return s;
  }
  if ('array' in ty) return fixedSize(ty.array[0]) * ty.array[1];
  if ('bytes' in ty) return ty.bytes;
  if ('enum' in ty) return 1;
  if ('struct' in ty) return ty.struct.reduce((n, [, t]) => n + fixedSize(t), 0);
  throw new Error('variable size');
}

/** An Anchor account: discriminator check, then the schema. */
export interface AccountCodec<T> { name: string; disc: Buffer; ty: Ty; decode(data: Buffer): T; encode(v: T): Buffer }
export function accountCodec<T>(name: string, fields: Field[]): AccountCodec<T> {
  const disc = discriminator('account', name);
  const ty: Ty = { struct: fields };
  return {
    name, disc, ty,
    decode(data: Buffer): T {
      if (data.length < 8 || !data.subarray(0, 8).equals(disc)) throw new Error(`not a ${name} account`);
      return decode<T>(ty, data, 8);
    },
    encode(v: T): Buffer { return Buffer.concat([disc, encode(ty, v)]); },
  };
}

/** An Anchor event: `sha256("event:<Name>")[..8]` then the fields. */
export interface EventCodec<T> { name: string; disc: Buffer; ty: Ty; decode(data: Buffer): T; encode(v: T): Buffer }
export function eventCodec<T>(name: string, fields: Field[]): EventCodec<T> {
  const disc = discriminator('event', name);
  const ty: Ty = { struct: fields };
  return {
    name, disc, ty,
    decode: (data: Buffer) => decode<T>(ty, data, 8),
    encode: (v: T) => Buffer.concat([disc, encode(ty, v)]),
  };
}
