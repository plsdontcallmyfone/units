import { Empty, Head, Panel } from '@/components/ui';

export default function Explorer() {
  return (
    <>
      <Head eyebrow="Explorer" title="Accounts of the standard" lede="Mints, holdings, pools, items, templates, proposals, slot states, equips, raid ledgers, war states, chests, treaty inboxes, seasons, loot tables, rolls and quest marks." />
      <Panel title="Look up">
        <Empty title="Paste an address on a token or item page" what="The explorer decodes an account by its owner and discriminator once the programs are on this cluster." />
      </Panel>
    </>
  );
}
