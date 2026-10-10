// Builds public/plnty/units.html, the landing page: the archived plnty home (public/plnty/index.html)
// with its layout, motion and art kept and every piece of copy replaced by units content. Their
// customers' logos and testimonials are removed, not reworded: they are not ours to show.
// Tags in the archive carry scoping attributes (data-astro-cid-*), so patterns allow [^>]* inside
// tags and captured attributes ($A) are put back on replaced markup.
// Run: node scripts/build-landing.mjs
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const dir = join(dirname(fileURLToPath(import.meta.url)), '..', 'public', 'plnty');
let h = readFileSync(join(dir, 'index.html'), 'utf8');
let misses = 0;
const re = (pattern, to) => { const before = h; h = h.replace(pattern, to); if (h === before) { console.error('no match:', String(pattern).slice(0, 80)); misses++; } };
const all = (from, to) => { if (!h.includes(from)) { console.error('not found:', from.slice(0, 70)); misses++; return; } h = h.split(from).join(to); };

// Head
re(/<title>[^<]*<\/title>/, '<title>units · Tokens whose hooks are owned items</title>');
re(/<meta name="description" content="[^"]*"/, '<meta name="description" content="units is a token platform on Solana where a token&#39;s rules are hooks you can own, trade and swap, and where tokens can act on each other. Spot only."');

// Hero
re(/The agentic creative canvas<br([^>]*)> for professional creators\./, 'Tokens whose hooks<br$1> are owned items.');
all('Plnty is a powerful suite of design tools that turns ideas into finished work across image, video, vector, 3D and audio, all inside one shared canvas.', 'Every trade, transfer and swap of a units token runs through the hooks in its slots. Each hook is an item with an owner who earns every time it runs. Holders vote what fills each slot, aim raids at rivals and fund a war chest from their own fees.');
re(/<button type="button" class="cta cta-request js-subscribe hp-squircle"([^>]*)>(<span[^>]*>)Request Access(<\/span>.*?)<\/button>/s, '<a class="cta cta-request hp-squircle" href="/launch" target="_top"$1>$2Launch a token$3</a>');
re(/<a class="cta cta-demo hp-squircle" href="[^"]*"[^>]*?( data-astro-cid-[a-z0-9]+)?>Book a Demo<\/a>/, '<a class="cta cta-demo hp-squircle" href="/projects" target="_top"$1>Explore tokens</a>');
// Their client logos: every logo item removed, the empty band hidden.
re(/<li class="logo"[^>]*>.*?<\/li>/gs, '');
// Social and search metadata, the archive's own header (hidden under ours), the band.
re(/<meta (?:property="og:|name="twitter:)[^>]*>/g, '');
re(/<script type="application\/ld\+json">.*?<\/script>/gs, '');
re(/<link rel="canonical"[^>]*>/, '');
re(/<\/head>/, '<style id="units-frame">:root{--hp-ticker-height:0px!important;--hp-bar-height:0px!important}.ticker,.header,.band{display:none!important}</style></head>');

