/**
 * Generated-IDL coder for the units programs (Anchor 1.x IDL format: `address`, `instructions`,
 * `accounts`, `events`, `types`, each with its `discriminator`). Replaces the spec-layout schemas of
 * `codec.ts` for every program whose IDL is in `../../idl/` (INTEGRATION.md section 2).
 *
 * Field names are converted from the Rust snake_case to camelCase on decode, and accepted in either
 * form on encode. Unit-only enums decode to their variant name; enums with data decode to
 * `{ name, ...fields }`.
 */
import { PublicKey, TransactionInstruction, type AccountMeta } from '@solana/web3.js';

export type IdlType =
  | string
  | { option: IdlType }
  | { coption: IdlType }
  | { vec: IdlType }
  | { array: [IdlType, number | { generic: string }] }
  | { defined: { name: string; generics?: unknown[] } | string };

export interface IdlField { name: string; type: IdlType }
export type IdlFields = IdlField[] | IdlType[];
export interface IdlTypeDef {
  name: string;
  type:
    | { kind: 'struct'; fields?: IdlFields }
    | { kind: 'enum'; variants: { name: string; fields?: IdlFields }[] }
    | { kind: 'type'; alias: IdlType };
}
export interface IdlAccountItem {
  name: string;
  writable?: boolean;
  signer?: boolean;
  optional?: boolean;
  address?: string;
  accounts?: IdlAccountItem[];
}
export interface IdlInstruction { name: string; discriminator: number[]; accounts: IdlAccountItem[]; args: IdlField[] }
export interface Idl {
  address: string;
  metadata: { name: string; version?: string; spec?: string };
  instructions: IdlInstruction[];
  accounts?: { name: string; discriminator: number[] }[];
  events?: { name: string; discriminator: number[] }[];
  errors?: { code: number; name: string; msg?: string }[];
  types?: IdlTypeDef[];
}

export const camel = (s: string): string => s.replace(/_([a-z0-9])/g, (_, c: string) => c.toUpperCase());

/** A coder bound to one IDL. */
export class IdlCoder {
  readonly idl: Idl;
  readonly programId: PublicKey;
  private readonly types = new Map<string, IdlTypeDef>();
  constructor(idl: Idl) {
    this.idl = idl;
    this.programId = new PublicKey(idl.address);
    for (const t of idl.types ?? []) this.types.set(t.name, t);
  }

  private def(name: string): IdlTypeDef {
    const t = this.types.get(name);
    if (!t) throw new Error(`${this.idl.metadata.name}: unknown type ${name}`);
    return t;
  }
  private definedName(d: { name: string } | string): string { return typeof d === 'string' ? d : d.name; }

  // ------------------------------------------------------------------ decode --
  private readFields(r: Cursor, fields: IdlFields | undefined): unknown {
    if (!fields || fields.length === 0) return {};
    if (typeof fields[0] === 'object' && fields[0] !== null && 'name' in (fields[0] as object) && 'type' in (fields[0] as object)) {
      const o: Record<string, unknown> = {};
      for (const f of fields as IdlField[]) o[camel(f.name)] = this.read(r, f.type);
      return o;
    }
    return (fields as IdlType[]).map((t) => this.read(r, t));
  }

