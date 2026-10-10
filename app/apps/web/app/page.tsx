import type { Metadata } from 'next';
import { Landing } from '@/components/landing';

export const metadata: Metadata = { title: 'units', description: 'Tokens whose hooks are owned items. Spot only, on Solana.' };

/** The landing page is the archived plnty site (public/plnty), framed under the site's own header. */
export default function Home() { return <Landing />; }
