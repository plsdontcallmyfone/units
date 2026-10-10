// Known app bugs the suite found. Each entry turns the route check into test.fixme for the listed
// projects, with the reason shown in the report; README.md "Known failures" lists the same. Remove
// an entry when the page is fixed (the test then has to pass).
export const KNOWN: Record<string, { projects: string[]; reason: string }> = {};
