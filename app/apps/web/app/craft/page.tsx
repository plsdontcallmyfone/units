// Changed by Hookwars: new file (app pass v3). Crafting (11 section 5): materials and their season
// caps, recipes that make or repair items, the armory's composite presets, and the craft, repair,
// fuse and preset forms. Every figure is read from the chain; nothing is estimated.
import Link from 'next/link';
import { TEMPLATES } from '@hookwars/shared';
import { read } from '@/lib/api';
import { short, sol } from '@/lib/format';
import { Empty, Head, Panel, ReadFailed } from '@/components/ui';
import { Action, Actions } from '@/components/action';

type Input = { materialId: number; amount: string };
type Recipe = { address: string; id: number; uses: string; terms: { kind: number; inputs: Input[]; feeLamports: string; templateId: number; chargesRestored: number; minLevel: number; active: boolean } };
type Material = { address: string; id: number; mint: string; name: string; emissionCapPerSeason: string; emittedThisSeason: string; emittedTotal: string; burnedTotal: string };
type Preset = { address: string; id: number; name: string; templateIds: number[] };
type Craft = { config: { treasury: string; seasonPool: string } | null; materials: Material[]; recipes: Recipe[]; presets: Preset[] };
const tname = (id: number) => TEMPLATES.find((t) => t.id === id)?.name ?? `template ${id}`;

export default async function CraftPage() {
  const r = await read<Craft>('/v1/craft');
  const mat = (id: number, d: Craft | null) => d?.materials.find((m) => m.id === id)?.name ?? `material ${id}`;
  const d = r.ok ? r.data : null;
  const recipes = (kind: number) => (d?.recipes ?? []).filter((x) => x.terms.kind === kind);
  const table = (kind: number) => (
    <table><thead><tr><th>Recipe</th><th>{kind === 0 ? 'Makes' : 'Repairs'}</th><th>Inputs</th><th className="r">Fee</th><th className="r">Uses</th></tr></thead>
      <tbody>{recipes(kind).map((x) => (
        <tr key={x.address}>
          <td>#{x.id}{x.terms.active ? null : <span className="chip warn">paused</span>}{x.terms.minLevel ? <div className="faint">crafter level {x.terms.minLevel}</div> : null}</td>
          <td><Link href={`/armory/templates/${x.terms.templateId}`}>{tname(x.terms.templateId)}</Link>{kind === 1 ? <div className="faint">restores {x.terms.chargesRestored} charges</div> : null}</td>
          <td>{x.terms.inputs.map((i) => `${i.amount} ${mat(i.materialId, d)}`).join(', ')}</td>
          <td className="r">{sol(x.terms.feeLamports, 4)}</td>
          <td className="r">{x.uses}</td>
        </tr>
      ))}</tbody></table>
  );
  return (
    <>
      <Head eyebrow="Craft" title="Materials and recipes" lede="Materials drop from settles, raids, seasons and quests, capped per season. Recipes burn them to make an item or to repair one that wore out: a dormant item answers every hook call with its default until it is repaired."
        right={<Link className="btn sm" href="/book">Order book</Link>} />
      {!r.ok ? <Panel title="Craft"><ReadFailed what="craft" error={r.error} /></Panel> : !d!.config ? (
        <Panel title="Craft"><Empty title="Craft is not set up on this cluster" what="Craft has no config here yet, so there are no materials or recipes to show." /></Panel>
      ) : (
        <>
          <div className="grid cols-main">
            <Panel title="Recipes that make items" meta={`${recipes(0).length} recipes`} flush>
              {recipes(0).length === 0 ? <Empty title="No crafting recipes yet" what="The admin adds recipes behind the timelock." /> : table(0)}
            </Panel>
            <Panel title="Materials" meta={`${d!.materials.length} materials`} flush>
              {d!.materials.length === 0 ? <Empty title="No materials yet" what="Materials are created by the admin, each with its season cap." /> : (
                <table><thead><tr><th>Material</th><th className="r">This season</th><th className="r">Burned</th></tr></thead>
                  <tbody>{d!.materials.map((m) => (
                    <tr key={m.address}><td>{m.name}<div className="faint">{short(m.mint)}</div></td><td className="r">{m.emittedThisSeason} of {m.emissionCapPerSeason}</td><td className="r">{m.burnedTotal}</td></tr>
                  ))}</tbody></table>
              )}
            </Panel>
          </div>
          <div style={{ height: 16 }} />
          <div className="grid cols-main">
            <Panel title="Repair recipes" meta={`${recipes(1).length} recipes`} flush>
              {recipes(1).length === 0 ? <Empty title="No repair recipes yet" what="Without one, a worn item stays dormant." /> : table(1)}
            </Panel>
            <Panel title="Presets" meta="composites in a fixed module order" flush>
              {d!.presets.length === 0 ? <Empty title="No presets registered" what="A preset fixes which templates a composite holds, in order; each mint picks its own params." /> : (
                <div className="rows">{d!.presets.map((p) => <div className="row" key={p.address}><b>{p.name}</b><span className="muted">#{p.id}</span><span className="faint">{p.templateIds.map(tname).join(' + ')}</span></div>)}</div>
              )}
            </Panel>
          </div>
        </>
      )}
      <div style={{ height: 16 }} />
      <Panel title="Act">
        <Actions>
          <Action route="craft" title="Craft" what="Burns the recipe's inputs and pays its fee; the armory mints the item to you." fields={[{ name: 'recipeId', label: 'Recipe', kind: 'int' }]} />
          <Action route="craft/repair" title="Repair" what="Burns the inputs, restores charges and wakes a dormant item you hold." fields={[{ name: 'recipeId', label: 'Repair recipe', kind: 'int' }, { name: 'itemMint', label: 'Item mint', kind: 'key' }]} />
          <Action route="items/fuse" title="Fuse" what="Burns two or more unequipped items you hold into one composite. One way: there is no unfuse. Claim their royalties first." fields={[{ name: 'components', label: 'Components', kind: 'json', hint: '[{"itemMint":"...","start":0,"count":1}, ...]' }, { name: 'royaltyBps', label: 'Royalty (bps)', kind: 'int' }]} />
          <Action route="presets/mint" title="Mint a preset" what="A composite of the preset's templates, in its order, with your params." fields={[{ name: 'presetId', label: 'Preset', kind: 'int' }, { name: 'modules', label: 'Modules', kind: 'json', hint: '[{"templateId":1,"params":[],"targetStart":0,"targetCount":0}, ...]' }, { name: 'royaltyBps', label: 'Royalty (bps)', kind: 'int' }]} />
          <Action route="counters/init" title="Open counters" what="Your author and claim counters, which feed the ItemsAuthored and RoyaltiesClaimed badges." />
        </Actions>
      </Panel>
    </>
  );
}
