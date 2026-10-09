// Changed by Hookwars: the public guide (docs/guide), compiled at build time by scripts/build-docs.mjs.
import content from './docs-content.json';

export interface DocHeading { id: string; text: string; depth: number }
export interface DocPage { slug: string; file: string; title: string; html: string; headings: DocHeading[] }
export type DocNav = { type: 'group'; title: string; depth: number } | { type: 'link'; title: string; slug: string; depth: number };

const data = content as unknown as { source: string; nav: DocNav[]; pages: DocPage[] };

export const DOC_SOURCE: string = data.source;
export const DOC_NAV: DocNav[] = data.nav;
export const DOC_PAGES: DocPage[] = data.pages;
export const docBySlug = (slug: string): DocPage | undefined => DOC_PAGES.find((p) => p.slug === slug);

/** The previous and next pages in sidebar order. */
export function neighbours(slug: string): { prev: DocNav | null; next: DocNav | null } {
  const links = DOC_NAV.filter((n): n is Extract<DocNav, { type: 'link' }> => n.type === 'link');
  const i = links.findIndex((l) => l.slug === slug);
  return { prev: i > 0 ? links[i - 1]! : null, next: i >= 0 && i < links.length - 1 ? links[i + 1]! : null };
}
