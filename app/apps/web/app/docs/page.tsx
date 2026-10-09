import { PARAMS, TEMPLATES } from '@hookwars/shared';
import { Head, Panel } from '@/components/ui';
import { TemplateTable } from '@/components/templates';

const SECTIONS: [string, string][] = [
  ['The standard', 'Bordrless tokens, kept: hooks answer with cuts, burns, a fee and 64 bytes per holding. units runs up to MAX_SLOTS hooks per token.'],
  ['Slots', 'Each slot has a kind, bounds, an equip rule and a notice fixed at launch. Only the item in it changes.'],
  ['Items', 'An item is a registered template with parameters. Its owner earns a royalty from what it collects, settled later by anyone for a bounty.'],
  ['Relations', 'Items read other tokens through time-weighted prices, see a trade route, and make treaties both tokens equip. No token is ever called because another traded.'],
  ['War', 'A chest per token, funded by fees, spends only through bounded cranks: siege, counter-strike, bounties. Holders decide through the War orders item.'],
];

export default function Docs() {
  return (
    <>
      <Head eyebrow="Docs" title="How units works" lede="Spot only: every action is a transfer, a cut, a burn, a fee change or a spot swap." />
      <div className="grid">
        <Panel title="Concepts" flush>
          <div className="rows">{SECTIONS.map(([t, d]) => <div className="row" key={t} style={{ gridTemplateColumns: '160px minmax(0,1fr)' }}><strong>{t}</strong><span className="muted">{d}</span></div>)}</div>
        </Panel>
        <Panel title={`Templates (${TEMPLATES.length})`} flush><TemplateTable registered={[]} /></Panel>
        <Panel title="Parameters" meta="a dash means not set yet" flush>
          <table><thead><tr><th>Name</th><th>Meaning</th><th className="num">Value</th></tr></thead>
            <tbody>{PARAMS.map((p) => <tr key={p.name}><td>{p.name}</td><td className="muted">{p.meaning}</td><td className="num" title={p.source ?? undefined}>{p.value ?? '-'}</td></tr>)}</tbody></table>
        </Panel>
      </div>
    </>
  );
}
