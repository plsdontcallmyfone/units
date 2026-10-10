// Changed by Hookwars: new file (launch page polish). The checks the launch page runs as the
// creator types, each mirroring a program rule and named by the error the program would return.
// Limits are the build values of docs/guide/reference/limits.md (PARAMS where the app has them).
import { PARAMS, TEMPLATES, type EquipRule, type SlotKind, type TemplateDef } from '@hookwars/shared';

export const MAX_SLOTS = PARAMS.find((p) => p.name === 'MAX_SLOTS')?.value ?? 4;
export const MAX_CUTTING = PARAMS.find((p) => p.name === 'MAX_CUTTING_SLOTS')?.value ?? 3;
/** Holder memory per holding, and the kit's share of it (token program; limits.md). */
export const HOLDER_BYTES = 64;
export const KIT_BYTES = 32;
/** Modules per composite (08 2.2, provisional in limits.md). */
export const MAX_MODULES = 4;
/** The launch's own transactions as measured (limits.md "Launch", with the token's table). */
export const MEASURED = [
  { path: 'prepare_launch, kit + 3 slots', bytes: 525, cu: 52_327 },
  { path: 'create_prepared_launch, kit + 3 pool items', bytes: 545, cu: 359_908 },
];
export const PACKET = 1_232;

export interface SlotDraft {
  kind: SlotKind; rule: EquipRule; maxCutBps: number; noticeSecs: number;
  templateId: number | null; launchItem: string; targets: string[];
}

/** Templates whose callbacks answer `on_touch` and that may burn (the API's TOUCH and BURN sets). */
const TOUCH = new Set([1]);
const BURN = new Set([32, 33]);
export const answersTouch = (id: number | null) => id !== null && TOUCH.has(id);
export const mayBurn = (id: number | null) => id !== null && BURN.has(id);

/** Bytes of holder memory a slot takes: the template's range plus its epoch byte (04 2.4). */
export function slotBytes(s: Pick<SlotDraft, 'templateId'>): number {
  const t = TEMPLATES.find((x) => x.id === s.templateId);
  return t && t.dataBytes > 0 ? t.dataBytes + 1 : 0;
}

/** A slot cuts on the token side when it may cut and is not Pool or War (01: those cut only on the pool side). */
export const cutsTokenSide = (s: Pick<SlotDraft, 'kind' | 'maxCutBps'>) => s.maxCutBps > 0 && s.kind !== 'pool' && s.kind !== 'war';

export interface Budget { slots: number; bytesUsed: number; bytesFree: number; cutting: number; cuttingMax: number; problems: string[] }

export function budget(slots: SlotDraft[], kit: boolean): Budget {
  const bytesFree = HOLDER_BYTES - (kit ? KIT_BYTES : 0);
  const bytesUsed = slots.reduce((n, s) => n + slotBytes(s), 0);
  const cutting = slots.filter(cutsTokenSide).length;
  const cuttingMax = MAX_CUTTING - (kit ? 1 : 0);
  const n = slots.length + (kit ? 1 : 0);
  const problems: string[] = [];
  if (n > MAX_SLOTS) problems.push(`At most ${MAX_SLOTS} slots, the kit's included (TooManySlots).`);
  if (bytesUsed > bytesFree) problems.push(`The items need ${bytesUsed} bytes of holder memory; ${bytesFree} are left (HookDataOverflow).`);
  if (cutting > cuttingMax) problems.push(`${cutting} token-side cutting slots; at most ${cuttingMax} (TooManyCuttingSlots).`);
  if (slots.filter((s) => s.kind === 'war').length > 1) problems.push('One War slot at most.');
  slots.forEach((s, i) => {
    if (!Number.isInteger(s.maxCutBps) || s.maxCutBps < 0 || s.maxCutBps > 10_000) problems.push(`Slot ${i + (kit ? 1 : 0)}: the max cut is 0 to 10,000 bps.`);
    if (!Number.isInteger(s.noticeSecs) || s.noticeSecs < 0) problems.push(`Slot ${i + (kit ? 1 : 0)}: the notice is whole seconds.`);
    const t = TEMPLATES.find((x) => x.id === s.templateId);
    if (t && t.kind !== s.kind && !(s.templateId === 41)) problems.push(`Slot ${i + (kit ? 1 : 0)}: ${t.name} is a ${t.kind} template, not ${s.kind} (KindMismatch).`);
  });
  return { slots: n, bytesUsed, bytesFree, cutting, cuttingMax, problems };
}

