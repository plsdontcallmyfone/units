// Changed by Hookwars: registers the launch, swap, agents, market and social IDLs.
/**
 * Turns the generated IDLs into the field schemas `codec.ts` runs (camelCase names), so accounts,
 * events and instruction arguments of the programs on main come from the programs themselves
 * (INTEGRATION.md section 2). Discriminators are taken from the IDL and checked against Anchor's
 * `sha256("<namespace>:<Name>")[..8]`, which `codec.ts` computes.
 */
import { PublicKey, TransactionInstruction, type AccountMeta } from '@solana/web3.js';
import { accountCodec, discriminator, encode, type AccountCodec, type Field, type Ty } from './codec.ts';
import { camel, IdlCoder, type Idl, type IdlFields, type IdlType } from './idl.ts';

import tokenIdl from '../../idl/bordrless_token.json' with { type: 'json' };
import armoryIdl from '../../idl/hookwars_armory.json' with { type: 'json' };
import itemsIdl from '../../idl/hookwars_items.json' with { type: 'json' };
import warIdl from '../../idl/hookwars_war.json' with { type: 'json' };
import kitIdl from '../../idl/bordrless_kit.json' with { type: 'json' };
import companionIdl from '../../idl/bordrless_companion.json' with { type: 'json' };
import launchIdl from '../../idl/bordrless_launch.json' with { type: 'json' };
import swapIdl from '../../idl/bordrless_swap.json' with { type: 'json' };
import agentsIdl from '../../idl/hookwars_agents.json' with { type: 'json' };
import marketIdl from '../../idl/hookwars_market.json' with { type: 'json' };
import socialIdl from '../../idl/hookwars_social.json' with { type: 'json' };

/** The IDLs of the programs whose interfaces are final on main, by the program names the app uses. */
export const IDLS: Record<string, Idl> = {
  token: tokenIdl as unknown as Idl,
  armory: armoryIdl as unknown as Idl,
  items: itemsIdl as unknown as Idl,
  war: warIdl as unknown as Idl,
  kit: kitIdl as unknown as Idl,
  companion: companionIdl as unknown as Idl,
  launch: launchIdl as unknown as Idl,
  swap: swapIdl as unknown as Idl,
  agents: agentsIdl as unknown as Idl,
  market: marketIdl as unknown as Idl,
  social: socialIdl as unknown as Idl,
};

const coders = new Map<string, IdlCoder>();
/** The IDL coder of a program (`token`, `armory`, `items`, `war`, `kit`, `companion`, `launch`, `swap`). */
export function coderOf(program: string): IdlCoder {
  let c = coders.get(program);
  if (!c) {
    const idl = IDLS[program];
    if (!idl) throw new Error(`no IDL for ${program}`);
    c = new IdlCoder(idl);
    coders.set(program, c);
  }
  return c;
}

function typeDef(idl: Idl, name: string) {
  const t = idl.types?.find((x) => x.name === name);
  if (!t) throw new Error(`${idl.metadata.name}: unknown type ${name}`);
  return t;
}

/** An IDL type as a `codec.ts` type. */
export function tyOf(idl: Idl, t: IdlType): Ty {
  if (typeof t === 'string') {
    if (t === 'bytes') return { vec: 'u8' };
    return t as Ty;
  }
  if ('option' in t) return { option: tyOf(idl, t.option) };
  if ('coption' in t) throw new Error('coption is not used by these programs');
  if ('vec' in t) return { vec: tyOf(idl, t.vec) };
  if ('array' in t) {
    const [inner, len] = t.array;
    if (typeof len !== 'number') throw new Error('generic array length');
    return inner === 'u8' ? { bytes: len } : { array: [tyOf(idl, inner), len] };
  }
  const def = typeDef(idl, typeof t.defined === 'string' ? t.defined : t.defined.name);
  const k = def.type;
  if (k.kind === 'struct') return { struct: fieldsOf(idl, k.fields) };
  if (k.kind === 'type') return tyOf(idl, k.alias);
  if (k.variants.some((v) => v.fields && v.fields.length > 0)) {
    return { tagged: k.variants.map((v) => [v.name, fieldsOf(idl, v.fields as IdlFields | undefined)] as [string, Field[]]) };
  }
  return { enum: k.variants.map((v) => v.name) };
}

/** Named fields of a struct (tuple structs get `0`, `1`, ...). */
export function fieldsOf(idl: Idl, fields: IdlFields | undefined): Field[] {
  if (!fields || fields.length === 0) return [];
  if (typeof fields[0] === 'object' && fields[0] !== null && 'name' in (fields[0] as object) && 'type' in (fields[0] as object)) {
    return (fields as { name: string; type: IdlType }[]).map((f) => [camel(f.name), tyOf(idl, f.type)]);
  }
  return (fields as IdlType[]).map((t, i) => [String(i), tyOf(idl, t)]);
}

const sameDisc = (a: number[], ns: 'account' | 'event' | 'global', name: string): boolean => Buffer.from(a).equals(discriminator(ns, name));

/** The field schema of an account or event type by name. */
export function structFields(program: string, name: string): Field[] {
  const idl = IDLS[program]!;
  const def = typeDef(idl, name);
  if (def.type.kind !== 'struct') throw new Error(`${name} is not a struct`);
  return fieldsOf(idl, def.type.fields);
}

/** An account codec from the IDL; refuses an IDL whose discriminator is not Anchor's default. */
export function idlAccountCodec<T>(program: string, name: string): AccountCodec<T> {
  const idl = IDLS[program]!;
  const a = idl.accounts?.find((x) => x.name === name);
  if (!a) throw new Error(`${idl.metadata.name}: no account ${name}`);
  if (!sameDisc(a.discriminator, 'account', name)) throw new Error(`${name}: custom discriminator`);
  return accountCodec<T>(name, structFields(program, name));
}

/** Every event of a program as `[name, fields]`, for `events.ts` and the indexer schema. */
export function idlEventSpecs(program: string): [string, Field[]][] {
  const idl = IDLS[program]!;
  return (idl.events ?? []).map((e) => {
    if (!sameDisc(e.discriminator, 'event', e.name)) throw new Error(`${e.name}: custom discriminator`);
    return [e.name, structFields(program, e.name)] as [string, Field[]];
  });
}

/** The argument schema of an instruction. */
export function idlArgs(program: string, name: string): Field[] {
  const idl = IDLS[program]!;
  const ix = idl.instructions.find((x) => x.name === name);
  if (!ix) throw new Error(`${idl.metadata.name}: no instruction ${name}`);
  return ix.args.map((f) => [camel(f.name), tyOf(idl, f.type)] as Field);
}

/**
 * Builds `program::name(args)` from the IDL: exact discriminator, argument encoding and account
 * order. `accounts` is keyed by camelCase account name; accounts with a fixed address and the
 * program's own id may be omitted; an omitted optional account is passed as the program id.
 */
export function idlIx(program: string, name: string, accounts: Record<string, PublicKey | null | undefined>, args: Record<string, unknown> = {}, remaining: AccountMeta[] = []): TransactionInstruction {
  const c = coderOf(program);
  const ix = c.instruction(name);
  const data = Buffer.concat([Buffer.from(ix.discriminator), encode({ struct: idlArgs(program, name) }, args)]);
  return new TransactionInstruction({ programId: c.programId, keys: [...c.metas(name, accounts, args), ...remaining], data });
}

/** The program id an IDL declares. */
export function idlProgramId(program: string): PublicKey { return coderOf(program).programId; }
