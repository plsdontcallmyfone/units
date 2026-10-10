// Domain-separated signed statements: the signed bytes are sha256 over a purpose prefix, so a
// signature made for one purpose (a linked-proof statement, a rules document, an off-chain offer)
// can never be replayed as another, nor as a Solana transaction.
import { createHash, createPrivateKey, createPublicKey, sign, verify } from 'node:crypto';
import { PublicKey, type Keypair } from '@solana/web3.js';

export const STATEMENT_DOMAIN = 'units-statement-v1';

/** Purposes the runtime signs for. A new purpose is a new string, never a reuse. */
export const PURPOSES = ['link-proof', 'rules-document', 'offer', 'provenance-log'] as const;
export type Purpose = (typeof PURPOSES)[number];

export function statementDigest(purpose: Purpose, payload: Uint8Array): Buffer {
  if (!(PURPOSES as readonly string[]).includes(purpose)) throw new Error(`unknown statement purpose "${purpose}"`);
  return createHash('sha256').update(STATEMENT_DOMAIN).update(Buffer.from([0])).update(purpose).update(Buffer.from([0])).update(payload).digest();
}

const PKCS8_PREFIX = Buffer.from('302e020100300506032b657004220420', 'hex');
const SPKI_PREFIX = Buffer.from('302a300506032b6570032100', 'hex');

export function signStatement(kp: Keypair, purpose: Purpose, payload: Uint8Array): Buffer {
  const key = createPrivateKey({ key: Buffer.concat([PKCS8_PREFIX, Buffer.from(kp.secretKey.subarray(0, 32))]), format: 'der', type: 'pkcs8' });
  return sign(null, statementDigest(purpose, payload), key);
}

export function verifyStatement(signer: PublicKey, purpose: Purpose, payload: Uint8Array, signature: Uint8Array): boolean {
  const key = createPublicKey({ key: Buffer.concat([SPKI_PREFIX, signer.toBuffer()]), format: 'der', type: 'spki' });
  try { return verify(null, statementDigest(purpose, payload), key, signature); } catch { return false; }
}
