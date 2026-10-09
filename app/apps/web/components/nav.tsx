'use client';
import Link from 'next/link';
import { usePathname } from 'next/navigation';

/** 06 6.1 navigation. */
const LINKS: [string, string][] = [
  ['/', 'Projects'], ['/launch', 'Launch'], ['/war', 'War room'], ['/armory', 'Armory'], ['/seasons', 'Seasons'],
  ['/quests', 'Quests'], ['/generals', 'Generals'], ['/marketplace', 'Marketplace'], ['/portfolio', 'Portfolio'],
  ['/bridge', 'Bridge'], ['/explorer', 'Explorer'], ['/docs', 'Docs'],
];

export function Nav() {
  const path = usePathname();
  return (
    <nav className="nav" aria-label="Main">
      {LINKS.map(([href, label]) => (
        <Link key={href} href={href} aria-current={(href === '/' ? path === '/' : path.startsWith(href)) ? 'page' : undefined}>{label}</Link>
      ))}
    </nav>
  );
}
