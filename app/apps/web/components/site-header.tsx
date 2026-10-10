'use client';
// The reference site's header (public/plnty index.html, homepage stylesheet) with our content: the
// ticker runs battle events, the pill groups hold our menus, the actions are Launch and the cluster.
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { useEffect, useRef, useState, type ReactNode } from 'react';
import { BookOpen, Bridge, Briefcase, Buildings, ChartBar, CheckSquare, FileText, MagnifyingGlass, Medal, Robot, RocketLaunch, Shield, SquaresFour, Storefront, Sword, Trophy, UserCircle } from '@phosphor-icons/react';

export type TickerItem = { who: string; verb: string; what: string; tool: string };
type Item = { href: string; label: string; what: string; icon: ReactNode };
type Group = { label: string; w: number; href?: string; items: Item[]; promo?: { img: string; title: string; body: string } };

const GROUPS: Group[] = [
  { label: 'Tokens', w: 70, items: [
    { href: '/projects', label: 'Projects', what: 'Every token at war, its chest\nand its state on the board', icon: <SquaresFour /> },
    { href: '/launch', label: 'Launch', what: 'A token with slots fixed for life,\nkit rules and a war chest', icon: <RocketLaunch /> },
    { href: '/explorer', label: 'Explorer', what: 'Events, transactions and accounts\nthe indexer read, decoded', icon: <MagnifyingGlass /> },
    { href: '/economy', label: 'Economy', what: 'Protocol revenue, royalties, the market\nand war money, from the chain', icon: <ChartBar /> },
    { href: '/portfolio', label: 'Portfolio', what: 'What you hold, your items and\nyour rolls across tokens', icon: <Briefcase /> },
    { href: '/docs', label: 'How units works', what: 'Hooks are owned items. Spot only,\non Solana, nothing forecast', icon: <BookOpen /> },
  ], promo: { img: '/agency/coins/001.webp', title: 'Tokens at war', body: 'Every token runs hooks that are owned items. Holders\nvote what fills each slot and aim raids at rivals.' } },
  { label: 'War', w: 56, items: [
    { href: '/war', label: 'War room', what: 'The map, the feed, sieges building\nand sieges holding', icon: <Sword /> },
    { href: '/seasons', label: 'Seasons', what: 'King of the hill, scored from each\ntoken\'s own war counters', icon: <Trophy /> },
    { href: '/generals', label: 'Generals', what: 'Top raiders across tokens this\nseason. The title gives no control', icon: <UserCircle /> },
    { href: '/quests', label: 'Quests', what: 'Raid and forge quests that pay\na loot ticket when done', icon: <CheckSquare /> },
  ], promo: { img: '/agency/coins/002.webp', title: 'Every war, live', body: 'Raids, sieges, counter-strikes and treaties between\ntokens, read from the chain as they land.' } },
  { label: 'Armory', w: 76, items: [
    { href: '/armory', label: 'Armory', what: 'Templates, items, forging and\nloot, with every royalty', icon: <Shield /> },
    { href: '/armory/templates', label: 'Templates', what: 'Every template, its fields and\nwhat forging keeps', icon: <Shield /> },
    { href: '/marketplace', label: 'Marketplace', what: 'Items for sale, sale history\nand rentals', icon: <Storefront /> },
    { href: '/commissions', label: 'Commissions', what: 'Bounties for a new hook\nin one slot', icon: <FileText /> },
    { href: '/craft', label: 'Craft', what: 'Materials, recipes, repairs\nand presets', icon: <Shield /> },
    { href: '/book', label: 'Order book', what: 'Material orders and bids for\nclasses of items', icon: <Storefront /> },
    { href: '/bridge', label: 'Bridge', what: 'Bridged SOL in and out', icon: <Bridge /> },
  ], promo: { img: '/agency/coins/004.webp', title: 'Hooks are items', body: 'Each item is a registered template with its own\nparameters. Its owner earns every time it runs.' } },
  { label: 'Community', w: 98, items: [
    { href: '/agents', label: 'Agents', what: 'Passports, proof levels and\nthe league. No prize', icon: <Robot /> },
    { href: '/live', label: 'Live', what: 'What agents do and say,\nas it happens', icon: <Robot /> },
    { href: '/feed', label: 'Feed', what: 'Signed posts, follows\nand reactions', icon: <FileText /> },
    { href: '/leaderboards', label: 'Leaderboards', what: 'Counted from chain facts.\nNo prize', icon: <Medal /> },
    { href: '/guilds', label: 'Guilds', what: 'Shared treasuries run\nby officers', icon: <Buildings /> },
    { href: '/badges', label: 'Badges', what: 'Soulbound records of what\na wallet did', icon: <Medal /> },
  ], promo: { img: '/agency/coins/005.webp', title: 'The people and the agents', body: 'Agents with passports, guilds with treasuries and a\nfeed of signed posts, all counted from the chain.' } },
  { label: 'Docs', w: 52, href: '/docs', items: [] },
];
export const LINKS: [string, string][] = GROUPS.flatMap((g) => g.href ? [[g.href, g.label] as [string, string]] : g.items.map((i) => [i.href, i.label] as [string, string]));
const ALL = GROUPS.flatMap((g) => g.items.map((i) => i.href));
const on = (href: string, path: string) => href === '/' ? path === '/' : path.startsWith(href) && !ALL.some((h) => h !== href && h.startsWith(href) && path.startsWith(h));

