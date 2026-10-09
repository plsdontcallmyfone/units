/** Display helpers. A missing figure is a dash, never zero (06 section 1 rule 3). */
export const DASH = '-';

export function short(addr: string | null | undefined, n = 4): string {
  if (!addr) return DASH;
  return addr.length <= n * 2 + 1 ? addr : `${addr.slice(0, n)}...${addr.slice(-n)}`;
}

export function sol(lamports: string | number | bigint | null | undefined, digits = 4): string {
  if (lamports === null || lamports === undefined || lamports === '') return DASH;
  const v = Number(lamports) / 1e9;
  return `${v.toLocaleString('en-US', { maximumFractionDigits: digits })} SOL`;
}

export function int(v: string | number | null | undefined): string {
  if (v === null || v === undefined || v === '') return DASH;
  return Number(v).toLocaleString('en-US');
}

export function ago(ts: number | null | undefined, now = Date.now() / 1000): string {
  if (!ts) return DASH;
  const d = Math.max(0, Math.round(now - ts));
  if (d < 60) return `${d} s ago`;
  if (d < 3600) return `${Math.floor(d / 60)} min ago`;
  if (d < 86400) return `${Math.floor(d / 3600)} h ago`;
  return `${Math.floor(d / 86400)} d ago`;
}

/** Time until a future timestamp; a past one reads as "now". */
export function until(ts: number | null | undefined, now = Date.now() / 1000): string {
  if (!ts) return DASH;
  const d = Math.round(ts - now);
  if (d <= 0) return 'now';
  if (d < 3600) return `in ${Math.max(1, Math.floor(d / 60))} min`;
  if (d < 86400) return `in ${Math.floor(d / 3600)} h`;
  return `in ${Math.floor(d / 86400)} d`;
}

/** A token amount in base units, compact: 1.25T, 310B. */
export function compact(v: string | number | null | undefined): string {
  if (v === null || v === undefined || v === '') return DASH;
  return Number(v).toLocaleString('en-US', { notation: 'compact', maximumFractionDigits: 2 });
}
