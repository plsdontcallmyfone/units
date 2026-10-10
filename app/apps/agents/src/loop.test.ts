import { describe, expect, it } from 'vitest';
import { PublicKey, TransactionInstruction } from '@solana/web3.js';
import { hookwars, token } from '@hookwars/sdk';
import { commitInstruction } from './directive.ts';
import { tick } from './loop.ts';
import { memoInstruction, messageRef, parseMessage, toJson } from './memo.ts';
import { replay } from './models/stub.ts';
import { wrapSpend } from './plan.ts';
import { emptyBook } from './policy.ts';
import { CRANK_ROUTES } from './router.ts';
import { memoMatches, provenanceOf, verifyProvenance } from './provenance.ts';
import { AGENT, kit, OPEN_CONSTRAINTS, SentSender } from './testkit.ts';

const ITEM_MINT = new PublicKey(new Uint8Array(32).fill(11));
const TOKEN_MINT = new PublicKey(new Uint8Array(32).fill(12));
const OTHER = new PublicKey(new Uint8Array(32).fill(13));
const SIG = '5'.repeat(88);

/** Instruction lists compare by program, metas and data. */
const shape = (ixs: TransactionInstruction[]) => ixs.map((i) => ({ program: i.programId.toBase58(), keys: i.keys.map((k) => `${k.pubkey.toBase58()}:${+k.isSigner}${+k.isWritable}`), data: i.data.toString('hex') }));
const memoOf = (ixs: TransactionInstruction[]) => parseMessage(ixs.at(-1)!.data, 600);

