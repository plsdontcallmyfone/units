/** The public half of a Solana keypair file (64 bytes: secret, then public), in base58. */
const ALPHABET = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';

export function base58(bytes: Uint8Array): string {
  let n = 0n;
  for (const b of bytes) n = n * 256n + BigInt(b);
  let s = '';
  while (n > 0n) { s = ALPHABET[Number(n % 58n)]! + s; n /= 58n; }
  for (const b of bytes) { if (b !== 0) break; s = '1' + s; }
  return s;
}

export function publicKeyOf(keypair: number[]): string {
  if (!Array.isArray(keypair) || keypair.length !== 64) throw new Error('not a keypair file');
  return base58(Uint8Array.from(keypair.slice(32)));
}