  read(r: Cursor, ty: IdlType): unknown {
    if (typeof ty === 'string') return r.prim(ty);
    if ('option' in ty) return r.u8() === 0 ? null : this.read(r, ty.option);
    if ('coption' in ty) { const tag = r.u32(); const v = this.read(r, ty.coption); return tag === 0 ? null : v; }
    if ('vec' in ty) { const n = r.u32(); const out: unknown[] = []; for (let i = 0; i < n; i++) out.push(this.read(r, ty.vec)); return out; }
    if ('array' in ty) {
      const [inner, len] = ty.array;
      if (typeof len !== 'number') throw new Error('generic array length');
      if (inner === 'u8') return r.bytes(len);
      const out: unknown[] = []; for (let i = 0; i < len; i++) out.push(this.read(r, inner)); return out;
    }
    const def = this.def(this.definedName(ty.defined));
    const t = def.type;
    if (t.kind === 'struct') return this.readFields(r, t.fields);
    if (t.kind === 'type') return this.read(r, t.alias);
    const idx = r.u8();
    const v = t.variants[idx];
    if (!v) throw new RangeError(`${def.name}: bad variant ${idx}`);
    if (t.variants.every((x) => !x.fields || x.fields.length === 0)) return v.name;
    const body = this.readFields(r, v.fields);
    return Array.isArray(body) ? { name: v.name, fields: body } : { name: v.name, ...(body as object) };
  }

  // ------------------------------------------------------------------ encode --
  private writeFields(w: Sink, fields: IdlFields | undefined, v: unknown): void {
    if (!fields || fields.length === 0) return;
    if (typeof fields[0] === 'object' && fields[0] !== null && 'name' in (fields[0] as object) && 'type' in (fields[0] as object)) {
      const o = (v ?? {}) as Record<string, unknown>;
      for (const f of fields as IdlField[]) {
        const val = camel(f.name) in o ? o[camel(f.name)] : o[f.name];
        if (val === undefined) throw new Error(`${this.idl.metadata.name}: missing field ${camel(f.name)}`);
        this.write(w, f.type, val);
      }
      return;
    }
    const a = (Array.isArray(v) ? v : (v as { fields: unknown[] }).fields) as unknown[];
    (fields as IdlType[]).forEach((t, i) => this.write(w, t, a[i]));
  }

  write(w: Sink, ty: IdlType, v: unknown): void {
    if (typeof ty === 'string') { w.prim(ty, v); return; }
    if ('option' in ty) { if (v === null || v === undefined) w.u8(0); else { w.u8(1); this.write(w, ty.option, v); } return; }
    if ('coption' in ty) { if (v === null || v === undefined) { w.u32(0); this.zero(w, ty.coption); } else { w.u32(1); this.write(w, ty.coption, v); } return; }
    if ('vec' in ty) { const a = [...(v as Iterable<unknown>)]; w.u32(a.length); for (const e of a) this.write(w, ty.vec, e); return; }
    if ('array' in ty) {
      const [inner, len] = ty.array;
      if (typeof len !== 'number') throw new Error('generic array length');
      if (inner === 'u8') { const b = Buffer.from(v as Uint8Array); if (b.length !== len) throw new RangeError(`bytes ${b.length} != ${len}`); w.push(b); return; }
      const a = [...(v as Iterable<unknown>)];
      if (a.length !== len) throw new RangeError(`array ${a.length} != ${len}`);
      for (const e of a) this.write(w, inner, e);
      return;
    }
    const def = this.def(this.definedName(ty.defined));
    const t = def.type;
    if (t.kind === 'struct') { this.writeFields(w, t.fields, v); return; }
    if (t.kind === 'type') { this.write(w, t.alias, v); return; }
    const name = typeof v === 'string' ? v : (v as { name: string }).name;
    const idx = t.variants.findIndex((x) => x.name === name);
    if (idx < 0) throw new RangeError(`${def.name}: bad variant ${String(name)}`);
    w.u8(idx);
    this.writeFields(w, t.variants[idx]!.fields, v);
  }
  private zero(w: Sink, ty: IdlType): void {
    const c = new Sink(); this.write(c, ty, defaultOf(ty)); w.push(Buffer.alloc(c.bytes().length));
  }

