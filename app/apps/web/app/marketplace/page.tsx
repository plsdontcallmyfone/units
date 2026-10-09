import { Empty, Head, Panel } from '@/components/ui';

export default function Marketplace() {
  return (
    <>
      <Head eyebrow="Marketplace" title="Configs and items" lede="Listed launch configs, as upstream, and items: an item is a supply-1 token, so selling it sells its royalty stream." />
      <Panel title="Listings">
        <Empty title="Nothing listed on this cluster" what="Before you sell an item, settle and claim its royalties: the buyer receives whatever is unclaimed." />
      </Panel>
    </>
  );
}
