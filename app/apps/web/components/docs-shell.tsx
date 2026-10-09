import Link from 'next/link';
import type { ReactNode } from 'react';
import { DOC_NAV, neighbours, type DocPage } from '@/lib/docs';

/** The Docs layout: the guide's SUMMARY.md as the sidebar, the page, and its section index. */
export function DocsShell({ page, current, children }: { page?: DocPage; current: string; children?: ReactNode }) {
  const { prev, next } = page ? neighbours(page.slug) : { prev: null, next: null };
  return (
    <div className="docs">
      <nav className="docs-side" aria-label="Docs">
        {DOC_NAV.map((n, i) => n.type === 'group'
          ? <div key={`g${i}`} className="docs-group">{n.title}</div>
          : <Link key={`l${i}`} href={n.slug ? `/docs/${n.slug}` : '/docs'} className={`docs-link ${n.slug === current ? 'on' : ''}`} style={{ paddingLeft: 12 + n.depth * 14 }} aria-current={n.slug === current ? 'page' : undefined}>{n.title}</Link>)}
        <div className="docs-group">Reference</div>
        <Link href="/docs/protocol" className={`docs-link ${current === 'protocol' ? 'on' : ''}`} style={{ paddingLeft: 12 }}>Templates and parameters</Link>
      </nav>
      <article className="docs-body">
        {page ? <div className="docs-prose" dangerouslySetInnerHTML={{ __html: page.html }} /> : children}
        {page && (prev || next) ? (
          <div className="docs-pager">
            {prev && prev.type === 'link' ? <Link href={prev.slug ? `/docs/${prev.slug}` : '/docs'}><small>Previous</small>{prev.title}</Link> : <span />}
            {next && next.type === 'link' ? <Link href={next.slug ? `/docs/${next.slug}` : '/docs'} className="next"><small>Next</small>{next.title}</Link> : <span />}
          </div>
        ) : null}
      </article>
      {page && page.headings.length > 1 ? (
        <aside className="docs-toc" aria-label="On this page">
          <div className="docs-group">On this page</div>
          {page.headings.map((h) => <a key={h.id} href={`#${h.id}`} style={{ paddingLeft: h.depth === 3 ? 14 : 0 }}>{h.text}</a>)}
        </aside>
      ) : null}
    </div>
  );
}
