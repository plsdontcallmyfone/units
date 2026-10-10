/**
 * Submissions, their queue and the runs. A submission is named by the sha256 of what was sent
 * (the tarball's bytes, or `git:<url>@<commit>`); nothing about who sent it is stored. Each run
 * unpacks into a fresh temporary directory and has two stages (security review 3 M-9): the build,
 * which runs the submitter's build scripts and macros, goes through the operator's sandbox command
 * with no key in reach (`hooklab build`); once its whole process group is gone, `hooklab check
 * --no-build` runs the property suite on the built `.so` in LiteSVM and signs the report with the
 * lab key, outside the sandbox. The git fetch also goes through the sandbox and its result is
 * held to the tarball limits (L-8). The signed report is kept and the directory deleted.
 *
 * App pass 5: the build stage gets a fixed environment (no variable of the service reaches it),
 * cargo runs offline against the operator's primed registry cache when one is set, the fetch can
 * use its own sandbox prefix (it needs the network; the build must not), and the lab records the
 * build's provenance itself, outside the sandbox: the source tree hash, the `Cargo.lock` hash and
 * the toolchain versions, so anyone can rebuild the same source and compare `code_hash`.
 */
import { execFile, spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, lstatSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, renameSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { crateRoot, joinRoot, readTarball, writeEntries, type TarLimits } from './tar.ts';

export type State = 'queued' | 'running' | 'done' | 'error';

export interface Status {
  id: string;
  kind: 'tarball' | 'git';
  state: State;
  /** `pass` or `fail` once done. */
  verdict: string | null;
  /** Why a run could not finish (not a verdict). */
  error: string | null;
  createdAt: number;
  startedAt: number | null;
  finishedAt: number | null;
  /** Git submissions: the repository and commit. */
  git: { url: string; commit: string } | null;
}

export interface LabConfig {
  dataDir: string;
  /** The `hooklab` binary. */
  bin: string;
  /** The lab's signing keypair file (Solana JSON). */
  keyPath: string;
  /** Command prefix every run goes through (e.g. a bubblewrap or container line); empty = none. */
  sandbox: string[];
  /** Allow runs with no sandbox (local development and tests only). */
  unsandboxed: boolean;
  /** Command prefix of the git fetch (it needs the network); empty = `sandbox`. */
  fetchSandbox?: string[];
  /** A primed cargo home (registry cache) the build reads offline; null = cargo's default, online. */
  cargoHome?: string | null;
  /** PATH of the build stage (the toolchain's bin directories); empty = the service's PATH. */
  buildPath?: string;
  /** Per-run limit, milliseconds. */
  timeoutMs: number;
  /** Extra arguments for `hooklab check` (e.g. `--ops 48`). */
  checkArgs: string[];
  tar: TarLimits;
  /** Concurrent runs. */
  workers: number;
}

export class SubmissionError extends Error {}

const GIT_URL = /^https:\/\/github\.com\/[A-Za-z0-9](?:[A-Za-z0-9-]{0,38})\/[A-Za-z0-9._-]{1,100}?(?:\.git)?$/;
const COMMIT = /^[0-9a-f]{40}$/;
const ID = /^[0-9a-f]{64}$/;

const sha256 = (b: Buffer | string): string => createHash('sha256').update(b).digest('hex');

export class Lab {
  private queue: string[] = [];
  private running = 0;
  private idle: Array<() => void> = [];

  readonly cfg: LabConfig;

  constructor(cfg: LabConfig) {
    this.cfg = cfg;
    mkdirSync(cfg.dataDir, { recursive: true });
  }

  private dir(id: string): string {
    if (!ID.test(id)) throw new SubmissionError('bad id');
    return join(this.cfg.dataDir, id);
  }

  status(id: string): Status | null {
    if (!ID.test(id)) return null;
    const f = join(this.dir(id), 'status.json');
    return existsSync(f) ? (JSON.parse(readFileSync(f, 'utf8')) as Status) : null;
  }

  report(id: string): string | null {
    if (!ID.test(id)) return null;
    const f = join(this.dir(id), 'report.json');
    return existsSync(f) ? readFileSync(f, 'utf8') : null;
  }

  private save(s: Status): void {
    const d = this.dir(s.id);
    mkdirSync(d, { recursive: true });
    const tmp = join(d, 'status.json.tmp');
    writeFileSync(tmp, JSON.stringify(s));
    renameSync(tmp, join(d, 'status.json'));
  }

  /** A tarball submission. The archive is fully checked before anything is queued. */
  submitTarball(gz: Buffer): Status {
    const entries = readTarball(gz, this.cfg.tar);
    crateRoot(entries);
    const id = sha256(gz);
    const known = this.status(id);
    if (known) return known;
    mkdirSync(this.dir(id), { recursive: true });
    writeFileSync(join(this.dir(id), 'submission.tgz'), gz);
    return this.enqueue({ id, kind: 'tarball', git: null });
  }

  /** A git submission: a public GitHub repository at a full commit id. */
  submitGit(url: unknown, commit: unknown): Status {
    if (typeof url !== 'string' || !GIT_URL.test(url)) throw new SubmissionError('git must be https://github.com/<owner>/<repo>');
    if (typeof commit !== 'string' || !COMMIT.test(commit)) throw new SubmissionError('commit must be a full 40-character commit id');
    const id = sha256(`git:${url}@${commit}`);
    const known = this.status(id);
    if (known) return known;
    return this.enqueue({ id, kind: 'git', git: { url, commit } });
  }

  private enqueue(p: Pick<Status, 'id' | 'kind' | 'git'>): Status {
    const s: Status = { ...p, state: 'queued', verdict: null, error: null, createdAt: Date.now(), startedAt: null, finishedAt: null };
    this.save(s);
    this.queue.push(s.id);
    this.pump();
    return s;
  }

  /** Requeues submissions a restart interrupted. */
  recover(ids: string[]): void {
    for (const id of ids) {
      const s = this.status(id);
      if (s && (s.state === 'queued' || s.state === 'running')) {
        this.save({ ...s, state: 'queued', startedAt: null });
        this.queue.push(id);
      }
    }
    this.pump();
  }

  queued(): number { return this.queue.length; }

  /** Resolves when nothing is queued or running. */
  drained(): Promise<void> {
    if (this.queue.length === 0 && this.running === 0) return Promise.resolve();
    return new Promise((r) => this.idle.push(r));
  }

  private pump(): void {
    while (this.running < this.cfg.workers && this.queue.length > 0) {
      const id = this.queue.shift()!;
      this.running++;
      void this.run(id).finally(() => {
        this.running--;
        this.pump();
        if (this.queue.length === 0 && this.running === 0) this.idle.splice(0).forEach((r) => r());
      });
    }
  }

  private async run(id: string): Promise<void> {
    const s = this.status(id);
    if (!s) return;
    this.save({ ...s, state: 'running', startedAt: Date.now() });
    const work = mkdtempSync(join(tmpdir(), 'hooklab-run-'));
    try {
      if (!this.cfg.unsandboxed && this.cfg.sandbox.length === 0) {
        throw new SubmissionError('no sandbox configured: the operator must set HOOKLAB_SANDBOX (a build runs the submitter\'s code)');
      }
      const src = join(work, 'src');
      mkdirSync(src);
      let crate: string;
      if (s.kind === 'tarball') {
        const entries = readTarball(readFileSync(join(this.dir(id), 'submission.tgz')), this.cfg.tar);
        writeEntries(src, entries);
        crate = joinRoot(src, crateRoot(entries));
      } else {
        await this.fetchGit(s.git!.url, s.git!.commit, src);
        crate = src;
      }
      const reportTmp = join(work, 'report.json');
      const so = join(work, 'out', 'template.so');
      const manifestPath = join(crate, 'hooklab.json');
      const manifest = existsSync(manifestPath) && regularFile(manifestPath) ? readFileSync(manifestPath) : null;
      if (!manifest) throw new SubmissionError('hooklab.json must be a regular file at the crate root');
      // Stage 1, sandboxed, no key: the build (the submitter's code runs here).
      const features = this.cfg.checkArgs.indexOf('--features');
      const buildArgs = [this.cfg.bin, 'build', crate, '--out-dir', join(work, 'out'), '--so-out', so,
        ...(features >= 0 && this.cfg.checkArgs[features + 1] ? ['--features', this.cfg.checkArgs[features + 1]!] : [])];
      const provenance = await this.provenance(crate);
      const built = await this.exec([...this.cfg.sandbox, ...buildArgs], work, this.buildEnv(work), true);
      const buildError = built !== 0 || !existsSync(so)
        ? (existsSync(`${so}.err`) ? readFileSync(`${so}.err`, 'utf8').slice(0, 4000) : `the build ended with exit ${built}`)
        : null;
      // Stage 2, outside the sandbox, with the key: the suite on the built bytecode, never a build.
      // What stage 1 left is only read if it is a plain file, and the manifest must be the one sent.
      if (!regularFile(manifestPath) || !readFileSync(manifestPath).equals(manifest)) throw new SubmissionError('the build changed hooklab.json');
      for (const f of [so, `${so}.json`]) if (existsSync(f) && !regularFile(f)) throw new SubmissionError('the build left a link where the lab reads its output');
      // The build facts the report signs are the lab's own (read outside the sandbox); only the
      // features come from the run's arguments, never from what the sandbox wrote.
      const metaPath = join(work, 'build-meta.json');
      writeFileSync(metaPath, JSON.stringify({ ...provenance, features: features >= 0 && this.cfg.checkArgs[features + 1] ? this.cfg.checkArgs[features + 1]!.split(',') : [] }));
      const errFile = join(work, 'build-error.txt');
      if (buildError !== null) writeFileSync(errFile, buildError);
      const args = [
        this.cfg.bin, 'check', crate, '--no-build',
        ...(buildError === null ? ['--so', so, '--build-meta', metaPath] : ['--build-error', errFile]),
        '--key', this.cfg.keyPath,
        '--report', reportTmp,
        '--submission', id,
        ...this.cfg.checkArgs.filter((_, i, a) => a[i] !== '--features' && a[i - 1] !== '--features'),
      ];
      rmSync(reportTmp, { force: true });
      const code = await this.exec(args, work);
      if (!existsSync(reportTmp)) throw new SubmissionError(`the lab produced no report (exit ${code})`);
      const report = readFileSync(reportTmp, 'utf8');
      const verdict = (JSON.parse(report) as { body?: { verdict?: string } }).body?.verdict ?? null;
      writeFileSync(join(this.dir(id), 'report.json'), report);
      this.save({ ...this.status(id)!, state: 'done', verdict, finishedAt: Date.now() });
    } catch (e) {
      const error = e instanceof Error ? e.message : String(e);
      this.save({ ...this.status(id)!, state: 'error', error: error.slice(0, 2000), finishedAt: Date.now() });
    } finally {
      rmSync(work, { recursive: true, force: true });
    }
  }

  private async fetchGit(url: string, commit: string, dest: string): Promise<void> {
    // Review 3 L-8: through the sandbox like the build, and held to the tarball limits after.
    const prefix = this.cfg.fetchSandbox && this.cfg.fetchSandbox.length ? this.cfg.fetchSandbox : this.cfg.sandbox;
    const git = [...prefix, 'git', '-c', 'core.symlinks=false', '-c', 'protocol.allow=never', '-c', 'protocol.https.allow=always', '-C', dest];
    for (const step of [['init', '-q'], ['fetch', '-q', '--depth', '1', '--no-tags', url, commit], ['checkout', '-q', 'FETCH_HEAD']]) {
      const code = await this.exec([...git, ...step], dest, { GIT_TERMINAL_PROMPT: '0', GIT_CONFIG_NOSYSTEM: '1' });
      if (code !== 0) throw new SubmissionError(`git ${step[0]} failed (${code})`);
    }
    rmSync(join(dest, '.git'), { recursive: true, force: true });
    const { bytes, files } = treeSize(dest);
    if (files > this.cfg.tar.maxFiles) throw new SubmissionError(`the repository has more than ${this.cfg.tar.maxFiles} files`);
    if (bytes > this.cfg.tar.maxUnpacked) throw new SubmissionError(`the repository is larger than ${this.cfg.tar.maxUnpacked} bytes`);
    for (const f of ['Cargo.toml', 'Cargo.lock', 'hooklab.json']) {
      if (!existsSync(join(dest, f))) throw new SubmissionError(`the repository must have ${f} at its root`);
    }
  }

  /**
   * The build stage's whole environment: nothing of the service's (its key path, its tokens) is
   * passed; cargo is offline against `cargoHome` when the operator primed one.
   */
  private buildEnv(work: string): Record<string, string> {
    const env: Record<string, string> = {
      PATH: this.cfg.buildPath || process.env.PATH || '/usr/bin:/bin',
      HOME: work, TMPDIR: work, LANG: 'C.UTF-8', CARGO_TERM_COLOR: 'never', SOURCE_DATE_EPOCH: '0',
    };
    for (const k of ['RUSTUP_HOME', 'RUSTUP_TOOLCHAIN']) if (process.env[k]) env[k] = process.env[k]!;
    if (this.cfg.cargoHome) Object.assign(env, { CARGO_HOME: this.cfg.cargoHome, CARGO_NET_OFFLINE: 'true' });
    else if (process.env.CARGO_HOME) env.CARGO_HOME = process.env.CARGO_HOME;
    return env;
  }

  /** What the report records about the build, read by the lab outside the sandbox. */
  private async provenance(crate: string): Promise<BuildProvenance> {
    const lock = join(crate, 'Cargo.lock');
    const v = async (argv: string[]) => (await this.capture(argv, crate)).split('\n')[0]!.trim();
    return {
      toolchain: await v(['cargo', 'build-sbf', '--version']),
      tools_version: TOOLS_VERSION, arch: SBF_ARCH,
      rustc: await v(['rustc', '--version']),
      source_sha256: treeHash(crate),
      cargo_lock_sha256: existsSync(lock) && regularFile(lock) ? sha256(readFileSync(lock)) : null,
      offline: Boolean(this.cfg.cargoHome),
    };
  }

  /** First output of a short command the lab runs itself (versions); empty when it fails. */
  private capture(argv: string[], cwd: string): Promise<string> {
    return new Promise((resolveOut) => {
      const [cmd, ...rest] = argv;
      execFile(cmd!, rest, { cwd, timeout: 30_000, env: { PATH: this.cfg.buildPath || process.env.PATH || '/usr/bin:/bin', ...(process.env.RUSTUP_HOME ? { RUSTUP_HOME: process.env.RUSTUP_HOME } : {}), ...(process.env.HOME ? { HOME: process.env.HOME } : {}) } }, (err, stdout) => resolveOut(err ? '' : String(stdout)));
    });
  }

  /**
   * Runs `argv` in its own process group and kills the whole group when it exits or times out, so
   * nothing a build script started outlives its stage (review 3 M-9). `isolated` passes `env` as
   * the whole environment instead of adding it to the service's.
   */
  private exec(argv: string[], cwd: string, env: Record<string, string> = {}, isolated = false): Promise<number> {
    return new Promise((resolveExit, reject) => {
      const [cmd, ...rest] = argv;
      const child = spawn(cmd!, rest, { cwd, env: isolated ? env : { ...process.env, ...env }, stdio: ['ignore', 'ignore', 'ignore'], detached: true });
      const killGroup = () => { try { if (child.pid) process.kill(-child.pid, 'SIGKILL'); } catch { /* the group is gone */ } };
      const timer = setTimeout(() => {
        killGroup();
        reject(new SubmissionError(`the run passed the ${this.cfg.timeoutMs} ms limit`));
      }, this.cfg.timeoutMs);
      child.on('error', (e) => { clearTimeout(timer); reject(e); });
      child.on('exit', (code) => { clearTimeout(timer); killGroup(); resolveExit(code ?? -1); });
    });
  }
}

/** The build facts a report signs (`build` in the report body). */
export interface BuildProvenance {
  toolchain: string; tools_version: string; arch: string; rustc: string;
  source_sha256: string; cargo_lock_sha256: string | null; offline: boolean;
}

/** Platform tools and SBF architecture the CLI builds with (tools/hooklab/src/build.rs). */
export const TOOLS_VERSION = 'v1.57';
export const SBF_ARCH = 'v3';

/**
 * sha256 over the source tree: for every regular file in path order, `<path>\0<sha256 of its
 * bytes>\n`. Anyone can recompute it from the same source (no timestamps or modes in it).
 */
export function treeHash(root: string): string {
  const files: string[] = [];
  const walk = (dir: string, rel: string) => {
    for (const name of readdirSync(dir).sort()) {
      const p = join(dir, name); const r = rel ? `${rel}/${name}` : name;
      const st = lstatSync(p);
      if (st.isDirectory()) { if (name !== 'target' && name !== '.git') walk(p, r); } else if (st.isFile()) files.push(r);
    }
  };
  walk(root, '');
  const h = createHash('sha256');
  for (const f of files.sort()) h.update(`${f}\0${sha256(readFileSync(join(root, f)))}\n`);
  return h.digest('hex');
}
/** A regular file, not a link. */
function regularFile(p: string): boolean {
  try { return lstatSync(p).isFile(); } catch { return false; }
}

/** Bytes and regular files under `dir` (links are not followed). */
function treeSize(dir: string): { bytes: number; files: number } {
  let bytes = 0;
  let files = 0;
  const walk = (d: string) => {
    for (const name of readdirSync(d)) {
      const p = join(d, name);
      const st = lstatSync(p);
      if (st.isDirectory()) walk(p);
      else { files++; bytes += st.size; }
    }
  };
  walk(dir);
  return { bytes, files };
}
