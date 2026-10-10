/**
 * A gzip tarball reader that only ever writes regular files and directories under one root.
 * No dependency and no system `tar`: the archive is parsed here, so nothing in it can name a
 * path outside the root, a link, a device or a fifo. macOS AppleDouble files (`._*`) and pax
 * headers are understood; everything else unusual is refused.
 */
import { gunzipSync } from 'node:zlib';
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve, sep } from 'node:path';

export interface TarLimits {
  /** Largest gzip upload, bytes. */
  maxCompressed: number;
  /** Largest unpacked total, bytes. */
  maxUnpacked: number;
  /** Most files. */
  maxFiles: number;
  /** Longest path, bytes. */
  maxPath: number;
}

export class TarError extends Error {}

export interface TarEntry { path: string; data: Buffer }

const text = (b: Buffer, at: number, len: number): string => {
  const s = b.subarray(at, at + len);
  const z = s.indexOf(0);
  return (z < 0 ? s : s.subarray(0, z)).toString('utf8');
};

const octal = (b: Buffer, at: number, len: number): number => {
  const s = text(b, at, len).trim();
  if (s === '') return 0;
  if (!/^[0-7]+$/.test(s)) throw new TarError('bad number in a tar header');
  return parseInt(s, 8);
};

/** Pax records `len key=value\n`; only `path` matters here. */
function paxPath(data: Buffer): string | null {
  let at = 0;
  let path: string | null = null;
  while (at < data.length) {
    const sp = data.indexOf(0x20, at);
    if (sp < 0) break;
    const len = Number(data.subarray(at, sp).toString('ascii'));
    if (!Number.isInteger(len) || len <= 0 || at + len > data.length) throw new TarError('bad pax record');
    const rec = data.subarray(sp + 1, at + len - 1).toString('utf8');
    const eq = rec.indexOf('=');
    if (eq > 0 && rec.slice(0, eq) === 'path') path = rec.slice(eq + 1);
    at += len;
  }
  return path;
}

/** A relative path with no `..`, no absolute start, no backslash, no NUL; null for a skipped one. */
export function cleanPath(raw: string, limits: TarLimits): string | null {
  let p = raw.replace(/^\.\/+/, '');
  if (p === '' || p === '.') return null;
  if (p.endsWith('/')) p = p.slice(0, -1);
  if (Buffer.byteLength(p) > limits.maxPath) throw new TarError(`path too long: ${p.slice(0, 64)}`);
  if (p.startsWith('/') || p.includes('\\') || p.includes('\0') || /^[a-zA-Z]:/.test(p)) throw new TarError(`unsafe path ${p}`);
  const parts = p.split('/');
  if (parts.some((s) => s === '..' || s === '.' || s === '')) throw new TarError(`unsafe path ${p}`);
  if (parts.some((s) => s.startsWith('._')) || parts.includes('__MACOSX')) return null;
  return p;
}

/** Reads every file of a gzip tarball (directories are implied by file paths). */
export function readTarball(gz: Buffer, limits: TarLimits): TarEntry[] {
  if (gz.length > limits.maxCompressed) throw new TarError('archive too large');
  let tar: Buffer;
  try {
    tar = gunzipSync(gz, { maxOutputLength: limits.maxUnpacked + 1024 * 1024 });
  } catch {
    throw new TarError('not a gzip archive, or it unpacks too large');
  }
  const out: TarEntry[] = [];
  const seen = new Set<string>();
  let total = 0;
  let at = 0;
  let nextPath: string | null = null;
  while (at + 512 <= tar.length) {
    const h = tar.subarray(at, at + 512);
    if (h.every((x) => x === 0)) break;
    let sum = 0;
    for (let i = 0; i < 512; i++) sum += i >= 148 && i < 156 ? 0x20 : h[i]!;
    if (sum !== octal(h, 148, 8)) throw new TarError('bad tar checksum');
    const size = octal(h, 124, 12);
    const type = String.fromCharCode(h[156]!);
    const prefix = text(h, 345, 155);
    const name = text(h, 0, 100);
    const data = tar.subarray(at + 512, at + 512 + size);
    if (data.length !== size) throw new TarError('truncated tar');
    at += 512 + Math.ceil(size / 512) * 512;
    if (type === 'x') { nextPath = paxPath(data); continue; }
    if (type === 'g') continue;
    if (type === 'L') { nextPath = text(data, 0, data.length); continue; }
    const raw = nextPath ?? (prefix ? `${prefix}/${name}` : name);
    nextPath = null;
    if (type === '5') { cleanPath(raw, limits); continue; }
    if (type !== '0' && type !== '\0' && type !== '7') throw new TarError(`refused entry type '${type}' (links, devices and fifos are not accepted): ${raw.slice(0, 64)}`);
    const p = cleanPath(raw, limits);
    if (p === null) continue;
    if (seen.has(p)) throw new TarError(`duplicate path ${p}`);
    seen.add(p);
    total += size;
    if (total > limits.maxUnpacked) throw new TarError('archive unpacks too large');
    if (out.length + 1 > limits.maxFiles) throw new TarError('too many files');
    out.push({ path: p, data: Buffer.from(data) });
  }
  return out;
}

/** Writes entries under `root` (each path resolved and checked to stay inside). */
export function writeEntries(root: string, entries: TarEntry[]): void {
  const base = resolve(root);
  for (const e of entries) {
    const target = resolve(base, e.path);
    if (!target.startsWith(base + sep)) throw new TarError(`unsafe path ${e.path}`);
    mkdirSync(dirname(target), { recursive: true });
    writeFileSync(target, e.data, { flag: 'wx', mode: 0o644 });
  }
}

/** The crate directory: the root, or the one top-level directory every file is in. */
export function crateRoot(entries: TarEntry[]): string {
  const tops = new Set(entries.map((e) => e.path.split('/')[0]));
  const one = tops.size === 1 && entries.every((e) => e.path.includes('/')) ? [...tops][0]! : '';
  const has = (f: string): boolean => entries.some((e) => e.path === (one ? `${one}/${f}` : f));
  for (const f of ['Cargo.toml', 'Cargo.lock', 'hooklab.json']) {
    if (!has(f)) throw new TarError(`the crate must have ${f} at its root`);
  }
  return one;
}

export const joinRoot = (root: string, sub: string): string => (sub ? join(root, sub) : root);
