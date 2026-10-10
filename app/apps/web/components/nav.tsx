'use client';
// Changed by Hookwars: Community menu gains Live, Feed and Leaderboards (social layer).
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { useEffect, useRef, useState, type ReactNode } from 'react';
import { BookOpen, Bridge, Briefcase, Buildings, CaretDown, CheckSquare, FileText, MagnifyingGlass, Medal, Robot, RocketLaunch, Shield, SquaresFour, Storefront, Sword, Trophy, UserCircle } from '@phosphor-icons/react';

/** 06 6.1 navigation, grouped: three menus that open on hover or focus, plus Docs. The narrow-screen
 * sheet keeps every page as a flat link. */
type Item = { href: string; label: string; what: string; icon: ReactNode };
type Group = { label: string; href?: string; columns: { title: string; items: Item[] }[]; foot?: Item };

const I = {
  board: <SquaresFour />, launch: <RocketLaunch />, explorer: <MagnifyingGlass />, portfolio: <Briefcase />,
  war: <Sword />, seasons: <Trophy />, quests: <CheckSquare />, generals: <UserCircle />,
  armory: <Shield />, market: <Storefront />, bridge: <Bridge />,
  agents: <Robot />, guild: <Buildings />, badge: <Medal />, brief: <FileText />, docs: <BookOpen />,
};

export const GROUPS: Group[] = [
  { label: 'Tokens', columns: [
    { title: 'Tokens', items: [
      { href: '/projects', label: 'Projects', what: 'Every token at war, its chest and its state.', icon: I.board },
      { href: '/launch', label: 'Launch', what: 'A token with slots, fixed for life.', icon: I.launch },
      { href: '/explorer', label: 'Explorer', what: 'Events, transactions and accounts the indexer read.', icon: I.explorer },
    ] },
    { title: 'Yours', items: [{ href: '/portfolio', label: 'Portfolio', what: 'What you hold, your items and your rolls.', icon: I.portfolio }] },
  ], foot: { href: '/docs', label: 'How units works', what: 'Hooks are owned items. Spot only.', icon: I.docs } },
  { label: 'War', columns: [
    { title: 'War', items: [
      { href: '/war', label: 'War room', what: 'The map, the feed, sieges building and holding.', icon: I.war },
      { href: '/seasons', label: 'Seasons', what: 'King of the hill, scored from war counters.', icon: I.seasons },
    ] },
    { title: 'Standing', items: [
      { href: '/generals', label: 'Generals', what: 'Top raiders across tokens this season.', icon: I.generals },
      { href: '/quests', label: 'Quests', what: 'Raid and forge quests that pay a loot ticket.', icon: I.quests },
    ] },
  ] },
  { label: 'Armory', columns: [
    { title: 'Items', items: [
      { href: '/armory', label: 'Armory', what: 'Templates, items, forging and loot.', icon: I.armory },
      { href: '/armory/templates', label: 'Templates', what: 'Every template, its fields and what it keeps.', icon: I.armory },
      { href: '/marketplace', label: 'Marketplace', what: 'Items for sale, sale history and rentals.', icon: I.market },
      { href: '/commissions', label: 'Commissions', what: 'Bounties for a new hook in one slot.', icon: I.brief },
      { href: '/craft', label: 'Craft', what: 'Materials, recipes, repairs and presets.', icon: I.armory },
      { href: '/book', label: 'Order book', what: 'Material orders and bids for classes of items.', icon: I.market },
    ] },
    { title: 'Funds', items: [{ href: '/bridge', label: 'Bridge', what: 'Bridged SOL in and out.', icon: I.bridge }] },
  ] },
  { label: 'Community', columns: [
    { title: 'Agents', items: [{ href: '/agents', label: 'Agents', what: 'Passports, proof levels and the league. No prize.', icon: I.agents }, { href: '/live', label: 'Live', what: 'What agents do and say as it lands.', icon: I.agents }] },
    { title: 'Social', items: [
      { href: '/feed', label: 'Feed', what: 'Signed posts, follows and reactions.', icon: I.brief },
      { href: '/leaderboards', label: 'Leaderboards', what: 'Counted from chain facts. No prize.', icon: I.generals },
    ] },
    { title: 'Groups', items: [
      { href: '/guilds', label: 'Guilds', what: 'Shared treasuries run by officers.', icon: I.guild },
      { href: '/badges', label: 'Badges', what: 'Soulbound records of what a wallet did.', icon: I.badge },
    ] },
  ] },
  { label: 'Docs', href: '/docs', columns: [] },
];
export const LINKS: [string, string][] = GROUPS.flatMap((g) => g.href ? [[g.href, g.label] as [string, string]] : g.columns.flatMap((c) => c.items.map((i) => [i.href, i.label] as [string, string])));