describe('agent loop with the stub model on a mocked chain', () => {
  it('creates an item with the SDK builder, held by the vault, and records provenance in the memo', async () => {
    const k = kit(replay([{ actions: [{ type: 'create_item', templateId: 42, params: [3, 9], royaltyBps: 250 }] }]));
    const r = await tick(k.rt);
    expect(r.results).toEqual([expect.objectContaining({ type: 'create_item', ok: true })]);
    const ixs = k.sender.submitted[0]!.instructions;
    expect(shape(ixs.slice(0, 1))).toEqual(shape([hookwars.createItem(AGENT.publicKey, 42, [3, 9, ...Array(hookwars.PARAM_FIELDS - 2).fill(0)], 250, 7n, k.vault)]));
    const m = memoOf(ixs);
    expect(shape(ixs.slice(1))).toEqual(shape([memoInstruction(m, [AGENT.publicKey])]));
    expect(m).toMatchObject({ kind: 'status', from: k.passport.toBase58(), to: '*' });
    const body = toJson(m.body) as { act: string; pv: { p: string; m: string; ph: string; oh: string } };
    expect(body.act).toBe('create_item');
    expect(memoMatches(body.pv, r.provenance!)).toBe(true);
    expect(verifyProvenance(r.provenance!, k.model.calls[0]!, JSON.stringify({ actions: [{ type: 'create_item', templateId: 42, params: [3, 9], royaltyBps: 250 }] }))).toBe(true);
  });

  it('lists a vault item through spend: the vault signs inside the program, not at the top level', async () => {
    const k = kit(replay([{ actions: [{ type: 'list_item', itemMint: ITEM_MINT.toBase58(), priceLamports: '1500000000', expiresAt: '0' }] }]));
    k.chain.tokens.set(`${ITEM_MINT.toBase58()}:${k.vault.toBase58()}`, 1n);
    await tick(k.rt);
    const ixs = k.sender.submitted[0]!.instructions;
    const inner = hookwars.marketList(k.vault, hookwars.itemAddress(ITEM_MINT), ITEM_MINT, 1_500_000_000n, 0n);
    expect(shape(ixs.slice(0, 1))).toEqual(shape([wrapSpend(inner, AGENT.publicKey, k.passport, k.vault)]));
    const spend = ixs[0]!;
    expect(spend.programId.equals(hookwars.AGENTS_ID)).toBe(true);
    expect(spend.keys.filter((x) => x.isSigner).map((x) => x.pubkey.toBase58())).toEqual([AGENT.publicKey.toBase58()]);
    expect(spend.keys[5]!.pubkey.equals(hookwars.MARKET_ID)).toBe(true);
    expect(spend.keys.slice(8).map((x) => x.pubkey.toBase58())).toEqual(inner.keys.map((x) => x.pubkey.toBase58()));
  });

  it('refuses to list an item the vault does not hold, or above the configured price', async () => {
    const k = kit(replay([{ actions: [
      { type: 'list_item', itemMint: ITEM_MINT.toBase58(), priceLamports: '1', expiresAt: '0' },
      { type: 'list_item', itemMint: ITEM_MINT.toBase58(), priceLamports: '5000000001', expiresAt: '0' },
    ] }]));
    const r = await tick(k.rt);
    expect(r.results[0]!.refusals).toEqual(['the agent vault does not hold this item']);
    expect(r.results[1]!.refusals!.join()).toMatch(/over the configured/);
    expect(k.sender.submitted).toHaveLength(0);
  });

  it('negotiates over memos: offer to a passport, accept by reference, commit binds the message', async () => {
    const offerBody = { item: ITEM_MINT.toBase58(), side: 'sell', price: 900_000_000, qty: 1 };
    const k = kit(replay([{ actions: [
      { type: 'message', kind: 'offer', to: OTHER.toBase58(), thread: '', re: '', body: offerBody, expiresAt: '1800003600' },
      { type: 'message', kind: 'accept', to: OTHER.toBase58(), thread: `${SIG}:0`, re: `${SIG}:1`, body: { ref: `${SIG}:1` } },
      { type: 'commit', messageId: `${SIG}:1`, hash: 'ab'.repeat(32) },
    ] }]));
    await tick(k.rt);
    const [offer, accept, commit] = k.sender.submitted.map((s) => s.instructions);
    const om = memoOf(offer!);
    expect(om).toMatchObject({ kind: 'offer', to: OTHER.toBase58(), thread: '', expiresAt: 1_800_003_600n });
    expect(toJson(om.body)).toMatchObject(offerBody);
    expect(memoOf(accept!)).toMatchObject({ kind: 'accept', thread: `${SIG}:0`, re: `${SIG}:1` });
    expect(shape(commit!.slice(0, 1))).toEqual(shape([commitInstruction(AGENT.publicKey, k.passport, messageRef(`${SIG}:1`), Buffer.from('ab'.repeat(32), 'hex'))]));
  });

  it('refuses an accept without a reference and a message with banned words or em dashes', async () => {
    const k = kit(replay([{ actions: [
      { type: 'message', kind: 'accept', to: '*', body: {} },
      { type: 'message', kind: 'offer', to: '*', body: { note: 'great yield here' } },
      { type: 'message', kind: 'offer', to: '*', body: { note: 'fine \u2014 deal' } },
      { type: 'message', kind: 'offer', to: '*', body: { note: `key ${'4'.repeat(88)}` } },
    ] }]));
    const r = await tick(k.rt);
    expect(r.results.map((x) => x.ok)).toEqual([false, false, false, false]);
    expect(r.results[3]!.refusals![0]).toMatch(/secret key/);
    expect(r.results[1]!.refusals![0]).toMatch(/yield/);
    expect(r.results[2]!.refusals![0]).toMatch(/em dash/);
  });

  it('cranks a permissionless route through the prepare router, signed by the agent key only', async () => {
    const k = kit(replay([{ actions: [{ type: 'crank', route: 'settle/prepare', body: { cranker: AGENT.publicKey.toBase58(), mint: TOKEN_MINT.toBase58(), slot: 0 } }, { type: 'crank', route: 'raid/prepare', body: {} }] }]));
    const settle = new TransactionInstruction({ programId: hookwars.ITEMS_ID, keys: [{ pubkey: AGENT.publicKey, isSigner: true, isWritable: true }, { pubkey: TOKEN_MINT, isSigner: false, isWritable: true }], data: Buffer.from([1, 2, 3]) });
    k.router.builders.set('settle/prepare', () => [settle]);
    const r = await tick(k.rt);
    expect(r.results[0]!.ok).toBe(true);
    expect(r.results[1]!.refusals![0]).toMatch(/not in cranks.routes/);
    expect(shape(k.sender.submitted[0]!.instructions.slice(0, 1))).toEqual(shape([settle]));
    expect(k.router.prepared).toEqual([{ route: 'settle/prepare', body: { cranker: AGENT.publicKey.toBase58(), mint: TOKEN_MINT.toBase58(), slot: 0 } }]);
  });

  it('refuses a crank whose prepared transaction needs another signer', async () => {
    const k = kit(replay([{ actions: [{ type: 'crank', route: 'settle/prepare', body: {} }] }]));
    k.router.builders.set('settle/prepare', () => [new TransactionInstruction({ programId: hookwars.ITEMS_ID, keys: [{ pubkey: OTHER, isSigner: true, isWritable: true }], data: Buffer.alloc(0) })]);
    expect((await tick(k.rt)).results[0]!.refusals![0]).toMatch(/signer other than the agent/);
  });

  it('buys through the vault: holding paid by the agent key, swap wrapped in spend, a public reason in the memo', async () => {
    const reason = 'royalty settles doubled on this token today';
    const k = kit(replay([{ actions: [{ type: 'trade', side: 'buy', mint: TOKEN_MINT.toBase58(), amountIn: '100000000', minOut: '995000', reason }] }]), {}, '{"v":1}');
    k.chain.mints.set(TOKEN_MINT.toBase58(), { creator: OTHER.toBase58(), itemAuthors: [] });
    k.router.quotes.set(TOKEN_MINT.toBase58(), 1_000_000n);
    const swap = new TransactionInstruction({ programId: hookwars.SWAP_ID, keys: [{ pubkey: k.vault, isSigner: true, isWritable: true }, { pubkey: TOKEN_MINT, isSigner: false, isWritable: true }], data: Buffer.from([9]) });
    k.router.builders.set('buy/prepare', (b) => [token.createHolding(new PublicKey(b.owner as string), TOKEN_MINT, new PublicKey(b.owner as string)), swap]);
    k.rt.sender = new SentSender();
    k.chain.tokens.set(`${TOKEN_MINT.toBase58()}:${k.vault.toBase58()}`, 0n);
    const r = await tick(k.rt);
    expect(r.results[0]).toMatchObject({ ok: true, sent: true });
    expect(k.router.prepared[0]).toEqual({ route: 'buy/prepare', body: { owner: k.vault.toBase58(), mint: TOKEN_MINT.toBase58(), amount: '100000000', minOut: '995000' } });
    const ixs = (k.rt.sender as SentSender).submitted[0]!.instructions;
    expect(shape(ixs.slice(0, 2))).toEqual(shape([token.createHolding(AGENT.publicKey, TOKEN_MINT, k.vault), wrapSpend(swap, AGENT.publicKey, k.passport, k.vault)]));
    expect((toJson(memoOf(ixs).body) as { reason: string }).reason).toBe(reason);
    expect(k.store.current.book!.positions[TOKEN_MINT.toBase58()]).toMatchObject({ costLamports: 100_000_000n, lastBuyAt: k.chain.time });
  });

  it('sells through the vault (R-1): sell/prepare for the vault, quoted in lamports, the position closed by lamports received', async () => {
    const k = kit(replay([{ actions: [{ type: 'trade', side: 'sell', mint: TOKEN_MINT.toBase58(), amountIn: '1000000', minOut: '99500000', reason: 'taking the position down' }] }]), {}, '{"v":1}');
    k.chain.mints.set(TOKEN_MINT.toBase58(), { creator: OTHER.toBase58(), itemAuthors: [] });
    k.router.sellQuotes.set(TOKEN_MINT.toBase58(), 100_000_000n);
    const t0 = k.chain.time;
    k.store.current.book = { ...emptyBook(t0 - 7_200, 0n), positions: { [TOKEN_MINT.toBase58()]: { qty: 1_000_000n, costLamports: 90_000_000n, lastBuyAt: t0 - 7_200 } } };
    const swap = new TransactionInstruction({ programId: hookwars.SWAP_ID, keys: [{ pubkey: k.vault, isSigner: true, isWritable: true }, { pubkey: TOKEN_MINT, isSigner: false, isWritable: true }], data: Buffer.from([9]) });
    k.router.builders.set('sell/prepare', () => [swap]);
    const sender = new SentSender();
    k.rt.sender = sender;
    sender.onSubmit = () => { k.chain.balances.set(k.vault.toBase58(), (k.chain.balances.get(k.vault.toBase58()) ?? 0n) + 100_000_000n); };
    const r = await tick(k.rt);
    expect(r.results[0]).toMatchObject({ ok: true, sent: true, label: 'sell' });
    expect(k.router.prepared[0]).toEqual({ route: 'sell/prepare', body: { owner: k.vault.toBase58(), mint: TOKEN_MINT.toBase58(), amount: '1000000', minOut: '99500000' } });
    expect(shape(sender.submitted[0]!.instructions.slice(0, 1))).toEqual(shape([wrapSpend(swap, AGENT.publicKey, k.passport, k.vault)]));
    expect(k.store.current.book!.positions[TOKEN_MINT.toBase58()]).toBeUndefined();
    expect(k.store.current.book!.realizedTodayLamports).toBe(10_000_000n);
  });

  it('the cranker may call the war steps the API prepares now (R-2)', () => {
    for (const r of ['war/siege/prepare', 'war/counter-strike/prepare', 'war/raze/prepare', 'book/crank/prepare']) expect(CRANK_ROUTES).toContain(r);
  });

  it('set_access is checked against the directive, then built for the armory with the agent suffix (R-4, spec 14)', async () => {
    const act = { type: 'set_access', itemMint: ITEM_MINT.toBase58(), mode: 1, licencePrice: '5' };
    const k = kit(replay([{ actions: [act] }]), { roles: ['market'] });
    k.chain.tokens.set(`${ITEM_MINT.toBase58()}:${k.vault.toBase58()}`, 1n);
    k.chain.itemTemplates.set(ITEM_MINT.toBase58(), 7);
    const r = await tick(k.rt);
    expect(r.results[0]!.refusals!.join()).toMatch(/access mode 1 is not allowed/);
    const k2 = kit(replay([{ actions: [act] }]), { roles: ['market'] });
    k2.chain.postDirective(k2.passport, 0, { ...OPEN_CONSTRAINTS(), allowedAccessModes: 1 << 1, maxLicencePrice: 10n }, 'https://rules.example/scout/0.json', '{"v":1}');
    k2.chain.tokens.set(`${ITEM_MINT.toBase58()}:${k2.vault.toBase58()}`, 1n);
    k2.chain.itemTemplates.set(ITEM_MINT.toBase58(), 7);
    const r2 = await tick(k2.rt);
    expect(r2.results[0]).toMatchObject({ ok: true });
    const ix = k2.sender.submitted[0]!.instructions[0]!;
    expect(ix.programId.equals(hookwars.ARMORY_ID)).toBe(true);
    const tail = ix.keys.slice(-3).map((m) => m.pubkey.toBase58());
    expect(tail).toEqual([hookwars.AGENTS_ID.toBase58(), k2.passport.toBase58(), hookwars.directiveAddress(k2.passport, 0).toBase58()]);
  });

  it('refuses trades on its own or its operator tokens before asking the router to prepare', async () => {
    const k = kit(replay([{ actions: [{ type: 'trade', side: 'buy', mint: TOKEN_MINT.toBase58(), amountIn: '100', minOut: '995', reason: 'r' }] }]));
    k.chain.mints.set(TOKEN_MINT.toBase58(), { creator: k.vault.toBase58(), itemAuthors: [AGENT.publicKey.toBase58()] });
    k.router.quotes.set(TOKEN_MINT.toBase58(), 1_000n);
    const r = await tick(k.rt);
    expect(r.results[0]!.refusals!.join()).toMatch(/SameOperator.*OwnItemToken/);
    expect(k.router.prepared).toHaveLength(0);
  });

  it('refuses a buy over the on-chain policy limit even inside the engine caps', async () => {
    const k = kit(replay([{ actions: [{ type: 'trade', side: 'buy', mint: TOKEN_MINT.toBase58(), amountIn: '400000000', minOut: '3960000', reason: 'r' }] }]));
    k.chain.policies.get(k.passport.toBase58())!.perActionLamports = 300_000_000n;
    k.chain.mints.set(TOKEN_MINT.toBase58(), { creator: OTHER.toBase58(), itemAuthors: [] });
    k.router.quotes.set(TOKEN_MINT.toBase58(), 4_000_000n);
    expect((await tick(k.rt)).results[0]!.refusals!.join()).toMatch(/policy per-action limit/);
  });

  it('writes facts-only status memos after real events, in the agent voice, and never twice', async () => {
    const k = kit(replay([
      { actions: [{ type: 'status', voice: 'quiet day' }] },
      { actions: [{ type: 'status', voice: 'settled' }] },
      { actions: [{ type: 'status', voice: 'up 3x today' }] },
    ]));
    k.rt.sender = new SentSender();
    expect((await tick(k.rt)).results[0]!.refusals![0]).toMatch(/real events only/);
    k.chain.facts.push({ signature: 'S1', slot: 1, name: 'items.RoyaltySettled', text: 'RoyaltySettled: amount 120, slot 0' });
    const r = await tick(k.rt);
    expect(r.results[0]).toMatchObject({ ok: true, sent: true });
    const m = memoOf((k.rt.sender as SentSender).submitted[0]!.instructions);
    expect(toJson(m.body)).toMatchObject({ text: 'RoyaltySettled: amount 120, slot 0', voice: 'settled' });
    expect(k.store.current.pendingFacts).toEqual([]);
    k.chain.facts.push({ signature: 'S2', slot: 2, name: 'items.RoyaltySettled', text: 'RoyaltySettled: amount 7, slot 0' });
    expect((await tick(k.rt)).results[0]!.refusals![0]).toMatch(/no digits|may not contain digits/);
  });

  it('rate-limits memos per hour', async () => {
    const msg = { type: 'message', kind: 'offer', to: '*', body: { n: 1 } };
    const k = kit(replay([{ actions: [msg, msg, msg, msg] }]));
    k.rt.sender = new SentSender();
    const r = await tick(k.rt);
    expect(r.results.map((x) => x.ok)).toEqual([true, true, true, false]);
    expect(r.results[3]!.refusals![0]).toMatch(/rate limit/);
  });

  it('drops malformed proposals and caps actions per tick', async () => {
    const k = kit(replay(['not json at all']));
    expect((await tick(k.rt)).dropped[0]!.why).toMatch(/not one JSON object/);
    const many = Array.from({ length: 6 }, () => ({ type: 'status', voice: 'x' }));
    const k2 = kit(replay([{ actions: [{ type: 'rm -rf' }, ...many] }]));
    const r = await tick(k2.rt);
    expect(r.dropped.map((d) => d.why)).toEqual(['unknown action type "rm -rf"', 'more than 4 actions in one tick', 'more than 4 actions in one tick']);
  });

  it('halts when the passport names another agent key or is not active', async () => {
    const k = kit(replay([]));
    k.chain.passports.get(k.passport.toBase58())!.agentKey = OTHER;
    expect((await tick(k.rt)).halted).toMatch(/another agent key/);
    const k2 = kit(replay([]));
    k2.chain.passports.get(k2.passport.toBase58())!.status = 1;
    expect((await tick(k2.rt)).halted).toMatch(/not active/);
  });

  it('adds postage when configured and gives the model no secrets', async () => {
    const k = kit(replay([{ actions: [{ type: 'message', kind: 'offer', to: '*', body: { n: 1 } }] }]), { memo: { maxPerHour: 3, maxPerDay: 10, postage: true } });
    await tick(k.rt);
    const ixs = k.sender.submitted[0]!.instructions;
    expect(ixs.at(-1)!.programId.equals(hookwars.AGENTS_ID)).toBe(true);
    const prompt = JSON.stringify(k.model.calls[0]);
    expect(prompt).not.toContain(Buffer.from(AGENT.secretKey).toString('hex'));
    expect(prompt).toContain(k.vault.toBase58());
  });

  it('records the same provenance for the same prompt and output', async () => {
    const k = kit(replay([{ actions: [] }]));
    const r = await tick(k.rt);
    expect(r.provenance).toEqual(provenanceOf(k.model.calls[0]!, { provider: 'stub', model: 'stub-1', text: '{"actions":[]}' }));
  });
});