  // ------------------------------------------------------------------ accounts and events --
  accountDisc(name: string): Buffer {
    const a = this.idl.accounts?.find((x) => x.name === name);
    if (!a) throw new Error(`${this.idl.metadata.name}: no account ${name}`);
    return Buffer.from(a.discriminator);
  }
  /** Decodes an account by its type name; checks the discriminator. */
  decodeAccount<T = Record<string, unknown>>(name: string, data: Buffer): T {
    const disc = this.accountDisc(name);
    if (data.length < 8 || !data.subarray(0, 8).equals(disc)) throw new Error(`not a ${name} account`);
    const r = new Cursor(data, 8);
    return this.read(r, { defined: { name } }) as T;
  }
  encodeAccount(name: string, v: unknown): Buffer {
    const s = new Sink(); this.write(s, { defined: { name } }, v);
    return Buffer.concat([this.accountDisc(name), s.bytes()]);
  }
  /** The account name whose discriminator starts `data`, or null. */
  accountNameOf(data: Buffer): string | null {
    for (const a of this.idl.accounts ?? []) if (data.length >= 8 && data.subarray(0, 8).equals(Buffer.from(a.discriminator))) return a.name;
    return null;
  }
  eventByDisc(body: Buffer): string | null {
    for (const e of this.idl.events ?? []) if (body.length >= 8 && body.subarray(0, 8).equals(Buffer.from(e.discriminator))) return e.name;
    return null;
  }
  /** Decodes an event body (discriminator first), or null when it is not one of this program's. */
  decodeEvent(body: Buffer): { name: string; data: Record<string, unknown> } | null {
    const name = this.eventByDisc(body);
    if (!name) return null;
    return { name, data: this.read(new Cursor(body, 8), { defined: { name } }) as Record<string, unknown> };
  }
  encodeEvent(name: string, data: Record<string, unknown>): Buffer {
    const e = this.idl.events?.find((x) => x.name === name);
    if (!e) throw new Error(`${this.idl.metadata.name}: no event ${name}`);
    const s = new Sink(); this.write(s, { defined: { name } }, data);
    return Buffer.concat([Buffer.from(e.discriminator), s.bytes()]);
  }

