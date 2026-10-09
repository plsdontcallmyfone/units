import type { QuestInfo } from '@hookwars/shared';
import { read } from '@/lib/api';
import { Empty, Head, Panel, ReadFailed } from '@/components/ui';

export default async function Quests() {
  const q = await read<QuestInfo[]>('/v1/quests');
  return (
    <>
      <Head eyebrow="Quests" title="Two quests the program checks" lede="Each pays one loot ticket, once per period, recorded in a QuestMark account. Nothing else is a quest." />
      <div className="grid cols-main">
        <Panel title="Quests" flush>
          {!q.ok ? <ReadFailed what="quests" error={q.error} /> : (
            <div className="rows">{q.data.map((x) => <div className="row" key={x.questId}><span className="chip accent">{x.name}</span><span className="muted">{x.sentence}</span><span /></div>)}</div>
          )}
        </Panel>
        <Panel title="Why no holding quest">
          <Empty title="Its proof would move with the tokens" what="A holding quest needs an age stamp, and age travels with tokens sent between wallets, so one bag could claim once per wallet per period." />
        </Panel>
      </div>
    </>
  );
}