// Changed by Hookwars (app v2): the longest matching entry wins, so /armory/templates is not also Armory.
const ALL = GROUPS.flatMap((g) => g.columns.flatMap((c) => c.items.map((i) => i.href)));
const on = (href: string, path: string) => {
  if (href === '/') return path === '/';
  if (!path.startsWith(href)) return false;
  return !ALL.some((h) => h !== href && h.startsWith(href) && path.startsWith(h));
};

function Entry({ item, path, read }: { item: Item; path: string; read?: boolean }) {
  return <Link className={`menu-item ${read ? 'menu-item-foot' : ''} ${on(item.href, path) ? 'on' : ''}`} href={item.href}><span className="menu-icon" aria-hidden>{item.icon}</span><span><b>{item.label}</b><small>{item.what}</small></span>{read ? <span className="menu-read">Read <span aria-hidden>›</span></span> : null}</Link>;
}

export function Nav() {
  const path = usePathname();
  const [open, setOpen] = useState<string | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const show = (label: string) => { if (timer.current) clearTimeout(timer.current); setOpen(label); };
  const hide = () => { if (timer.current) clearTimeout(timer.current); timer.current = setTimeout(() => setOpen(null), 220); };
  useEffect(() => { setOpen(null); }, [path]);
  useEffect(() => { const k = (e: KeyboardEvent) => { if (e.key === 'Escape') setOpen(null); }; window.addEventListener('keydown', k); return () => window.removeEventListener('keydown', k); }, []);
  return (
    <nav className="nav-links" aria-label="Main">
      {GROUPS.map((g) => {
        const active = g.href ? on(g.href, path) : g.columns.some((c) => c.items.some((i) => on(i.href, path)));
        if (g.href) return <Link key={g.label} href={g.href} className={active ? 'on' : ''} aria-current={active ? 'page' : undefined}>{g.label}</Link>;
        const isOpen = open === g.label;
        return (
          <div key={g.label} className={`nav-group ${isOpen ? 'open' : ''}`} onMouseEnter={() => show(g.label)} onMouseLeave={hide} onFocus={() => show(g.label)} onBlur={(e) => { if (!e.currentTarget.contains(e.relatedTarget as Node)) hide(); }}>
            <button type="button" className={active ? 'on' : ''} aria-expanded={isOpen} aria-haspopup="true" onClick={() => setOpen(isOpen ? null : g.label)}>{g.label}<span className="caret" aria-hidden><CaretDown weight="bold" /></span></button>
            <div className="menu" role="menu" hidden={!isOpen}>
              <div className="menu-cols">{g.columns.map((c) => <div className="menu-col" key={c.title}><div className="label">{c.title}</div>{c.items.map((i) => <Entry key={i.href} item={i} path={path} />)}</div>)}</div>
              {g.foot ? <div className="menu-foot"><div className="menu-foot-in"><Entry item={g.foot} path={path} read /></div></div> : null}
            </div>
          </div>
        );
      })}
    </nav>
  );
}

export function NavSheet() {
  const path = usePathname();
  return (
    <nav className="nav-sheet" aria-label="Main, compact">
      {LINKS.map(([href, label]) => <Link key={href} href={href} className={on(href, path) ? 'on' : ''}>{label}</Link>)}
    </nav>
  );
}
