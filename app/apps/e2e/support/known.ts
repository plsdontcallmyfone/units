// Known app bugs the suite found. An entry without `checks` turns the route check into test.fixme
// for the listed projects; an entry with `checks` skips only those checks (the rest still run) and
// notes it in the report. README.md "Known failures" lists the same. Remove an entry when the page
// is fixed: the check then has to pass.
export type Check = 'one-main';
export const KNOWN: Record<string, { projects: string[]; reason: string; checks?: Check[] }> = {
  '/': {
    projects: ['desktop', 'mobile'],
    reason: 'The landing frames the archived plnty site (public/plnty). Its scripts load Google Tag Manager (AW-18440458478) and call plnty\'s Supabase activity feed (both blocked by the CSP), throw "Invalid or unexpected token", and request archive assets that 404 (/_astro/*, /homepage/canvas/*, /plnty/agency/coins/*): about 150 console errors and failed requests per load.',
  },
  '/coins': {
    projects: ['desktop', 'mobile'],
    checks: ['one-main'],
    reason: 'The page renders its own <main> inside the layout\'s <main>: two main landmarks.',
  },
};
