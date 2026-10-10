// Changed by Hookwars: new file (launch page). What a prepared step says about itself is read back
// from its own bytes; composite module lists are refused by the armory's error names.
import { describe, expect, it } from 'vitest';
import { Keypair, PublicKey, TransactionMessage, VersionedTransaction, ComputeBudgetProgram } from '@solana/web3.js';
import bs58 from 'bs58';
import { hookwars } from '@hookwars/sdk';
import { compositeModules, describe as describeStep } from './launch-plan.ts';

describe('describe', () => {
  it('reads the instructions, size, signatures and compute limit from the transaction itself', () => {
    const payer = Keypair.generate().publicKey;
    const mint = Keypair.generate().publicKey;
    const ix = hookwars.initWar(payer, mint);
    const msg = new TransactionMessage({ payerKey: payer, recentBlockhash: bs58.encode(Buffer.alloc(32, 1)), instructions: [ComputeBudgetProgram.setComputeUnitLimit({ units: 120_000 }), ix] }).compileToV0Message();
    const tx = new VersionedTransaction(msg);
    const b64 = Buffer.from(tx.serialize()).toString('base64');
    const st = describeStep({ transaction: b64, version: 'v0', stage: 3, label: 'War chest', extraSigners: [] });
    expect(st.instructions.map((x) => `${x.program}.${x.name}`)).toEqual(['computeBudget.setComputeUnitLimit', 'war.initWar']);
    expect(st.computeLimit).toBe(120_000);
    expect(st.simulatedNow).toBe(true);
    expect(st.signatures).toBe(1);
    expect(st.feeLamports).toBe(5_000);
    expect(st.bytes).toBe(Buffer.from(b64, 'base64').length);
    expect(st.packet).toBe(1_232);
    void PublicKey;
  });
});

describe('compositeModules', () => {
  it('refuses what the armory refuses, by its error names', () => {
    expect(compositeModules([])).toMatchObject({ refused: 'TooManyModules' });
    expect(compositeModules([{ templateId: 9, params: [], targetStart: 0, targetCount: 0 }])).toMatchObject({ refused: 'NotComposable' });
    expect(compositeModules(Array(5).fill({ templateId: 7, params: [1], targetStart: 0, targetCount: 0 }))).toMatchObject({ refused: 'TooManyModules' });
    expect(compositeModules([{ templateId: 7, params: [-1], targetStart: 0, targetCount: 0 }])).toMatchObject({ refused: 'BadParams' });
  });
  it('writes the module list as the armory takes it', () => {
    const r = compositeModules([{ templateId: 7, params: [200_000, 3_600, 4], targetStart: 0, targetCount: 0 }]);
    expect('modules' in r && r.modules[0]).toMatchObject({ templateId: 7, readsModule: 255 });
    expect('modules' in r && (r.modules[0]!.params as number[]).length).toBe(11);
  });
});