// Canvas section
re(/(<section id="canvas" class="canvas" role="img" aria-label=")[^"]*"/, '$1A units board mid-session: tokens on the map, items moving between slots, raids drawn as arrows between rivals."');
all('Join the Waitlist!', 'Launch a token');

// Ecosystem
all('Behind the canvas', 'Behind every token');
all('class="eco-root" aria-label="plnty.app"', 'class="eco-root" aria-label="units"');
all('150+ tools. 50+ models across image, video, 3D, vector and sound. One spatial system designed for creative work that branches, rather than follows a linear pipeline.', 'Slots fixed at launch, items that fill them, a war chest per token and an armory of audited templates. One standard where tokens read and act on each other, rather than trading alone.');
for (const [a, b] of [['Tools', 'Slots'], ['AI Models', 'Items'], ['Local Compute', 'War chest'], ['Prompt Refiner', 'Armory'], ['GPU Cluster', 'Indexer']]) {
  const before = h; h = h.replace(new RegExp(`(class="eco-pill"[^>]*>)${a}<`, 'g'), `$1${b}<`); if (h === before) { console.error('no pill:', a); misses++; }
}

// Testimonials: the protocol's three ideas in place of quotes from real people
const QUOTES = [
  { text: 'A units token has slots. Each slot holds one hook, and its limits, how much it may take, whether it may refuse and who may change it, are fixed at launch and never change. What sits inside can change, by holder vote or by an on-chain performance rule.', name: 'Tokens with slots', role: 'Fixed at launch, filled by vote', avatar: '/agency/coins/001.webp', stats: [{ value: '4', label: 'Slots per token, at most' }, { value: '64', label: 'Bytes of holder data' }] },
  { text: 'A hook is an item: a supply-one token that wraps an audited template with its own parameters. Items have an owner, a level and a royalty. Every time an item runs on any token, its owner earns a share of what it collects. Items can be listed, rented, forged and combined.', name: 'Hooks are assets', role: 'Owned, traded and forged', avatar: '/agency/coins/004.webp', stats: [{ value: '1', label: 'Supply of every item' }, { value: '2 to 1', label: 'Items burned per forge' }] },
  { text: 'Tokens act on each other. A Raid sells a rival to buy this token in one swap. A siege spends from the war chest when raids pass the threshold the holders set. A treaty is one item both tokens equip, sharing fees both ways. Spot only: every action is a transfer, a cut, a burn, a fee or a swap.', name: 'Tokens that interact', role: 'Raids, sieges and treaties', avatar: '/agency/coins/002.webp', stats: [{ value: '1', label: 'Swap per raid' }, { value: '0', label: 'Leverage, ever' }] },
];
re(/const QUOTES = \[.*?\];/s, `const QUOTES = ${JSON.stringify(QUOTES)};`);
re(/It doesn&#39;t feel like simple image generation[^<]*/, QUOTES[0].text);
all('Owen Schmidt', QUOTES[0].name);
all('3D Artist, United States', QUOTES[0].role);
all('/homepage/showcase/avatar-owen.webp', QUOTES[0].avatar);
re(/(<span class="stat-value[^"]*"[^>]*>)126h</, `$1${QUOTES[0].stats[0].value}<`);
re(/(<span class="stat-label[^"]*"[^>]*>)Time in Plnty</, `$1${QUOTES[0].stats[0].label}<`);
re(/(<span class="stat-value[^"]*"[^>]*>)329</, `$1${QUOTES[0].stats[1].value}<`);
re(/(<span class="stat-label[^"]*"[^>]*>)Sessions</, `$1${QUOTES[0].stats[1].label}<`);

// Claim fold: claim a ticker and go to Launch
all('Join early &amp; claim', 'Launch early &amp; claim');
re(/<form class="cta" action="[^"]*" method="get"/, '<form class="cta" action="/launch" method="get" target="_top"');
all('Claim your handle', 'Claim your ticker');
re(/name="handle" type="text"([^>]*?)maxlength="20" pattern="[^"]*" placeholder="yourname"/, 'name="symbol" type="text"$1maxlength="10" pattern="[A-Za-z0-9]{1,10}" placeholder="TICKER"');
re(/<input type="hidden" name="from" value="waitlist-fold"[^>]*>/, '');
all('Claim this handle', 'Claim this ticker');
all('Limited access opened to new users on a monthly basis.', 'units is built and tested in the program suite, on its way to devnet.');
re(/Join the waitlist or <a class="discord" href="[^"]*" rel="noopener"([^>]*)>visit the Discord<\/a> for further details\./, 'Launch a token or <a class="discord" href="/docs" target="_top"$1>read how units works</a> first.');

// Footer
re(/Create as Fast as<br([^>]*)>You Can Think\./, 'Own the hooks.<br$1>Arm the token.');
re(/<button class="pill pill-light js-subscribe" type="button" data-from="footer"([^>]*)>Request Access<\/button>/, '<a class="pill pill-light" href="/launch" target="_top"$1>Launch a token</a>');
re(/<a class="pill pill-dark" href="https:\/\/plnty\.app\/login"([^>]*)>(<svg.*?<\/svg>)Log In<\/a>/s, '<a class="pill pill-dark" href="/projects" target="_top"$1>$2Explore</a>');
re(/<span class="addr"([^>]*)>10 Luria St,<br([^>]*)>Tel-Aviv, 6314210, Israel<\/span><a class="addr-link"[^>]*>support@plnty\.app<\/a>/, '<span class="addr"$1>Solana. Spot only.<br$2>The Bordrless token standard.</span>');
all('2026 &copy; Plnty Labs Ltd', '2026 units');
const col = (title, links) => (_m, a) => `<h3 class="col-title"${a}>${title}</h3><ul class="col-list"${a}>${links.map(([href, l]) => `<li${a}><a class="col-link" href="${href}" target="_top"${a}>${l}</a></li>`).join('')}</ul>`;
re(/<h3 class="col-title"([^>]*)>Platform<\/h3><ul class="col-list"[^>]*>.*?<\/ul>/s, col('Platform', [['/projects', 'Projects'], ['/launch', 'Launch'], ['/war', 'War room']]));
re(/<h3 class="col-title"([^>]*)>Company<\/h3><ul class="col-list"[^>]*>.*?<\/ul>/s, col('Items', [['/armory', 'Armory'], ['/marketplace', 'Marketplace'], ['/craft', 'Craft']]));
re(/<h3 class="col-title"([^>]*)>Resources<\/h3><ul class="col-list"[^>]*>.*?<\/ul>/s, col('Resources', [['/docs', 'Documentation'], ['/explorer', 'Explorer'], ['/agents', 'Agents'], ['/feed', 'Feed'], ['/seasons', 'Seasons']]));
all('aria-label="Company"', 'aria-label="Items"');
re(/<ul class="legal"([^>]*)>.*?<\/ul>/s, (_m, a) => `<ul class="legal"${a}>${[['/docs', 'How units works'], ['/docs/protocol', 'Protocol'], ['/explorer', 'Explorer']].map(([href, l]) => `<li${a}><a href="${href}" target="_top"${a}>${l}</a></li>`).join('')}</ul>`);

// The ecosystem map script holds its own copy: a units copy of the module with our hubs and root.
const mapSrc = readFileSync(join(dir, '_astro', 'EcosystemMap.CYGJ9AP0.js'), 'utf8');
let map = mapSrc;
const ms = (from, to) => { if (!map.includes(from)) { console.error('map: not found:', from.slice(0, 60)); misses++; } map = map.split(from).join(to); };
ms('title:`plnty.app`,subtitle:`Where creation just works`', 'title:`units`,subtitle:`Tokens whose hooks are owned items`');
ms('150+ tools. 50+ models across image, video, 3D, vector and sound. One spatial system designed for creative work that branches, rather than follows a linear pipeline.', 'Slots fixed at launch, items that fill them, a war chest per token and an armory of audited templates. One standard where tokens read and act on each other.');
ms('title:`Tools`,subtitle:`${o.tools.filter(e=>e.status===`live`).length} live`,kind:`hub`,description:`Where the craft lives. A tool is a card on the canvas that does one job, with the model, the settings and the cleanup already decided so you are choosing an outcome rather than assembling a pipeline. Swap the engine underneath and the card stays the same.', 'title:`Slots`,subtitle:`Up to 4 per token`,kind:`hub`,description:`Each slot holds one hook. Its kind, bounds, equip rule and notice are fixed at launch and never change. Only the item inside changes, by holder vote or by an on-chain performance rule.');
ms('title:`AI Models`,subtitle:`${r} models`,kind:`hub`,description:`The models Plnty can reach, grouped by what they produce. None of them is the product. Each one is an ingredient a tool decides how to use, which is why a model can be swapped underneath a tool without the tool changing shape.', 'title:`Items`,subtitle:`Owned hooks`,kind:`hub`,description:`An item is a supply-one token that wraps an audited template with its own parameters. It has an owner, a level and a royalty, and every time it runs on any token its owner earns a share of what it collects.');
ms('title:`GPU Cluster`,subtitle:`Inference we host ourselves`,kind:`hub`,description:`Some work cannot wait on somebody else queue. For the interactive surfaces, where a frame has to come back while you are still moving the camera, Plnty rents a GPU outright and serves the model directly. It is the difference between asking for a result and watching one.', 'title:`War chest`,subtitle:`One per token`,kind:`hub`,description:`Funded by fees, items, partners and donations. It spends only through bounded cranks the holders allow in the War orders item: sieges, counter-strikes, bounties. What is not spent stays as reserve.');
ms('title:`Local Compute`,subtitle:`${e.length} operations`,kind:`hub`,description:`Work that never leaves your machine. Meshes, masks, colour and vectors are handled by code running in your browser on your own GPU, which means no upload, no queue, no cost per attempt. It is also why some things feel instant while others take a minute.', 'title:`Armory`,subtitle:`Audited templates`,kind:`hub`,description:`The only code a slot can run, besides the kit. Raid, Shield, Wall, Spy, Treaty, Tribute, Half-Life, Transfer Fee and War orders, each with floors and ceilings that forging moves toward and never past.');
ms('title:`Prompt Refiner`,subtitle:`${t.length} language models`,kind:`hub`,description:`A rough sentence is not a brief. Language models sit between what you type and what the image model receives, filling in what you left implied and naming what you could not. They also read images back, so a picture can become a prompt.', 'title:`Indexer`,subtitle:`Every event, decoded`,kind:`hub`,description:`One signature cursor per program. Raids, sieges, forges, votes and season results land in the feed as the indexer reads them, each linked to its transaction. Figures stay blank rather than guessed.');
writeFileSync(join(dir, '_astro', 'EcosystemMap.units.js'), map);
all('/plnty/_astro/EcosystemMap.CYGJ9AP0.js', '/plnty/_astro/EcosystemMap.units.js');

// The scene note (a quote from their canvas) goes; the drawn collaborator cursor gets a plain label;
// the footer tunnel shows our token art with tickers for credits.
re(/<div class="note note-dog"[^>]*>.*?<\/div><\/div>/s, '');
all('@dor', '@holder');
const coins = Array.from({ length: 34 }, (_, i) => `/agency/coins/${String((i % 50) + 1).padStart(3, '0')}.webp`);
const tickers = ['ASH', 'BRINE', 'CNDR', 'DUSK', 'EMBR', 'FLNT'];
re(/data-sources="[^"]*"/, `data-sources="${coins.join(',')}"`);
re(/data-credits="[^"]*"/, `data-credits="${coins.map((_, i) => '$' + tickers[i % tickers.length]).join(',')}"`);
re(/data-fallback="[^"]*"/, `data-fallback="${coins.slice(0, 4).join(',')}"`);

writeFileSync(join(dir, 'units.html'), h);
console.log(misses ? `units.html written with ${misses} misses` : 'units.html written');
if (misses) process.exitCode = 1;