  // ------------------------------------------------------------------ instructions --
  instruction(name: string): IdlInstruction {
    const ix = this.idl.instructions.find((x) => x.name === name);
    if (!ix) throw new Error(`${this.idl.metadata.name}: no instruction ${name}`);
    return ix;
  }
  /** The flattened account list of an instruction, in IDL order. */
  accountsOf(name: string): IdlAccountItem[] {
    const out: IdlAccountItem[] = [];
    const walk = (items: IdlAccountItem[]) => { for (const a of items) (a.accounts ? walk(a.accounts) : out.push(a)); };
    walk(this.instruction(name).accounts);
    return out;
  }
  /**
   * Builds `name(args)`. `accounts` is keyed by camelCase (or snake_case) account name; an account
   * with a fixed `address` in the IDL may be omitted; an optional account that is omitted becomes the
   * program id (Anchor's `None`). `signers` overrides the IDL's signer flag where a PDA signs by CPI.
   */
  ix(name: string, accounts: Record<string, PublicKey | null | undefined>, args: Record<string, unknown> = {}, remaining: AccountMeta[] = []): TransactionInstruction {
    const def = this.instruction(name);
    const keys = this.metas(name, accounts, args);
    const s = new Sink();
    for (const f of def.args) {
      const v = camel(f.name) in args ? args[camel(f.name)] : args[f.name];
      if (v === undefined) throw new Error(`${this.idl.metadata.name}::${name}: missing arg ${camel(f.name)}`);
      this.write(s, f.type, v);
    }
    return new TransactionInstruction({ programId: this.programId, keys: [...keys, ...remaining], data: Buffer.concat([Buffer.from(def.discriminator), s.bytes()]) });
  }
  /**
   * The account metas of an instruction in IDL order (see `ix`). Accounts the caller leaves out are
   * filled from the IDL: a fixed `address`, a PDA whose seeds are constants, other accounts of the
   * instruction or instruction arguments, the event authority (`["__event_authority"]`) and the
   * program itself; an optional account left out is Anchor's `None` (the program id).
   */
  metas(name: string, accounts: Record<string, PublicKey | null | undefined>, args: Record<string, unknown> = {}): AccountMeta[] {
    const items = this.accountsOf(name);
    const def = this.instruction(name);
    const known = new Map<string, PublicKey>();
    for (const [k, v] of Object.entries(accounts)) if (v) known.set(camel(k), v);
    const argBytes = (path: string): Buffer | null => {
      const f = def.args.find((a) => a.name === path || camel(a.name) === camel(path));
      if (!f) return null;
      const v = camel(f.name) in args ? args[camel(f.name)] : args[f.name];
      if (v === undefined) return null;
      const s = new Sink(); this.write(s, f.type, v);
      const b = s.bytes();
      return f.type === 'string' || f.type === 'bytes' ? b.subarray(4) : b;
    };
    type Seed = { kind: 'const'; value: number[] } | { kind: 'account'; path: string } | { kind: 'arg'; path: string };
    const resolveSeed = (sd: Seed): Buffer | null => {
      if (sd.kind === 'const') return Buffer.from(sd.value);
      if (sd.kind === 'arg') return argBytes(sd.path);
      if (sd.path.includes('.')) return null; // a field of another account's data
      return known.get(camel(sd.path))?.toBuffer() ?? null;
    };
    for (let pass = 0; pass < 4; pass++) {
      for (const a of items) {
        const key = camel(a.name);
        if (known.has(key)) continue;
        if (a.address) { known.set(key, new PublicKey(a.address)); continue; }
        if (key === 'eventAuthority') { known.set(key, PublicKey.findProgramAddressSync([Buffer.from('__event_authority')], this.programId)[0]); continue; }
        if (key === 'program') { known.set(key, this.programId); continue; }
        if (a.optional) continue; // an optional account is only passed when the caller gives it
        const pda = (a as IdlAccountItem & { pda?: { seeds: Seed[]; program?: Seed } }).pda;
        if (!pda) continue;
        const seeds = pda.seeds.map(resolveSeed);
        if (seeds.some((x) => x === null)) continue;
        const prog = pda.program ? resolveSeed(pda.program) : this.programId.toBuffer();
        if (!prog) continue;
        known.set(key, PublicKey.findProgramAddressSync(seeds as Buffer[], new PublicKey(prog))[0]);
      }
    }
    const keys: AccountMeta[] = [];
    for (const a of items) {
      const key = camel(a.name);
      const k = known.get(key);
      if (k) { keys.push({ pubkey: k, isSigner: !!a.signer, isWritable: !!a.writable }); continue; }
      if (a.optional) { keys.push({ pubkey: this.programId, isSigner: false, isWritable: false }); continue; }
      throw new Error(`${this.idl.metadata.name}::${name}: missing account ${key}`);
    }
    return keys;
  }
  errorName(code: number): string | null { return this.idl.errors?.find((e) => e.code === code)?.name ?? null; }
}

function defaultOf(ty: IdlType): unknown {
  if (typeof ty === 'string') return ty === 'pubkey' ? PublicKey.default : ty === 'bool' ? false : ty === 'string' ? '' : ty === 'bytes' ? Buffer.alloc(0) : /^[ui](64|128)$/.test(ty) ? 0n : 0;
  return 0;
}

