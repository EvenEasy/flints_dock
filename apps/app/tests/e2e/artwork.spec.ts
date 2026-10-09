import { expect, test } from '@playwright/test';
import { mkdir, writeFile } from 'node:fs/promises';
import type { Page } from '@playwright/test';
import { wallet, token, address } from '../../src/test/fixtures';
import { lamportsToSol } from '../../src/shared/format';

const viewports = [
  { width: 360, height: 800 },
  { width: 390, height: 844 },
  { width: 430, height: 932 },
  { width: 1440, height: 900 },
];
const screens = ['welcome', 'main', 'cleanup', 'scanning', 'success'];

/** Measure slots and text baselines relative to their component, independent of canvas scale. */
async function geometry(page: Page) {
  return page.evaluate(() => {
    const main = document.querySelector('main')!;
    const unit = parseFloat(getComputedStyle(main).getPropertyValue('--ui-unit'));
    function bounds(selector: string, root?: string) {
      const el = document.querySelector(selector);
      if (!el || el.getClientRects().length === 0) return null;
      const box = el.getBoundingClientRect();
      const origin = (root ? document.querySelector(root)! : el).getBoundingClientRect();
      return {
        x: (box.x - origin.x) / unit,
        y: (box.y - origin.y) / unit,
        width: box.width / unit,
        height: box.height / unit,
      };
    }
    function baseline(selector: string) {
      const el = document.querySelector(selector)!;
      const probe = document.createElement('span');
      probe.style.cssText = 'display:inline-block;width:0;height:0;padding:0;margin:0';
      el.append(probe);
      const y =
        (probe.getBoundingClientRect().y -
          document.querySelector('.brand')!.getBoundingClientRect().y) /
        unit;
      probe.remove();
      return y;
    }
    return {
      unit,
      brand: bounds('.brand'),
      emblem: bounds('.brand-emblem', '.brand'),
      copy: bounds('.brand-copy', '.brand'),
      sector: bounds('.type-sector', '.brand'),
      brandBaseline: baseline('.type-brand'),
      sectorBaseline: baseline('.type-sector'),
      title: bounds('.title-pedestal'),
      swords: bounds('.pedestal-anchor', '.title-pedestal'),
      hero: bounds('.hero-stage'),
      art: bounds('.hero-art', '.hero-stage'),
      variant: document.querySelector('[data-hero-variant]')?.getAttribute('data-hero-variant'),
      nav: bounds('.bottom-navigation'),
      icon: bounds('.bottom-navigation button > img'),
      navFont: document.querySelector('.type-navigation')
        ? parseFloat(getComputedStyle(document.querySelector('.type-navigation')!).fontSize) / unit
        : null,
      overflow: main.scrollHeight - main.clientHeight,
      pageWidthOverflow: document.documentElement.scrollWidth - innerWidth,
    };
  });
}

function close(actual: unknown, expected: unknown) {
  if (typeof actual === 'number' && typeof expected === 'number')
    expect(actual).toBeCloseTo(expected, 0);
  else if (actual && expected && typeof actual === 'object' && typeof expected === 'object') {
    for (const [key, value] of Object.entries(expected)) close(Reflect.get(actual, key), value);
  } else expect(actual).toEqual(expected);
}

