import { expect, test } from '@playwright/test';
import type { Page } from '@playwright/test';
import type { AssetCategory, CategoryItem } from '../../frontend-contract/categories';
import type { ScanStatus, WalletAnalysis } from '../../frontend-contract/types';
import { address, secondMint, wallet } from '../../src/test/fixtures';

// These browser regressions replay controlled IPC fixtures; they are not live wallet scans.
const keys = ['scam', 'nft', 'dust', 'dead_token'] as const;
const labels = ['SCAM', 'NFT', 'DUST', 'DEAD TOKEN'] as const;
function item(key: string, index: number): CategoryItem {
  const id = `${key}-${index}`;
  return {
    id,
    mint: id,
    program: 'legacy',
    name: `${key} asset ${index}`,
    kind: key === 'nft' ? 'nonFungible' : 'fungible',
    accounts: [`account-${id}`],
    rawAmount: '1',
    risk:
      key === 'scam'
        ? {
            status: 'suspicious',
            source: 'E2E fixture',
            reasons: ['Confirmed test signal'],
            checkedAt: '1',
          }
        : null,
    valuation: key === 'dust' ? 'priced' : 'unavailable',
    valueUsd: key === 'dust' ? 0.01 : null,
    tradability: key === 'dead_token' ? 'noRoute' : 'unknown',
    evidence: `Confirmed ${key} fixture evidence`,
    checkedAt: '1',
    providerScope: 'E2E fixture only',
  };
}
function category(
  key: string,
  count: number,
  status: ScanStatus = { status: 'complete' },
): AssetCategory {
  return {
    items: Array.from({ length: count }, (_, index) => item(key, index)),
    count,
    status,
    checkedAt: '1',
    coverage: { inventory: { status: 'complete' } },
  };
}
function snapshot(results: Record<string, AssetCategory> = {}): WalletAnalysis {
  return wallet({
    nfts: null,
    cnfts: null,
    categories: {
      network: 'test fixture',
      dustThresholdUsd: 0.1,
      categories: results,
      providers: {},
    },
  });
}
async function mockDesktop(page: Page, analysis: WalletAnalysis) {
  await page.addInitScript((fixture) => {
    Reflect.set(window, '__ipcCalls', []);
    Reflect.set(window, '__categoryFixture', fixture);
    Reflect.set(window, '__TAURI_INTERNALS__', {
      invoke: async (command: string, payload: unknown) => {
        (Reflect.get(window, '__ipcCalls') as unknown[]).push({ command, payload });
        const current = Reflect.get(window, '__categoryFixture') as WalletAnalysis;
        if (command === 'connect_wallet')
          return { walletAddress: current.owner, sourceKind: 'publicKey', canSign: false };
        if (command === 'analyze_wallet') return current;
        throw new Error(`Unexpected command: ${command}`);
      },
    });
  }, analysis);
}
async function scan(page: Page, owner = address) {
  await page.getByRole('radio', { name: 'Public key (read only)' }).check();
  await page.getByRole('textbox', { name: 'WALLET PUBLIC KEY' }).fill(owner);
  await page.getByRole('button', { name: 'SCAN WALLET' }).click();
}
async function connect(page: Page) {
  await page.goto('/');
  await page.getByRole('button', { name: 'CONNECT WALLET' }).click();
  await scan(page);
  await expect(page.locator('.category-card')).toHaveCount(4);
}
async function expectCounters(page: Page, values: string[]) {
  const numbers = page.locator('.category-card .type-numeric--count');
  await expect(numbers).toHaveText(values);
  for (const number of await numbers.all()) await expect(number).toHaveText(/^(?:\d+|—)$/);
  for (let index = 0; index < labels.length; index++)
    await expect(page.locator(`.category-card--${keys[index]}`)).toHaveAttribute(
      'aria-label',
      `${labels[index]}: ${values[index]}`,
    );
}