// ------------------------------------------------------------------------------ composites --

/** Templates a composite may not carry (`hookwars_common::composable`). */
const NOT_COMPOSABLE = new Set([9, 41, 42, 43, 44, 45]);
/** Whether a slot of `host` kind may run a module of `kind` (08 2.8, `composite::hostable`). */
export function hostable(host: string, kind: string): boolean {
  switch (host) {
    case 'fee': return kind === 'fee';
    case 'reward': return kind === 'reward' || kind === 'fee';
    case 'defense': return kind === 'defense';
    case 'relation': return kind === 'relation';
    case 'pool': return ['pool', 'fee', 'defense', 'reward'].includes(kind);
    default: return false;
  }
}
/** The narrowest kind that hosts every module (`composite::host_kind`). */
export function hostKind(kinds: string[]): string | null {
  return ['fee', 'reward', 'defense', 'relation', 'pool'].find((h) => kinds.every((k) => hostable(h, k))) ?? null;
}

export interface ModuleDraft { templateId: number; params: number[]; targetStart: number; targetCount: number }

/** `composite::validate_modules` as far as the page can know it: count, composable, conflicts, host,
 * one touch module, target slices, bytes. Floors and ceilings are on chain; the armory checks them. */
export function compositeProblems(modules: ModuleDraft[], slotKind?: SlotKind): { problems: string[]; host: string | null; bytes: number; templates: TemplateDef[] } {
  const problems: string[] = [];
  const templates = modules.map((m) => TEMPLATES.find((t) => t.id === m.templateId)).filter((t): t is TemplateDef => Boolean(t));
  if (modules.length === 0 || modules.length > MAX_MODULES) problems.push(`One to ${MAX_MODULES} modules (TooManyModules).`);
  modules.forEach((m, i) => { if (NOT_COMPOSABLE.has(m.templateId) || !TEMPLATES.some((t) => t.id === m.templateId)) problems.push(`Module ${i + 1} cannot be a module (NotComposable).`); });
  const count = (id: number) => modules.filter((m) => m.templateId === id).length;
  if ((count(2) && count(40)) || (count(1) && count(29)) || count(24) > 1 || count(38) > 1) problems.push('Shield with Patience, Raid with Mercenary, or a second Loyalty Pot or First Blood share one state (ModuleConflict).');
  if (modules.filter((m) => answersTouch(m.templateId)).length > 1) problems.push('Only one module may answer touches (ModuleConflict).');
  const host = templates.length === modules.length ? hostKind(templates.map((t) => t.kind)) : null;
  if (modules.length && !host) problems.push('No one slot kind hosts all these modules (KindMismatch).');
  if (host && slotKind && !hostable(slotKind, host) && slotKind !== host) problems.push(`A ${slotKind} slot cannot host a ${host} composite (KindMismatch).`);
  const used = new Set<number>();
  for (const m of modules) for (let t = m.targetStart; t < m.targetStart + m.targetCount; t++) { if (used.has(t)) problems.push('Two modules claim the same target (BadTargets).'); used.add(t); }
  const bytes = templates.reduce((n, t) => n + t.dataBytes, 0);
  if (bytes > 63) problems.push(`${bytes} bytes of holder memory; a composite holds at most 63 (CompositeBytes).`);
  return { problems: [...new Set(problems)], host, bytes, templates };
}