for (const viewport of viewports) {
  for (const mode of ['phone', 'reference']) {
    test(`Artwork_${mode}_${viewport.width}x${viewport.height}_SharedGeometryAndScreens`, async ({
      page,
    }) => {
      await page.setViewportSize(viewport);
      const measurements: Record<string, Awaited<ReturnType<typeof geometry>>> = {};
      for (const screen of screens) {
        await page.goto(`/?preview=1${mode === 'reference' ? '&layout=reference' : ''}#${screen}`);
        await page.evaluate(() => document.fonts.ready);
        expect(
          await page.evaluate(() => document.fonts.check('800 22px "Dock Condensed"', 'ОЧИЩЕННЯ')),
        ).toBe(true);
        await expect(
          page.locator('.hero-ring, .hero-portrait, .hero-badge, .scene-planet, .scene-ships'),
        ).toHaveCount(0);
        await expect(page.locator('.hero-art')).toHaveCount(1);
        // ResizeObserver can select a different derivative after the fonts settle.
        await expect
          .poll(() =>
            page
              .locator('.hero-art')
              .evaluate(
                (el) =>
                  el instanceof HTMLImageElement &&
                  el.complete &&
                  el.naturalWidth >= el.getBoundingClientRect().width * devicePixelRatio,
              ),
          )
          .toBe(true);
        await expect
          .poll(() =>
            page
              .locator('.scene-art')
              .evaluate(
                (el) => el instanceof HTMLImageElement && el.complete && el.naturalWidth > 0,
              ),
          )
          .toBe(true);
        const measured = await geometry(page);
        if (mode === 'reference') {
          const canvas = await page.locator('.dock-screen').boundingBox();
          expect(canvas!.height).toBeCloseTo(
            canvas!.width * (screen === 'main' ? 1280 / 853 : 1.5),
            0,
          );
        }
        if (measured.nav) {
          expect(
            await page.locator('.bottom-navigation').evaluate((nav) => {
              const box = nav.getBoundingClientRect();
              return [...nav.querySelectorAll('button')].every((button) => {
                const child = button.getBoundingClientRect();
                return child.y >= box.y - 1 && child.bottom <= box.bottom + 1;
              });
            }),
          ).toBe(true);
        }
        measurements[screen] = measured;
        expect(measured.emblem!.y + measured.emblem!.height / 2).toBeCloseTo(
          measured.brand!.height / 2,
          0,
        );
        expect(measured.copy!.y + measured.copy!.height / 2).toBeCloseTo(
          measured.brand!.height / 2,
          0,
        );
        expect(measured.pageWidthOverflow).toBeLessThanOrEqual(1);
        if (mode === 'phone') expect(measured.overflow, screen).toBeLessThanOrEqual(1);
        expect(measured.hero!.width).toBeCloseTo(measured.hero!.height, 0);
        expect(measured.art!.width).toBeCloseTo(measured.hero!.width, 0);
        for (const property of [
          'brand',
          'emblem',
          'copy',
          'sector',
          'brandBaseline',
          'sectorBaseline',
        ] as const) {
          close(measured[property], measurements.welcome![property]);
        }
        if (screen === 'main') {
          close(measured.title, measurements.welcome!.title);
          close(measured.swords, measurements.welcome!.swords);
          // The whole decorative anchor stays above the live title words.
          const swords = await page.locator('.pedestal-anchor').boundingBox();
          const heading = await page.locator('.title-pedestal h1').boundingBox();
          if (swords) expect(swords.y + swords.height).toBeLessThanOrEqual(heading!.y);
        }
        if (['cleanup', 'success'].includes(screen)) {
          close(measured.nav, measurements.main!.nav);
          close(measured.icon, measurements.main!.icon);
          close(measured.navFont, measurements.main!.navFont);
        }
        if (viewport.width > 600 && mode === 'phone') {
          const box = await page.locator('.dock-screen').boundingBox();
          expect(box!.x + box!.width / 2).toBeCloseTo(viewport.width / 2, 0);
          expect(box!.y + box!.height / 2).toBeCloseTo(viewport.height / 2, 0);
          await expect(page.locator('.app-backdrop')).toHaveCount(1);
        }
        await page.screenshot({
          path: `.cache/art-review/${mode}-${screen}-${viewport.width}x${viewport.height}.png`,
          fullPage: true,
          animations: 'disabled',
        });
      }
      await mkdir('.cache/art-review', { recursive: true });
      await writeFile(
        `.cache/art-review/${mode}-${viewport.width}x${viewport.height}-geometry.json`,
        JSON.stringify(measurements, null, 2),
      );
    });
  }
}

/** Only IPC is mocked: the UI must preserve exact backend values and unknown classifications. */
async function mockSnapshot(page: Page, snapshot: ReturnType<typeof wallet>) {
  await page.addInitScript((analysis) => {
    Reflect.set(window, '__TAURI_INTERNALS__', {
      invoke: async (command: string) => {
        if (command === 'connect_wallet')
          return { walletAddress: analysis.owner, sourceKind: 'publicKey', canSign: false };
        if (command === 'analyze_wallet') return analysis;
        throw new Error('No mutation is supported in this test');
      },
    });
  }, snapshot);
  await page.goto('/');
  await page.getByRole('button', { name: 'CONNECT WALLET' }).click();
  await page.getByRole('radio', { name: 'Public key (read only)' }).check();
  await page.getByRole('textbox', { name: 'WALLET PUBLIC KEY' }).fill(address);
  await page.getByRole('button', { name: 'SCAN WALLET' }).click();
  await expect(page.getByRole('button', { name: 'ПОВЕРНУТИ SOL', exact: true })).toBeVisible();
}

