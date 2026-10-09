import { TEMPLATES, type TemplateInfo } from '@hookwars/shared';

const FORGE: Record<string, string> = { towardCeiling: 'toward ceiling', towardFloor: 'toward floor', keep: 'kept', none: 'not forgeable' };

/** Templates as the chain registers them, or, before registration, as 04 defines them (labelled so). */
export function TemplateTable({ registered }: { registered: TemplateInfo[] }) {
  const byId = new Map(registered.map((t) => [t.templateId, t]));
  return (
    <table>
      <thead><tr><th>Template</th><th>Kind</th><th>Fields (floor, ceiling, forge)</th><th>Registered</th></tr></thead>
      <tbody>
        {TEMPLATES.map((t) => {
          const r = byId.get(t.id);
          return (
            <tr key={t.id}>
              <td><div style={{ fontWeight: 600 }}>{t.name}</div><div className="faint" style={{ fontSize: 12 }}>id {t.id} · {t.spec}</div></td>
              <td><span className="chip">{t.kind}</span></td>
              <td>
                <div style={{ display: 'grid', gap: 2 }}>
                  {t.fields.map((f) => {
                    const lo = r ? r.fields.find((x) => x.index === f.index)?.min : f.floor;
                    const hi = r ? r.fields.find((x) => x.index === f.index)?.max : f.ceiling;
                    return <div key={f.index} className="muted" style={{ fontSize: 13 }}><span style={{ color: 'var(--text)' }}>{f.name}</span> {String(lo)} to {String(hi)}, {FORGE[f.forge]}</div>;
                  })}
                </div>
              </td>
              <td>{r ? <span className={`chip ${r.status === 'active' ? 'ok' : ''}`}>{r.status}</span> : <span className="chip warn" title="Floors and ceilings are parameter names until the armory registers the template on this cluster">not on this cluster</span>}</td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}
