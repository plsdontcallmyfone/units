import { Empty, Head, Panel } from '@/components/ui';

export default function Bridge() {
  return (
    <>
      <Head eyebrow="Bridge" title="SPL and SOL in and out" lede="Upstream's bridge, unchanged: any SPL or Token-2022 mint without a transfer hook, and SOL, one for one into the standard and back." />
      <Panel title="Bridge">
        <Empty title="Opens once the bridge program is on this cluster" what="Every launch is quoted in bridged SOL; trades wrap and unwrap SOL in the same transaction, so you only ever see SOL." />
      </Panel>
    </>
  );
}
