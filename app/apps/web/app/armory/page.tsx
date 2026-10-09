import Link from 'next/link';
import type { ItemSummary, Page, TemplateInfo } from '@hookwars/shared';
import { read } from '@/lib/api';
import { short } from '@/lib/format';
import { Empty, Head, Panel, ReadFailed, Stat } from '@/components/ui';
import { TemplateTable } from '@/components/templates';

export default async function Armory() {
  const [templates, items] = await Promise.all([read<TemplateInfo[]>('/v1/templates'), read<Page<ItemSummary>>('/v1/items')]);
  const reg = templates.ok ? templates.data : [];
  const list = items.ok ? items.data.items : [];
  return (
    <>
      <Head eyebrow="Armory" title="Hooks are items" lede="Each item is a registered template with its own parameters. Its owner earns a royalty from what it collects, every time it runs, on any token that equips it." />
      <div className="stats">
        <Stat label="Templates defined" value={9} sub="04 section 3" />
        <Stat label="Registered here" value={templates.ok ? reg.length : '-'} sub="on this cluster" />
        <Stat label="Items" value={items.ok ? list.length : '-'} sub="not forged away" />
        <Stat label="Looted" value={items.ok ? list.filter((i) => i.source === 'loot').length : '-'} />
        <Stat label="Forged" value={items.ok ? list.filter((i) => i.source === 'forged').length : '-'} />
      </div>
      <div className="grid">
        <Panel title="Templates" meta="the only code a slot can run, besides the kit" flush>
          {!templates.ok ? <ReadFailed what="templates" error={templates.error} /> : <TemplateTable registered={reg} />}
        </Panel>
        <div className="grid cols-2">
          <Panel title="Items" meta="newest first" flush>
            {!items.ok ? <ReadFailed what="items" error={items.error} /> : list.length === 0 ? (
              <Empty title="No items yet" what="An item is created from an open template, dropped as loot, or forged from two of a kind. Each shows its sentence, level, owner and royalties realized." next="Creating one needs the armory on this cluster." />
            ) : (
              <div className="rows">
                {list.map((it) => (
                  <Link className="row" key={it.item} href={`/armory/items/${it.item}`}>
                    <span className="chip">{it.templateName} L{it.level}</span>
                    <span className="muted" style={{ minWidth: 0 }}>{it.paramsText}</span>
                    <span className="faint">{short(it.owner)}</span>
                  </Link>
                ))}
              </div>
            )}
          </Panel>
          <div className="grid">
            <Panel title="Forge">
              <Empty title="Burn two, forge one" what="Two unequipped items of one forgeable template become one item a level higher. Each field moves toward its ceiling or floor by FORGE_GAIN_BPS of the distance left, and never past it. War orders, Treaty and Tribute cannot be forged." next="Settle and claim both items' royalties first: forging destroys unclaimed royalties, so the site will not build it while either has any." />
            </Panel>
            <Panel title="Loot">
              <Empty title="Tickets come from raids and quests" what="A raid buy above LOOT_MIN_RAID_LAMPORTS stamps a loot ticket in the buyer's holding; a quest adds one. Rolling spends a ticket and asks the randomness oracle; the reveal mints an item from the season's drop table." next="A cancelled roll does not return its ticket." />
            </Panel>
          </div>
        </div>
      </div>
    </>
  );
}
