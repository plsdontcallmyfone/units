/**
 * Submissions, their queue and the runs. A submission is named by the sha256 of what was sent
 * (the tarball's bytes, or `git:<url>@<commit>`); nothing about who sent it is stored. Each run
 * unpacks into a fresh temporary directory, runs `hooklab check` with a timeout inside the
 * operator's sandbox command, keeps the signed report and deletes the directory.
 */
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, renameSync, rmSync, writeFileSync } from 'node:fs';
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
      const args = [
        this.cfg.bin, 'check', crate,
        '--key', this.cfg.keyPath,
        '--report', reportTmp,
        '--submission', id,
        '--out-dir', join(work, 'out'),
        ...this.cfg.checkArgs,
      ];
      const code = await this.exec([...this.cfg.sandbox, ...args], work);
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
    const git = ['git', '-c', 'core.symlinks=false', '-c', 'protocol.allow=never', '-c', 'protocol.https.allow=always', '-C', dest];
    for (const step of [['init', '-q'], ['fetch', '-q', '--depth', '1', '--no-tags', url, commit], ['checkout', '-q', 'FETCH_HEAD']]) {
      const code = await this.exec([...git, ...step], dest, { GIT_TERMINAL_PROMPT: '0', GIT_CONFIG_NOSYSTEM: '1' });
      if (code !== 0) throw new SubmissionError(`git ${step[0]} failed (${code})`);
    }
    rmSync(join(dest, '.git'), { recursive: true, force: true });
    for (const f of ['Cargo.toml', 'Cargo.lock', 'hooklab.json']) {
      if (!existsSync(join(dest, f))) throw new SubmissionError(`the repository must have ${f} at its root`);
    }
  }

  private exec(argv: string[], cwd: string, env: Record<string, string> = {}): Promise<number> {
    return new Promise((resolveExit, reject) => {
      const [cmd, ...rest] = argv;
      const child = spawn(cmd!, rest, { cwd, env: { ...process.env, ...env }, stdio: ['ignore', 'ignore', 'ignore'] });
      const timer = setTimeout(() => {
        child.kill('SIGKILL');
        reject(new SubmissionError(`the run passed the ${this.cfg.timeoutMs} ms limit`));
      }, this.cfg.timeoutMs);
      child.on('error', (e) => { clearTimeout(timer); reject(e); });
      child.on('exit', (code) => { clearTimeout(timer); resolveExit(code ?? -1); });
    });
  }
}
