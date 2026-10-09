'use client';
import Link from 'next/link';
import { usePathname } from 'next/navigation';

/** 06 6.1 navigation, in the reference's nav-links and the narrow-screen sheet. */
export const LINKS: [string, string][] = [
  ['/', 'Projects'], ['/launch', 'Launch'], ['/war', 'War room'], ['/armory', 'Armory'], ['/seasons', 'Seasons'],
  ['/quests', 'Quests'], ['/generals', 'Generals'], ['/marketplace', 'Marketplace'], ['/portfolio', 'Portfolio'],
  ['/bridge', 'Bridge'], ['/explorer', 'Explorer'], ['/docs', 'Docs'],
];

function on(href: string, path: string): boolean { return href === '/' ? path === '/' : path.startsWith(href); }

export function Nav() {
  const path = usePathname();
  return (
    <nav className="nav-links" aria-label="Main">
      {LINKS.map(([href, label]) => <Link key={href} href={href} className={on(href, path) ? 'on' : ''} aria-current={on(href, path) ? 'page' : undefined}>{label}</Link>)}
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
