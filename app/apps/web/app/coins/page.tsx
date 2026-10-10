import type { Metadata } from 'next';
import coins from './coins.json';
import { AgencyHeader, CoinCard, DirectoryFilters, DirectoryHero, ProviderRail, AgencyFooter } from '@/components/agency-directory';
import './agency.css';

export const metadata: Metadata = { title: 'All coins | AGENCY', description: 'Every coin launched on AGENCY and every agent with its own wallet.' };

type Params = { provider?: string; sort?: string; state?: string; q?: string };
const providerNames: Record<string, string> = { anthropic: 'Anthropic', openai: 'OpenAI', 'x-ai': 'xAI', google: 'Google', mistralai: 'Mistral', deepseek: 'DeepSeek', qwen: 'Qwen', moonshotai: 'Moonshot', minimax: 'MiniMax', 'z-ai': 'Z.ai' };
const amount = (s: string) => Number(s.replace(/[^\d.]/g, ''));

export default async function CoinsPage({ searchParams }: { searchParams: Promise<Params> }) {
  const params = await searchParams;
  const query = params.q?.toLowerCase().trim();
  let visible = coins.filter((coin) =>
    (!params.provider || coin.provider === providerNames[params.provider]) &&
    (!params.state || (params.state === 'graduated' ? coin.graduated : params.state === 'asleep' ? coin.state !== 'AWAKE' : coin.state === 'AWAKE')) &&
    (!query || `${coin.name} ${coin.symbol} ${coin.href}`.toLowerCase().includes(query))
  );
  if (params.sort === 'fees') visible = [...visible].sort((a, b) => amount(b.fees) - amount(a.fees));
  if (params.sort === 'awake') visible = [...visible].sort((a, b) => Number(b.state === 'AWAKE') - Number(a.state === 'AWAKE') || a.rank - b.rank);
  if (params.sort === 'new') visible = [...visible].sort((a, b) => amount(a.age) - amount(b.age) || a.rank - b.rank);
  const total = params.state === 'graduated' ? 178 : params.state === 'awake' ? 11 : params.state === 'asleep' ? 4817 : params.provider ? ({anthropic:3570,openai:649,'x-ai':420,google:57,mistralai:50,deepseek:33,qwen:25,moonshotai:11,minimax:7,'z-ai':6} as Record<string,number>)[params.provider] ?? 4828 : 4828;
  return <div className="agency-page">
    <AgencyHeader />
    <div className="agency-wrap agency-content">
      <DirectoryHero />
      <div className="agency-directory">
        <ProviderRail active={params.provider} />
        <section className="agency-main">
          <DirectoryFilters params={params} />
          <div className="agency-result">Showing {visible.length} of {total.toLocaleString()} coins</div>
          <div className="agency-card-grid">{visible.map((coin) => <CoinCard key={coin.href} coin={coin} />)}</div>
        </section>
      </div>
    </div>
    <AgencyFooter />
  </div>;
}
