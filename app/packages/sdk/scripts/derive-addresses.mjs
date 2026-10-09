// Derives every fixed address of the Hookwars programs from the program ids, the way the
// programs do (find_program_address). Prints JSON; packages/shared/src/programs.ts holds the result
// and packages/sdk tests derive each again.
import { PublicKey } from '@solana/web3.js';

const ids = {
  token: '5yeVq5rEWWBRkBWiA49So9u4jpxeZQTeYsjQcFRwX618',
  swap: 'AhmowBwJF7E1uDQ3quQz8xMKevre3i8kYPBEbkhAedvo',
  bridge: '5TzyKXK6tzSrkkRdximMebWoV4rRjuyzEwCnisS6DKwj',
  launch: 'fBvY7neytvwSuJLF1Sur5tHk7vkWyPzjyfDVDm1m2qD',
  taxHook: 'q9mMtM6vfJ8YMffnkUNW5XLz7xeVyyeo1HL8SA27AuX',
  kit: 'CLEEZe3v8Sqa45J1VdmfKkjxqFGSj44MxA5prQH3xTLG',
  halfLife: '67nAgW7h9wYmM1jLzrXVNy8UDNxgVFqGTqrTEPamtokh',
  companion: 'HzeAN8e7HbGx8wzgd5SpduF51c7rXCTqkQKcmw44YHkK',
  armory: '7xnkyX5B7Ywz86Cu2ofM5T2z2UPmy1qhpgrAuK9Kk5MU',
  items: '8wMqHBAWhKxw2fNpczHfMohKjowbUM4hGqQkoYPf93Gv',
  war: '5vJnBvr33jpsfYxMY2pvNf6tF9tkj8eaZ6goFtByUWA2',
};
const P = Object.fromEntries(Object.entries(ids).map(([k, v]) => [k, new PublicKey(v)]));
const NATIVE = new PublicKey('So11111111111111111111111111111111111111112');
const s = (x) => Buffer.from(x);
const pda = (seeds, prog) => {
  const [a, b] = PublicKey.findProgramAddressSync(seeds, prog);
  return { address: a.toBase58(), bump: b };
};
const out = {
  hookSigners: {
    tokenForKit: pda([s('hook-authority'), P.kit.toBuffer()], P.token),
    tokenForTaxHook: pda([s('hook-authority'), P.taxHook.toBuffer()], P.token),
    tokenForHalfLife: pda([s('hook-authority'), P.halfLife.toBuffer()], P.token),
    tokenForItems: pda([s('hook-authority'), P.items.toBuffer()], P.token),
    dexForLaunch: pda([s('hook-authority'), P.launch.toBuffer()], P.swap),
    launchForItems: pda([s('hook-authority'), P.items.toBuffer()], P.launch),
  },
  fixed: {
    tokenEventAuthority: pda([s('__event_authority')], P.token),
    swapEventAuthority: pda([s('__event_authority')], P.swap),
    swapConfig: pda([s('config')], P.swap),
    bridgeEventAuthority: pda([s('__event_authority')], P.bridge),
    bridgeConfig: pda([s('config')], P.bridge),
    solWrapper: pda([s('wrapper'), NATIVE.toBuffer()], P.bridge),
    solVault: pda([s('sol-vault')], P.bridge),
    bridgedSolMint: pda([s('wrapped'), NATIVE.toBuffer()], P.bridge),
    launchEventAuthority: pda([s('__event_authority')], P.launch),
    launchHookAuthority: pda([s('hook-authority')], P.launch),
    launchConfig: pda([s('config')], P.launch),
    kitEventAuthority: pda([s('__event_authority')], P.kit),
    kitHookAuthority: pda([s('hook-authority')], P.kit),
    companionEventAuthority: pda([s('__event_authority')], P.companion),
    armoryEventAuthority: pda([s('__event_authority')], P.armory),
    armorySigner: pda([s('armory')], P.armory),
    itemsEventAuthority: pda([s('__event_authority')], P.items),
    warEventAuthority: pda([s('__event_authority')], P.war),
    warConfig: pda([s('war-config')], P.war),
    warSigner: pda([s('war-signer')], P.war),
    lootSigner: pda([s('loot-signer')], P.war),
    prizeVault: pda([s('prize-vault')], P.war),
  },
};
console.log(JSON.stringify(out, null, 2));