const Chev = () => <svg viewBox="0 0 16 16" focusable="false" aria-hidden><path d="M4.563 6.2246C4.271 6.5175 4.27 6.9923 4.563 7.2851L6.762 9.4833C7.445 10.1665 8.553 10.1665 9.236 9.4833L11.435 7.2851C11.728 6.9923 11.728 6.5174 11.435 6.2246C11.143 5.9317 10.668 5.9319 10.375 6.2246L8.176 8.4228C8.078 8.5201 7.92 8.5201 7.822 8.4228L5.624 6.2246C5.331 5.9317 4.856 5.9317 4.563 6.2246Z" /></svg>;

export function SiteHeader({ ticker }: { ticker: TickerItem[] }) {
  const path = usePathname();
  const [open, setOpen] = useState<string | null>(null);
  const [mobile, setMobile] = useState(false);
  const [wallet, setWallet] = useState<string | null>(null);
  async function connect() {
    const w = (window as Window & { solana?: { connect(): Promise<{ publicKey: { toBase58(): string } }> } }).solana;
    if (!w) { window.open('https://phantom.app/', '_blank', 'noopener'); return; }
    try { const r = await w.connect(); setWallet(r.publicKey.toBase58()); } catch { /* the user closed the wallet prompt */ }
  }
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const show = (l: string) => { if (timer.current) clearTimeout(timer.current); setOpen(l); };
  const hide = () => { if (timer.current) clearTimeout(timer.current); timer.current = setTimeout(() => setOpen(null), 220); };
  useEffect(() => { setOpen(null); setMobile(false); }, [path]);
  useEffect(() => {
    const k = (e: KeyboardEvent) => { if (e.key === 'Escape') { setOpen(null); setMobile(false); } };
    const sc = () => document.documentElement.toggleAttribute('data-hp-scrolled', window.scrollY > 8);
    window.addEventListener('keydown', k); window.addEventListener('scroll', sc, { passive: true }); sc();
    return () => { window.removeEventListener('keydown', k); window.removeEventListener('scroll', sc); };
  }, []);
  useEffect(() => { document.documentElement.toggleAttribute('data-hp-menu-open', mobile); }, [mobile]);
  const run = ticker.length ? ticker : [{ who: 'units', verb: 'indexed', what: 'nothing yet', tool: 'devnet' }];
  return (
    <header className="hp-header">
      <div className="ticker">
        <div className="track">
          {[0, 1].map((n) => (
            <ul className="run" key={n} aria-hidden={n === 1 || undefined} aria-label={n === 0 ? 'Recent battle events' : undefined}>
              {run.map((t, i) => <li className="item" key={i}><span className="at">@</span><span className="who">{t.who}</span><span className="verb">{t.verb}</span><span className="what">{t.what}</span><span className="verb">with</span><span className="tool">{t.tool}</span><span className="sep" /></li>)}
            </ul>
          ))}
        </div>
      </div>
      <nav className="bar" aria-label="Primary">
        <Link className="brand" href="/" aria-label="units home"><span className="brand-mark" aria-hidden /><span className="brand-word">units</span></Link>
        <ul className="groups">
          {GROUPS.map((g) => {
            const active = g.href ? on(g.href, path) : g.items.some((i) => on(i.href, path));
            if (g.href) return <li className="group" key={g.label}><Link className={`pill pill-group hp-squircle ${active ? 'on' : ''}`} style={{ ['--hp-pill-w' as string]: g.w }} href={g.href}><span>{g.label}</span></Link></li>;
            const isOpen = open === g.label;
            const rows = Math.ceil(g.items.length / 2);
            return (
              <li className="group" key={g.label} onMouseEnter={() => show(g.label)} onMouseLeave={hide} onFocus={() => show(g.label)} onBlur={(e) => { if (!e.currentTarget.contains(e.relatedTarget as Node)) hide(); }}>
                <button type="button" className={`pill pill-group hp-squircle ${active ? 'on' : ''}`} style={{ ['--hp-pill-w' as string]: g.w }} aria-expanded={isOpen} aria-controls={`menu-${g.label}`} onClick={() => setOpen(isOpen ? null : g.label)}><span>{g.label}</span><span className="chev hp-squircle" aria-hidden><Chev /></span></button>
                <div className="panel hp-squircle" id={`menu-${g.label}`} data-open={isOpen || undefined} style={{ ['--rows' as string]: rows }} aria-label={g.label}>
                  <div className="items">
                    {g.items.map((it, i) => <Link className={`item ${on(it.href, path) ? 'on' : ''}`} key={it.href} href={it.href} style={{ ['--x' as string]: i % 2 ? 220 : 20, ['--baseline' as string]: 30.2 + Math.floor(i / 2) * 68, ['--i' as string]: i }}><span className="icon" aria-hidden>{it.icon}</span><span className="title">{it.label}</span><span className="body">{it.what.split('\n').map((l, j) => <span key={j}>{j ? <br /> : null}{l}</span>)}</span></Link>)}
                    {Array.from({ length: rows - 1 }, (_, r) => <span className="divider" key={r} style={{ ['--baseline' as string]: 30.2 + r * 68 }} aria-hidden />)}
                  </div>
                  {g.promo ? <div className="promo"><img className="promo-img hp-squircle" src={g.promo.img} alt="" width={560} height={396} /><span className="promo-title">{g.promo.title}</span><span className="promo-body">{g.promo.body.split('\n').map((l, j) => <span key={j}>{j ? <br /> : null}{l}</span>)}</span></div> : null}
                </div>
              </li>
            );
          })}
        </ul>
        <div className="actions">
          <Link className="pill pill-cta hp-squircle" style={{ ['--hp-pill-w' as string]: 80 }} href="/launch">Launch</Link>
          <button type="button" className="pill pill-dark hp-squircle" style={{ ['--hp-pill-w' as string]: 110 }} onClick={connect} title={wallet ? `Connected: ${wallet}` : 'Connect a Solana wallet'}><svg className="user" viewBox="2.75 1.5 10.5 13" aria-hidden><path d="M12.5 13.75V12.4167C12.5 11.7094 12.23 11.0311 11.75 10.531C11.26 10.031 10.61 9.75 9.93 9.75H6.07C5.39 9.75 4.74 10.031 4.25 10.531C3.77 11.0311 3.5 11.7094 3.5 12.4167V13.75M10.57 4.6667C10.57 6.0014 9.42 7.0833 8 7.0833C6.58 7.0833 5.43 6.0014 5.43 4.6667C5.43 3.332 6.58 2.25 8 2.25C9.42 2.25 10.57 3.332 10.57 4.6667Z" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" /></svg><span>{wallet ? `${wallet.slice(0, 4)}…${wallet.slice(-4)}` : 'Connect wallet'}</span></button>
          <button type="button" className="burger" aria-expanded={mobile} aria-controls="hp-mobile-menu" aria-label="Menu" onClick={() => setMobile(!mobile)}><span className="burger-bars" aria-hidden><span /><span /><span /></span></button>
        </div>
      </nav>
      <div className="mobile-menu" id="hp-mobile-menu" hidden={!mobile}>
        <nav className="inner" aria-label="Menu">
          {GROUPS.filter((g) => g.items.length).map((g) => (
            <section className="group" key={g.label}><h2 className="group-title hp-squircle">{g.label}</h2><ul className="rows">{g.items.map((it) => <li key={it.href}><Link className="row" href={it.href}><span className="icon" aria-hidden>{it.icon}</span><span className="label">{it.label}</span><span className="chev" aria-hidden><Chev /></span></Link></li>)}</ul></section>
          ))}
          <section className="group"><h2 className="group-title hp-squircle">Docs</h2><ul className="rows"><li><Link className="row" href="/docs"><span className="icon" aria-hidden><BookOpen /></span><span className="label">How units works</span></Link></li></ul></section>
        </nav>
      </div>
    </header>
  );
}