export class Cursor {
  off: number;
  readonly buf: Buffer;
  constructor(buf: Buffer, off = 0) { this.buf = buf; this.off = off; }
  private need(n: number): void { if (this.off + n > this.buf.length) throw new RangeError(`read past end at ${this.off} (+${n} of ${this.buf.length})`); }
  u8(): number { this.need(1); return this.buf.readUInt8(this.off++); }
  u32(): number { this.need(4); const v = this.buf.readUInt32LE(this.off); this.off += 4; return v; }
  bytes(n: number): Buffer { this.need(n); const v = Buffer.from(this.buf.subarray(this.off, this.off + n)); this.off += n; return v; }
  prim(t: string): unknown {
    const b = this.buf;
    switch (t) {
      case 'u8': return this.u8();
      case 'i8': { this.need(1); return b.readInt8(this.off++); }
      case 'bool': return this.u8() !== 0;
      case 'u16': { this.need(2); const v = b.readUInt16LE(this.off); this.off += 2; return v; }
      case 'i16': { this.need(2); const v = b.readInt16LE(this.off); this.off += 2; return v; }
      case 'u32': return this.u32();
      case 'i32': { this.need(4); const v = b.readInt32LE(this.off); this.off += 4; return v; }
      case 'u64': { this.need(8); const v = b.readBigUInt64LE(this.off); this.off += 8; return v; }
      case 'i64': { this.need(8); const v = b.readBigInt64LE(this.off); this.off += 8; return v; }
      case 'u128': { this.need(16); const lo = b.readBigUInt64LE(this.off); const hi = b.readBigUInt64LE(this.off + 8); this.off += 16; return (hi << 64n) | lo; }
      case 'i128': { this.need(16); const lo = b.readBigUInt64LE(this.off); const hi = b.readBigInt64LE(this.off + 8); this.off += 16; return (hi << 64n) | lo; }
      case 'pubkey': return new PublicKey(this.bytes(32));
      case 'string': { const n = this.u32(); this.need(n); const v = b.toString('utf8', this.off, this.off + n); this.off += n; return v; }
      case 'bytes': { const n = this.u32(); return this.bytes(n); }
      default: throw new Error(`unsupported idl type ${t}`);
    }
  }
}

export class Sink {
  private parts: Buffer[] = [];
  push(b: Buffer): void { this.parts.push(b); }
  u8(v: number): void { const x = Buffer.alloc(1); x.writeUInt8(v); this.parts.push(x); }
  u32(v: number): void { const x = Buffer.alloc(4); x.writeUInt32LE(v); this.parts.push(x); }
  prim(t: string, v: unknown): void {
    const big = (n: number) => { const x = Buffer.alloc(n); return x; };
    switch (t) {
      case 'u8': this.u8(Number(v)); return;
      case 'i8': { const x = big(1); x.writeInt8(Number(v)); this.parts.push(x); return; }
      case 'bool': this.u8(v ? 1 : 0); return;
      case 'u16': { const x = big(2); x.writeUInt16LE(Number(v)); this.parts.push(x); return; }
      case 'i16': { const x = big(2); x.writeInt16LE(Number(v)); this.parts.push(x); return; }
      case 'u32': this.u32(Number(v)); return;
      case 'i32': { const x = big(4); x.writeInt32LE(Number(v)); this.parts.push(x); return; }
      case 'u64': { const x = big(8); x.writeBigUInt64LE(BigInt(v as bigint)); this.parts.push(x); return; }
      case 'i64': { const x = big(8); x.writeBigInt64LE(BigInt(v as bigint)); this.parts.push(x); return; }
      case 'u128': case 'i128': {
        const n = BigInt(v as bigint); const m = t === 'i128' && n < 0n ? (1n << 128n) + n : n;
        const x = big(16); x.writeBigUInt64LE(m & ((1n << 64n) - 1n)); x.writeBigUInt64LE(m >> 64n, 8); this.parts.push(x); return;
      }
      case 'pubkey': this.parts.push((v instanceof PublicKey ? v : new PublicKey(v as string)).toBuffer()); return;
      case 'string': { const s = Buffer.from(String(v), 'utf8'); this.u32(s.length); this.parts.push(s); return; }
      case 'bytes': { const s = Buffer.from(v as Uint8Array); this.u32(s.length); this.parts.push(s); return; }
      default: throw new Error(`unsupported idl type ${t}`);
    }
  }
  bytes(): Buffer { return Buffer.concat(this.parts); }
}
