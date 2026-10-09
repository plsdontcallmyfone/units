#!/usr/bin/env node
// Compiles the public docs (docs/guide at the repository root) into lib/docs-content.json at build
// time, so the Docs pages are static: SUMMARY.md gives the sidebar order, every page is rendered to
// HTML with heading anchors, mermaid blocks are shown as labelled diagram sources, and links between
// .md files become /docs routes. When docs/guide is absent (a branch without it), the sample pages in
// docs-sample/ are used so the renderer is still exercised. Run by predev, prebuild, pretest and
// pretypecheck.
import { existsSync, readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { Marked } from 'marked';

const here = dirname(fileURLToPath(import.meta.url));
const web = resolve(here, '..');
const guide = process.env.DOCS_DIR ? resolve(process.env.DOCS_DIR) : resolve(web, '../../../docs/guide');
const root = existsSync(join(guide, 'SUMMARY.md')) || existsSync(join(guide, 'index.md')) ? guide : join(web, 'docs-sample');
const out = join(web, 'lib', 'docs-content.json');

/** `concepts/slots.md` -> `concepts/slots`; `index.md` -> ``; `concepts/index.md` -> `concepts`. */
const slugOf = (file) => file.replace(/\\/g, '/').replace(/\.md$/, '').replace(/(^|\/)index$/, '').replace(/^\//, '');
const anchor = (text) => text.toLowerCase().replace(/<[^>]+>/g, '').replace(/&[a-z]+;/g, '').replace(/[^a-z0-9\s-]/g, '').trim().replace(/\s+/g, '-');
const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);

function files(dir) {
  return readdirSync(dir).flatMap((f) => {
    const p = join(dir, f);
    return statSync(p).isDirectory() ? files(p) : f.endsWith('.md') && f !== 'SUMMARY.md' ? [relative(root, p)] : [];
  });
}

/** SUMMARY.md: nested `- [Title](path.md)` lines and `# Heading` group titles. */
function summary() {
  const p = join(root, 'SUMMARY.md');
  const items = [];
  if (!existsSync(p)) return items;
  for (const line of readFileSync(p, 'utf8').split('\n')) {
    const h = /^#{1,6}\s+(.*)$/.exec(line);
    if (h && !/^summary$/i.test(h[1].trim())) { items.push({ type: 'group', title: h[1].trim(), depth: 0 }); continue; }
    const m = /^(\s*)[-*]\s+\[([^\]]+)\]\(([^)]+)\)/.exec(line);
    if (m) items.push({ type: 'link', title: m[2].trim(), slug: slugOf(m[3].replace(/^\.\//, '').split('#')[0]), depth: Math.floor(m[1].replace(/\t/g, '  ').length / 2) });
  }
  return items;
}

function render(file) {
  const src = readFileSync(join(root, file), 'utf8');
  const dir = dirname(file);
  const headings = [];
  const used = new Map();
  const marked = new Marked({ gfm: true });
  marked.use({
    renderer: {
      heading({ tokens, depth }) {
        const html = this.parser.parseInline(tokens);
        let id = anchor(html) || 'section';
        const n = used.get(id) ?? 0; used.set(id, n + 1); if (n) id = `${id}-${n}`;
        if (depth >= 2 && depth <= 3) headings.push({ id, text: html.replace(/<[^>]+>/g, ''), depth });
        return `<h${depth} id="${id}"><a class="anchor" href="#${id}" aria-label="Link to this section">#</a>${html}</h${depth}>\n`;
      },
      code({ text, lang }) {
        if ((lang ?? '').trim() === 'mermaid') {
          return `<figure class="diagram"><figcaption>Diagram (mermaid source)</figcaption><pre><code>${esc(text)}</code></pre></figure>\n`;
        }
        return `<pre class="code"${lang ? ` data-lang="${esc(lang)}"` : ''}><code>${esc(text)}</code></pre>\n`;
      },
      link({ href, title, tokens }) {
        const text = this.parser.parseInline(tokens);
        let h = href ?? '';
        if (!/^[a-z]+:|^#|^\//i.test(h)) {
          const [path, hash] = h.split('#');
          if (path.endsWith('.md')) h = '/docs' + (slugOf(join(dir, path)) ? '/' + slugOf(join(dir, path)) : '') + (hash ? '#' + hash : '');
        }
        const ext = /^https?:/i.test(h);
        return `<a href="${esc(h)}"${title ? ` title="${esc(title)}"` : ''}${ext ? ' rel="noopener noreferrer" target="_blank"' : ''}>${text}</a>`;
      },
    },
  });
  const html = marked.parse(src);
  const t = /^#\s+(.+)$/m.exec(src);
  return { slug: slugOf(file), file, title: t ? t[1].trim() : slugOf(file) || 'Docs', html, headings };
}

const pages = files(root).map(render).sort((a, b) => a.slug.localeCompare(b.slug));
const nav = summary();
for (const p of pages) if (!nav.some((n) => n.type === 'link' && n.slug === p.slug)) nav.push({ type: 'link', title: p.title, slug: p.slug, depth: 0 });
const bad = pages.filter((p) => p.html.includes('—'));
if (bad.length) { console.error(`docs: em dash in ${bad.map((p) => p.file).join(', ')}`); process.exit(1); }
writeFileSync(out, JSON.stringify({ source: relative(resolve(web, '../../..'), root).replace(/\\/g, '/'), nav, pages }, null, 1));
console.log(`docs: ${pages.length} pages from ${relative(web, root)}`);
