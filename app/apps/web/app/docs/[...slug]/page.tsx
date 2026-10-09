// Changed by Hookwars: one route per guide page, generated from docs/guide at build time.
import type { Metadata } from 'next';
import { notFound } from 'next/navigation';
import { DocsShell } from '@/components/docs-shell';
import { DOC_PAGES, docBySlug } from '@/lib/docs';
import '../docs.css';

export const dynamicParams = false;
export function generateStaticParams() {
  return DOC_PAGES.filter((p) => p.slug !== '').map((p) => ({ slug: p.slug.split('/') }));
}

export async function generateMetadata({ params }: { params: Promise<{ slug: string[] }> }): Promise<Metadata> {
  const p = docBySlug((await params).slug.join('/'));
  return { title: p ? `${p.title} | units docs` : 'units docs' };
}

export default async function DocPageRoute({ params }: { params: Promise<{ slug: string[] }> }) {
  const slug = (await params).slug.join('/');
  const page = docBySlug(slug);
  if (!page) notFound();
  return <DocsShell page={page} current={slug} />;
}
