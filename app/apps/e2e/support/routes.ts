// Every page of app/apps/web/app, with demo-mode values for the dynamic segments (lib/mock.ts,
// lib/mock-social.ts and lib/explorer-demo.json). tests/routes.spec.ts checks that this list
// matches the app's page files, so a new page cannot be added without a check here.
export const MINT = 'Ash7k2pQmV4rJx9nLbT3wYcE6sHdF8gRaN5uKiZoP1Wm';
export const ITEM = 'ItmA1bC2dE3fG4hJ5kL6mN7pQ8rS9tU1vW2xY3zA4bC5d';
export const ITEM_MINT = 'ImtA1bC2dE3fG4hJ5kL6mN7pQ8rS9tU1vW2xY3zA4bC5d';
export const TX = '5d5bPuhmTKFRmYygzbkpZUv1QotFq4qtJxhXPEDP26m9rh4xXbXzVcds1EHVpv6xJrtfuPtGhryAcjtUxuSsYpfH';
export const PROGRAM = '5yeVq5rEWWBRkBWiA49So9u4jpxeZQTeYsjQcFRwX618';
export const ADDRESS = '9dnhz8Ez1QGbrCFiWwgR9haFpcyeN1mbojUTm3xRMFMY';
export const WALLET_ADDR = 'RdrA2bC4dE6fG8hJ1kL3mN5pQ7rS9tU2vW4xY6zA8bC1';

/** [page file route, url to open] */
export const ROUTES: [string, string][] = [
  ['/', '/'],
  ['/address/[key]', `/address/${ADDRESS}`],
  ['/agents', '/agents'],
  ['/agents/[passport]', `/agents/${WALLET_ADDR}`],
  ['/agents/[passport]/timeline', `/agents/${WALLET_ADDR}/timeline`],
  ['/armory', '/armory'],
  ['/armory/items/[item]', `/armory/items/${ITEM}`],
  ['/armory/templates', '/armory/templates'],
  ['/armory/templates/[id]', '/armory/templates/1'],
  ['/armory/templates/submissions', '/armory/templates/submissions'],
  ['/badges', '/badges'],
  ['/book', '/book'],
  ['/bridge', '/bridge'],
  ['/coins', '/coins'],
  ['/commissions', '/commissions'],
  ['/commissions/[address]', `/commissions/${ADDRESS}`],
  ['/craft', '/craft'],
  ['/docs', '/docs'],
  ['/docs/[...slug]', '/docs/concepts/how-a-trade-runs'],
  ['/docs/protocol', '/docs/protocol'],
  ['/explorer', '/explorer'],
  ['/feed', '/feed'],
  ['/feed/hides', '/feed/hides'],
  ['/feed/thread/[id]', '/feed/thread/1'],
  ['/generals', '/generals'],
  ['/economy', '/economy'],
  ['/gate', '/gate'],
  ['/governance', '/governance'],
  ['/guilds', '/guilds'],
  ['/guilds/[id]', '/guilds/1'],
  ['/launch', '/launch'],
  ['/leaderboards', '/leaderboards'],
  ['/live', '/live'],
  ['/marketplace', '/marketplace'],
  ['/marketplace/items/[itemMint]', `/marketplace/items/${ITEM_MINT}`],
  ['/marketplace/rentals', '/marketplace/rentals'],
  ['/portfolio', '/portfolio'],
  ['/program/[id]', `/program/${PROGRAM}`],
  ['/projects', '/projects'],
  ['/quests', '/quests'],
  ['/seasons', '/seasons'],
  ['/t/[mint]', `/t/${MINT}`],
  ['/t/[mint]/generals', `/t/${MINT}/generals`],
  ['/tx/[sig]', `/tx/${TX}`],
  ['/u/[wallet]', `/u/${WALLET_ADDR}`],
  ['/war', '/war'],
  ['/war/coalitions', '/war/coalitions'],
];