test('LongExactValuesAndNames_WrapWithoutLossOrOverlap', async ({ page }) => {
  await page.setViewportSize({ width: 360, height: 800 });
  const asset = token();
  asset.metadata.name = 'Дуже довга назва активу з додатковим описом'.repeat(4);
  asset.metadata.symbol = 'VERY_LONG_SYMBOL_WITHOUT_BREAKS'.repeat(3);
  const snapshot = wallet({ tokens: { status: { status: 'complete' }, items: [asset] } });
  snapshot.accountSummary!.potentiallyReclaimableLamports = '18446744073709551615';
  await mockSnapshot(page, snapshot);
  const amount = lamportsToSol('18446744073709551615');
  await expect(page.locator('.exact-digits')).toHaveText(amount);
  await expect(page.locator('.exact-amount')).toHaveAttribute('title', `${amount} SOL`);
  expect(await page.locator('.scan-rent').evaluate((el) => el.scrollWidth <= el.clientWidth)).toBe(
    true,
  );
  const amountBox = await page.locator('.scan-rent .exact-amount').boundingBox();
  const stationBox = await page.locator('.orbital-station').boundingBox();
  const summaryBox = await page.locator('.scan-summary').boundingBox();
  expect(amountBox!.x + amountBox!.width).toBeLessThanOrEqual(summaryBox!.x + summaryBox!.width);
  if (stationBox) expect(amountBox!.x + amountBox!.width).toBeLessThanOrEqual(stationBox.x + 1);
  await page.getByRole('button', { name: 'ПОВЕРНУТИ SOL', exact: true }).click();
  await expect(page.locator('.asset-identity > strong')).toHaveText(asset.metadata.name!);
  await expect(page.locator('.asset-identity')).toHaveAttribute('title', asset.mint);
  await expect(page.getByText('МЕРТВИЙ', { exact: true })).toHaveCount(0);
  await expect(page.getByText('НЕ ОЦІНЕНО', { exact: true })).toHaveCount(2);
  await expect(page.getByRole('button', { name: 'ПОВЕРНУТИ SOL', exact: true })).toBeDisabled();
  expect(await page.locator('.asset-row').evaluate((el) => el.scrollWidth <= el.clientWidth)).toBe(
    true,
  );
  await page.getByRole('checkbox').uncheck();
  await expect(page.getByText('ЗАЛИШИТИ', { exact: true })).toBeVisible();
  expect(
    await page.locator('main').evaluate((el) => el.scrollHeight - el.clientHeight),
  ).toBeLessThanOrEqual(1);
  await page.screenshot({ path: '.cache/art-review/live-long-values.png' });
});

test('EmptyInventory_IsDistinctFromUnavailableEstimate', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await mockSnapshot(page, wallet({ tokens: { status: { status: 'complete' }, items: [] } }));
  await page.getByRole('button', { name: 'ПОВЕРНУТИ SOL', exact: true }).click();
  await expect(page.getByRole('checkbox')).toHaveCount(0);
  await expect(page.getByText('No token assets found in this category.')).toBeVisible();
  await expect(page.getByText('НЕ ОЦІНЕНО', { exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'ПОВЕРНУТИ SOL', exact: true })).toBeDisabled();
  expect(
    await page.locator('main').evaluate((el) => el.scrollHeight - el.clientHeight),
  ).toBeLessThanOrEqual(1);
  await page.screenshot({ path: '.cache/art-review/live-empty.png' });
});

test('LocalCyrillicFont_LoadingPreservesBrandAndNavigationSlots', async ({ page }) => {
  let release!: () => void;
  const pending = new Promise<void>((resolve) => {
    release = resolve;
  });
  await page.route('**/*.ttf', async (route) => {
    await pending;
    await route.continue();
  });
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('/?preview=1#main', { waitUntil: 'domcontentloaded' });
  await expect(page.locator('.brand')).toBeVisible();
  const before = await geometry(page);
  expect(await page.evaluate(() => document.fonts.status)).toBe('loading');
  expect(await page.locator('.type-brand').evaluate((el) => el.scrollWidth <= el.clientWidth)).toBe(
    true,
  );
  expect(
    await page.locator('.type-sector').evaluate((el) => el.scrollWidth <= el.clientWidth),
  ).toBe(true);
  release();
  await page.evaluate(() => document.fonts.ready);
  const after = await geometry(page);
  for (const property of ['brand', 'emblem', 'copy', 'sector', 'nav', 'icon'] as const)
    close(after[property], before[property]);
  expect(await page.locator('.type-brand').evaluate((el) => el.scrollWidth <= el.clientWidth)).toBe(
    true,
  );
  expect(
    await page.locator('.type-sector').evaluate((el) => el.scrollWidth <= el.clientWidth),
  ).toBe(true);
  expect(
    await page.evaluate(() => document.fonts.check('800 22px "Dock Condensed"', 'ЧИСТИМО ТРЮМИ')),
  ).toBe(true);
});

// The compact desktop composition must reserve enough space for the complete hero.
test('ShortDesktopHeroDoesNotExtendUnderTheTitle', async ({ page }) => {
  await page.setViewportSize({ width: 1100, height: 760 });
  await page.goto('/?preview=1#main');
  await page.evaluate(() => document.fonts.ready);
  const allocated = await page.locator('.main-hero').boundingBox();
  const hero = await page.locator('.hero-stage').boundingBox();
  const title = await page.locator('.title-pedestal').boundingBox();
  expect(hero!.height).toBeGreaterThan(60);
  expect(hero!.y + hero!.height).toBeLessThanOrEqual(allocated!.y + allocated!.height + 1);
  expect(hero!.y + hero!.height).toBeLessThanOrEqual(title!.y + 1);
});
