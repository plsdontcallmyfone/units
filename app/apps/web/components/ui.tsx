import type { ReactNode } from 'react';

export function Panel({ title, meta, children, flush, id }: { title: ReactNode; meta?: ReactNode; children: ReactNode; flush?: boolean; id?: string }) {
  return (
    <section className="panel" id={id} aria-label={typeof title === 'string' ? title : undefined}>
      <div className="panel-head"><h2>{title}</h2>{meta ? <span className="meta">{meta}</span> : null}</div>
      <div className={flush ? 'panel-body flush' : 'panel-body'}>{children}</div>
    </section>
  );
}

/** An empty state names what is empty and what would fill it (06 section 1 rule 3). */
export function Empty({ title, what, next }: { title: string; what: ReactNode; next?: ReactNode }) {
  return (
    <div className="empty" role="status">
      <div className="title">{title}</div>
      <div className="what">{what}</div>
      {next ? <div className="next">{next}</div> : null}
    </div>
  );
}

export function Stat({ label, value, sub }: { label: string; value: ReactNode; sub?: ReactNode }) {
  return (
    <div className="stat">
      <div className="label" title={label}>{label}</div>
      <div className="value">{value}</div>
      {sub ? <div className="sub">{sub}</div> : null}
    </div>
  );
}

export function Head({ eyebrow, title, lede, right }: { eyebrow: string; title: string; lede?: ReactNode; right?: ReactNode }) {
  return (
    <header className="page-head">
      <div>
        <div className="eyebrow">{eyebrow}</div>
        <h1>{title}</h1>
        {lede ? <p className="lede">{lede}</p> : null}
      </div>
      {right}
    </header>
  );
}

/** A read the backend could not make: says so, with the reason. */
export function ReadFailed({ what, error }: { what: string; error: string }) {
  return <Empty title={`Could not read ${what}`} what={`The backend did not answer: ${error}.`} next="Figures stay blank rather than guessed; reload once the backend is reachable." />;
}
