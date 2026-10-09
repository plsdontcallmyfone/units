// Changed by Hookwars: the Docs section renders the public guide (docs/guide) compiled at build time.
import type { Metadata } from 'next';
import { notFound } from 'next/navigation';
import { DocsShell } from '@/components/docs-shell';
import { docBySlug } from '@/lib/docs';
import './docs.css';

export const metadata: Metadata = { title: 'Docs | units' };

export default function DocsIndex() {
  const page = docBySlug('');
  if (!page) notFound();
  return <DocsShell page={page} current="" />;
}