for (const viewport of [
  { width: 360, height: 640 },
  { width: 1440, height: 900 },
]) {
  test(`Categories_CompleteNumbersMatchDetails_${viewport.width}px`, async ({ page }) => {
    const results = Object.fromEntries(keys.map((key, index) => [key, category(key, index + 1)]));
    await mockDesktop(page, snapshot(results));
    await page.setViewportSize(viewport);
    await connect(page);
    await expectCounters(page, ['1', '2', '3', '4']);
    for (const key of keys) {
      await page.locator(`.category-card--${key}`).click();
      await expect(page.locator('.category-results > li')).toHaveCount(results[key]!.items.length);
      await expect(page.locator('.category-results > li > code')).toHaveText(
        results[key]!.items.map((asset) => asset.mint!),
      );
      await page.getByRole('button', { name: 'Close dialog' }).click();
    }
    await page.screenshot({ path: `.cache/category-counters/complete-${viewport.width}.png` });
  });

  test(`Categories_MixedStatusesKeepNumericTypographyAndTileDimensions_${viewport.width}px`, async ({
    page,
  }) => {
    await mockDesktop(
      page,
      snapshot({
        scam: category('scam', 1, { status: 'partial', reason: 'Risk provider rate limited.' }),
        nft: category('nft', 0),
        dust: category('dust', 0, { status: 'failed', reason: 'Pricing authorization failed.' }),
        dead_token: category('dead_token', 0, {
          status: 'unsupported',
          reason: 'Routes unavailable for devnet.',
        }),
      }),
    );
    await page.setViewportSize(viewport);
    await connect(page);
    await expectCounters(page, ['1', '0', '—', '—']);
    const dimensions = await page
      .locator('.category-card .type-numeric--count')
      .evaluateAll((elements) =>
        elements.map((element) => {
          const style = getComputedStyle(element);
          const box = element.getBoundingClientRect();
          return {
            font: style.font,
            lineHeight: style.lineHeight,
            textAlign: style.textAlign,
            height: box.height,
          };
        }),
      );
    expect(
      dimensions.every((dimension) => JSON.stringify(dimension) === JSON.stringify(dimensions[0])),
    ).toBe(true);
    const cards = await page
      .locator('.category-card')
      .evaluateAll((elements) =>
        elements.map((element) => ({ width: element.clientWidth, height: element.clientHeight })),
      );
    expect(cards.every((card) => JSON.stringify(card) === JSON.stringify(cards[0]))).toBe(true);
    await page.locator('.category-card--scam').click();
    await expect(page.getByRole('dialog')).toContainText('Risk provider rate limited.');
    await page.getByRole('button', { name: 'Close dialog' }).click();
    await page.screenshot({ path: `.cache/category-counters/mixed-${viewport.width}.png` });
  });
}

test('Categories_CompleteZeroPreservedForEveryCategory', async ({ page }) => {
  await mockDesktop(page, snapshot(Object.fromEntries(keys.map((key) => [key, category(key, 0)]))));
  await connect(page);
  await expectCounters(page, ['0', '0', '0', '0']);
  for (const key of keys) {
    await page.locator(`.category-card--${key}`).click();
    await expect(page.locator('.category-results > li')).toHaveCount(0);
    await expect(page.getByRole('dialog')).toContainText('No assets found in this category.');
    await page.getByRole('button', { name: 'Close dialog' }).click();
  }
});

for (const status of ['unsupported', 'failed', 'skipped', 'partial'] as const) {
  test(`Categories_${status}WithoutConfirmedItemsShowsDash`, async ({ page }) => {
    const reason = `${status} provider reason`;
    await mockDesktop(
      page,
      snapshot(Object.fromEntries(keys.map((key) => [key, category(key, 0, { status, reason })]))),
    );
    await connect(page);
    await expectCounters(page, ['—', '—', '—', '—']);
    await page.locator('.category-card--dust').click();
    await expect(page.getByRole('dialog')).toContainText(reason);
    await expect(page.getByText('No assets found in this category.')).toHaveCount(0);
  });
}

test('Categories_MissingResultsNeverUseDesignPreviewTotals', async ({ page }) => {
  await mockDesktop(page, snapshot());
  await connect(page);
  await expectCounters(page, ['—', '—', '—', '—']);
});

test('Categories_PartialAndFailedRetainOnlyUniqueConfirmedItemsInTileAndDetails', async ({
  page,
}) => {
  const partial = category('scam', 2, { status: 'partial', reason: 'Risk coverage incomplete.' });
  partial.items.push({ ...partial.items[0]!, accounts: ['second-account-for-same-holding'] });
  partial.count = 3;
  const retained = category('dust', 1, {
    status: 'failed',
    reason: 'Additional prices unavailable.',
  });
  await mockDesktop(page, snapshot({ scam: partial, dust: retained }));
  await connect(page);
  await expectCounters(page, ['2', '—', '1', '—']);
  await page.locator('.category-card--scam').click();
  await expect(page.locator('.category-results > li')).toHaveCount(2);
  await expect(page.locator('.category-results > li > code')).toHaveText(['scam-0', 'scam-1']);
});

test('Categories_NftCompatibilityFallbackDeduplicatesClassicCoreIdsWithoutDas', async ({
  page,
}) => {
  const fixture = snapshot();
  fixture.categories = null;
  fixture.nfts = {
    status: { status: 'complete' },
    classic: {
      status: { status: 'complete' },
      items: [
        {
          mint: 'classic-id',
          tokenAccounts: ['account-a'],
          metadata: {
            name: 'Classic NFT',
            symbol: null,
            uri: null,
            imageUri: null,
            tokenStandard: 'nonFungible',
            source: 'Metaplex',
            collection: null,
          },
          programmable: false,
          edition: false,
          evidence: 'Verified standard and ownership',
        },
        {
          mint: 'classic-id',
          tokenAccounts: ['account-b'],
          metadata: {
            name: 'Classic NFT',
            symbol: null,
            uri: null,
            imageUri: null,
            tokenStandard: 'nonFungible',
            source: 'Metaplex',
            collection: null,
          },
          programmable: false,
          edition: false,
          evidence: 'Verified standard and ownership',
        },
      ],
    },
    core: {
      status: { status: 'complete' },
      items: [
        {
          address: 'core-id',
          owner: address,
          name: 'Core NFT',
          uri: '',
          lamports: '1',
          dataLen: '1',
          updateAuthority: address,
          collection: null,
          pluginsStatus: { status: 'complete' },
        },
      ],
    },
  };
  fixture.cnfts = {
    status: { status: 'unsupported', reason: 'DAS provider unavailable.' },
    items: null,
  };
  await mockDesktop(page, fixture);
  await connect(page);
  await expectCounters(page, ['—', '2', '—', '—']);
  await page.locator('.category-card--nft').click();
  await expect(page.locator('.category-results > li')).toHaveCount(2);
  await expect(page.getByRole('dialog')).toContainText('Classic NFT');
  await expect(page.getByRole('dialog')).toContainText('Core NFT');
  await expect(page.getByRole('dialog')).toContainText('DAS provider unavailable.');
});

