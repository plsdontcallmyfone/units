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
      await button.hover();
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
    await button.hover();
    await expect(button).toHaveAttribute('aria-expanded', 'true');
    await page.keyboard.press('Escape');
    await expect(button).toHaveAttribute('aria-expanded', 'false');
  });

  test('clicking a menu pill leaves its menu open', async ({ page }) => {
    await page.goto('/projects');
    const button = page.getByRole('navigation', { name: 'Primary' }).getByRole('button', { name: 'Armory', exact: true });
    await button.click();
    await expect(button).toHaveAttribute('aria-expanded', 'true');
    // Pinned by the click: moving the pointer away keeps it open; a second click closes it.
    await page.mouse.move(5, 600);
    await page.waitForTimeout(400);
    await expect(button).toHaveAttribute('aria-expanded', 'true');
    await button.click();
    await expect(button).toHaveAttribute('aria-expanded', 'false');
  });

  test('the keyboard opens a menu, walks into it and Escape returns to the pill', async ({ page }) => {
    await page.goto('/projects');
    const button = page.getByRole('navigation', { name: 'Primary' }).getByRole('button', { name: 'War', exact: true });
    await button.focus();
    await expect(button).toHaveAttribute('aria-expanded', 'false');
    await page.keyboard.press('Enter');
    await expect(button).toHaveAttribute('aria-expanded', 'true');
    await page.keyboard.press('Tab');
    await expect(page.locator(`#${await button.getAttribute('aria-controls')} a`).first()).toBeFocused();
    await page.keyboard.press('Escape');
    await expect(button).toHaveAttribute('aria-expanded', 'false');
    await expect(button).toBeFocused();
    await page.keyboard.press('ArrowDown');
    await expect(button).toHaveAttribute('aria-expanded', 'true');
    await expect(page.locator(`#${await button.getAttribute('aria-controls')} a`).first()).toBeFocused();
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
    await page.goto('/projects');
    for (const href of ['/governance', '/war/coalitions', '/economy']) await expect(page.getByRole('navigation', { name: 'Primary' }).locator(`a[href="${href}"]`)).toHaveCount(1);
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
