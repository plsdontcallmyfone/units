/**
 * Changed by Hookwars: `POST /v1/submit` sends a transaction the wallet signed (a prepare's
 * output) to the cluster and waits for it to confirm, so the site never needs the RPC URL (app
 * audit A-2). It refuses anything that is not one fully signed transaction of at most one packet.
 */
import { VersionedTransaction, type Connection } from '@solana/web3.js';
import { PrepareError } from './prepares.ts';

/** The packet limit (`solana_packet::PACKET_DATA_SIZE`). */
const PACKET = 1232;
/** Base64 of one packet, padding included. */
const MAX_B64 = Math.ceil(PACKET / 3) * 4;

/** Decodes and checks a submitted transaction; exported for the tests. */
export function signedTransaction(body: Record<string, unknown>): VersionedTransaction {
  const b64 = body.transaction;
  if (typeof b64 !== 'string' || b64.length === 0 || b64.length > MAX_B64 || !/^[A-Za-z0-9+/]+={0,2}$/.test(b64)) {
    throw new PrepareError(400, 'BadRequest', '"transaction" must be one base64 transaction of at most one packet.');
  }
  let tx: VersionedTransaction;
  try { tx = VersionedTransaction.deserialize(Buffer.from(b64, 'base64')); } catch { throw new PrepareError(400, 'BadRequest', '"transaction" is not a transaction.'); }
  const need = tx.message.header.numRequiredSignatures;
  if (tx.signatures.length < need || tx.signatures.slice(0, need).some((s) => s.every((x) => x === 0))) {
    throw new PrepareError(400, 'Unsigned', 'Every required signature must be present before the transaction is sent.');
  }
  return tx;
}

export async function submit(conn: Connection, body: Record<string, unknown>): Promise<{ signature: string }> {
  const tx = signedTransaction(body);
  let signature: string;
  try {
    signature = await conn.sendRawTransaction(tx.serialize(), { skipPreflight: false, preflightCommitment: 'confirmed', maxRetries: 3 });
  } catch (e) {
    const logs = (e as { logs?: string[] }).logs ?? [];
    const line = [...logs].reverse().find((l) => /Error Message:|failed/.test(l));
    throw new PrepareError(409, 'Refused', line ? `The programs refused this: ${line.replace(/^Program log: /, '')}` : 'The cluster refused this transaction.');
  }
  const { blockhash, lastValidBlockHeight } = await conn.getLatestBlockhash('confirmed');
  const r = await conn.confirmTransaction({ signature, blockhash: tx.message.recentBlockhash || blockhash, lastValidBlockHeight }, 'confirmed');
  if (r.value.err) throw new PrepareError(409, 'Failed', 'The transaction landed and failed.');
  return { signature };
}