test('Categories_NftFallbackPreservesConfirmedCompressedIdsAndAuthoritativeCategory', async ({
  page,
}) => {
  const fixture = snapshot();
  fixture.categories = null;
  fixture.cnfts = {
    status: { status: 'partial', reason: 'DAS page two timed out.' },
    items: [
      { id: 'compressed-id', name: 'Verified compressed NFT' },
      { id: 'compressed-id', name: 'Verified compressed NFT' },
    ],
  };
  await mockDesktop(page, fixture);
  await connect(page);
  await expectCounters(page, ['—', '1', '—', '—']);
  await page.locator('.category-card--nft').click();
  await expect(page.locator('.category-results > li')).toHaveCount(1);
  await expect(page.getByRole('dialog')).toContainText('Verified compressed NFT');
  await expect(page.getByRole('dialog')).toContainText('DAS page two timed out.');
  await page.getByRole('button', { name: 'Close dialog' }).click();

  fixture.categories = {
    network: 'test fixture',
    dustThresholdUsd: 0.1,
    categories: { nft: category('nft', 2) },
    providers: {},
  };
  await page.evaluate((analysis) => Reflect.set(window, '__categoryFixture', analysis), fixture);
  await page.getByRole('button', { name: 'PROFILE' }).click();
  await page.getByRole('button', { name: 'CHANGE WALLET / RESCAN' }).click();
  await scan(page);
  await expectCounters(page, ['—', '2', '—', '—']);
  await page.locator('.category-card--nft').click();
  await expect(page.locator('.category-results > li > code')).toHaveText(['nft-0', 'nft-1']);
  await expect(page.getByRole('dialog')).not.toContainText('Verified compressed NFT');
});

test('Categories_RescanAndChangedWalletIgnoreDismissedLateResults', async ({ page }) => {
  const first = snapshot(Object.fromEntries(keys.map((key) => [key, category(key, 1)])));
  const next = snapshot(Object.fromEntries(keys.map((key) => [key, category(key, 2)])));
  next.owner = secondMint;
  await page.addInitScript(
    ({ first, next }) => {
      let call = 0;
      const pending: Array<() => void> = [];
      Reflect.set(window, '__resolveScans', pending);
      Reflect.set(window, '__TAURI_INTERNALS__', {
        invoke: async (command: string, payload: unknown) => {
          if (command === 'connect_wallet')
            return {
              walletAddress: (payload as { request: { source: { address: string } } }).request
                .source.address,
              sourceKind: 'publicKey',
              canSign: false,
            };
          if (command !== 'analyze_wallet') throw new Error(`Unexpected mutation: ${command}`);
          if (call++ === 0) return first;
          const result = call === 2 ? first : next;
          return new Promise((resolve) => pending.push(() => resolve(result)));
        },
      });
    },
    { first, next },
  );
  await connect(page);
  await expectCounters(page, ['1', '1', '1', '1']);
  await page.getByRole('button', { name: 'PROFILE' }).click();
  await page.getByRole('button', { name: 'CHANGE WALLET / RESCAN' }).click();
  await scan(page);
  await expect(page.getByRole('status')).toContainText('FETCHING DATA');
  await expect(page.locator('.category-card')).toHaveCount(0);
  await page.getByRole('button', { name: 'DISMISS SCAN' }).click();
  await page.getByRole('button', { name: 'CONNECT WALLET', exact: true }).click();
  await scan(page, secondMint);
  await expect(page.locator('.category-card')).toHaveCount(0);
  await page.evaluate(() => (Reflect.get(window, '__resolveScans') as Array<() => void>)[1]!());
  await expectCounters(page, ['2', '2', '2', '2']);
  await page.evaluate(() => (Reflect.get(window, '__resolveScans') as Array<() => void>)[0]!());
  await page.evaluate(
    () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))),
  );
  await expectCounters(page, ['2', '2', '2', '2']);
  await page.locator('.category-card--nft').click();
  await expect(page.locator('.category-results > li')).toHaveCount(2);
});
