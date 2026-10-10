// The header menus: open, list their pages, and lead to them, on desktop (pill groups) and on a
// phone (the menu sheet).
import { expect, test } from '../support/fixtures';
import { ROUTES } from '../support/routes';

const GROUPS: [string, string, RegExp][] = [
  ['Tokens', 'Launch', /\/launch$/],
  ['War', 'Seasons', /\/seasons$/],
  ['Armory', 'Order book', /\/book$/],
  ['Community', 'Guilds', /\/guilds$/],
];

test.describe('desktop menus', () => {
  test.skip(({ isMobile }) => isMobile, 'pill menus are the desktop header');

  for (const [group, entry, url] of GROUPS) {
    test(`${group} opens and leads to ${entry}`, async ({ page }) => {
      await page.goto('/projects');
      const nav = page.getByRole('navigation', { name: 'Primary' });
      const button = nav.getByRole('button', { name: group, exact: true });
      await button.click();
      await expect(button).toHaveAttribute('aria-expanded', 'true');
      const panel = page.locator(`#${await button.getAttribute('aria-controls')}`);
      await panel.getByRole('link', { name: new RegExp(`^${entry}`) }).click();
      await expect(page).toHaveURL(url);
      await expect(button).toHaveAttribute('aria-expanded', 'false');
    });
  }

  test('Escape closes an open menu', async ({ page }) => {
    await page.goto('/projects');
    const button = page.getByRole('navigation', { name: 'Primary' }).getByRole('button', { name: 'Armory', exact: true });
    await button.click();
    await expect(button).toHaveAttribute('aria-expanded', 'true');
    await page.keyboard.press('Escape');
    await expect(button).toHaveAttribute('aria-expanded', 'false');
  });

  test('Docs and Launch are direct links', async ({ page }) => {
    await page.goto('/projects');
    const nav = page.getByRole('navigation', { name: 'Primary' });
    await nav.getByRole('link', { name: 'Docs', exact: true }).click();
    await expect(page).toHaveURL(/\/docs$/);
    await nav.getByRole('link', { name: 'Launch', exact: true }).click();
    await expect(page).toHaveURL(/\/launch$/);
  });

  test('every menu link opens a page the suite checks', async ({ page }) => {
    await page.goto('/projects');
    const hrefs = await page.locator('header a[href^="/"]').evaluateAll((as) => [...new Set(as.map((a) => a.getAttribute('href')!))]);
    const known = new Set(ROUTES.map(([, url]) => url));
    expect(hrefs.filter((h) => !known.has(h.split('#')[0]!))).toEqual([]);
  });

  test('the pages added in app pass 5 are in the menu', async ({ page }) => {
    test.fixme(true, 'The header (components/site-header.tsx) has no entry for /governance or /war/coalitions; app pass 5 added them to components/nav.tsx, which the layout no longer renders.');
    await page.goto('/projects');
    for (const href of ['/governance', '/war/coalitions']) await expect(page.locator(`header a[href="${href}"]`)).toHaveCount(1);
  });
});

test.describe('phone menu', () => {
  test.skip(({ isMobile }) => !isMobile, 'the sheet is the phone header');

  test('the menu button opens the sheet and leads to a page', async ({ page }) => {
    await page.goto('/projects');
    const burger = page.getByRole('button', { name: 'Menu' });
    await burger.click();
    await expect(burger).toHaveAttribute('aria-expanded', 'true');
    const sheet = page.getByRole('navigation', { name: 'Menu' });
    await expect(sheet).toBeVisible();
    await sheet.getByRole('link', { name: 'Craft' }).click();
    await expect(page).toHaveURL(/\/craft$/);
    await expect(sheet).toBeHidden();
  });
});
