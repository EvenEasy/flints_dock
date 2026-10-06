import { expect, test } from '@playwright/test';
import type { AxeResults } from 'axe-core';
import { screens } from '../../src/app/navigation';
import { address, wallet } from '../../src/test/fixtures';

// Browser tests use the IPC boundary only; no RPC endpoint or wallet signer is contacted.
for (const width of [320, 390, 1280]) {
  for (const descriptor of screens) {
    test(`Preview_${descriptor.id}_${width}px_NoOverflowOrAccessibilityViolations`, async ({
      page,
    }) => {
      const errors: string[] = [];
      const remote: string[] = [];
      page.on('pageerror', (error) => errors.push(error.message));
      page.on('request', (request) => {
        if (
          !request.url().startsWith('http://127.0.0.1:1420/') &&
          !request.url().startsWith('data:')
        )
          remote.push(request.url());
      });
      await page.setViewportSize({ width, height: 960 });
      await page.goto(`/?preview=1#${descriptor.id}`);
      await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
      await page.evaluate(() => document.fonts.ready);
      await expect(page.getByText('REFERENCE PREVIEW · NO TRANSACTIONS')).toBeVisible();
      expect(
        await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth),
      ).toBe(true);
      await page.addScriptTag({ url: '/node_modules/axe-core/axe.min.js' });
      const result = await page.evaluate(async () => {
        const axe = Reflect.get(window, 'axe') as { run: (options: object) => Promise<AxeResults> };
        return axe.run({
          runOnly: { type: 'tag', values: ['wcag2a', 'wcag2aa', 'wcag21aa', 'wcag22aa'] },
        });
      });
      expect(
        result.violations.map((item) => ({
          id: item.id,
          nodes: item.nodes.map((node) => ({ target: node.target, message: node.failureSummary })),
        })),
      ).toEqual([]);
      expect(errors).toEqual([]);
      expect(remote).toEqual([]);
      await page.screenshot({
        path: `.cache/screenshots/${descriptor.id}-${width}.png`,
        fullPage: true,
        animations: 'disabled',
      });
    });
  }
}

test('WalletDialog_KeyboardEscape_RestoresFocus', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'CONNECT WALLET' }).click();
  await expect(page.getByRole('dialog', { name: 'DOCK A WALLET' })).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'CONNECT WALLET' })).toBeFocused();
});

test('DesktopTransport_PublicAddress_RendersContractAndNeverMutates', async ({ page }) => {
  await page.addInitScript((analysis) => {
    Reflect.set(window, 'isTauri', true);
    Reflect.set(window, '__ipcCalls', []);
    Reflect.set(window, '__TAURI_INTERNALS__', {
      invoke: async (command: string, payload: unknown) => {
        const calls = Reflect.get(window, '__ipcCalls') as { command: string; payload: unknown }[];
        calls.push({ command, payload });
        if (command !== 'analyze_wallet') throw new Error('Mutation attempted');
        return analysis;
      },
    });
  }, wallet());
  await page.goto('/');
  await page.getByRole('button', { name: 'CONNECT WALLET' }).click();
  await page.getByRole('textbox', { name: 'WALLET PUBLIC KEY' }).fill(address);
  await page.getByRole('button', { name: 'SCAN WALLET' }).click();
  await expect(page.getByRole('heading', { name: 'SCAN COMPLETE' })).toBeVisible();
  await expect(page.getByText('12.345678901 SOL')).toBeVisible();
  await page.getByRole('button', { name: 'REVIEW CLEANUP' }).click();
  await page.getByRole('button', { name: 'REVIEW REQUIRED CAPABILITIES' }).click();
  await expect(page.getByRole('button', { name: 'CLEANUP API REQUIRED' })).toBeDisabled();
  expect(await page.evaluate(() => Reflect.get(window, '__ipcCalls'))).toEqual([
    {
      command: 'analyze_wallet',
      payload: {
        request: {
          walletAddress: address,
          noPrices: true,
          selection: { balance: true, tokens: true, allTokens: true, nfts: true, cnfts: true },
        },
      },
    },
  ]);
});

test('SkipLink_InventoryScreen_FocusesContentWithoutChangingRoute', async ({ page }) => {
  await page.goto('/?preview=1#tokens');
  await page.keyboard.press('Tab');
  await expect(page.getByRole('link', { name: 'Skip to content' })).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('main')).toBeFocused();
  await expect(page.getByRole('heading', { name: 'SWAPPABLE TOKENS' })).toBeVisible();
  await expect(page).toHaveURL(/#tokens$/);
});

test('LiveInventory_PartialMetadata_NoAccessibilityViolations', async ({ page }) => {
  const snapshot = wallet();
  snapshot.tokens!.status = {
    status: 'partial',
    reason: 'Some account metadata could not be resolved.',
  };
  await page.addInitScript((analysis) => {
    Reflect.set(window, '__TAURI_INTERNALS__', { invoke: async () => analysis });
  }, snapshot);
  await page.setViewportSize({ width: 320, height: 960 });
  await page.goto('/');
  await page.getByRole('button', { name: 'CONNECT WALLET' }).click();
  await page.getByRole('textbox', { name: 'WALLET PUBLIC KEY' }).fill(address);
  await page.getByRole('button', { name: 'SCAN WALLET' }).click();
  await expect(page.getByRole('heading', { name: 'SCAN COMPLETE' })).toBeVisible();
  await page.getByRole('button', { name: 'Open navigation' }).click();
  await page.getByRole('button', { name: /Swappable tokens/ }).click();
  await expect(page.getByText('Some account metadata could not be resolved.')).toBeVisible();
  await page.addScriptTag({ url: '/node_modules/axe-core/axe.min.js' });
  const results = await page.evaluate(async () => {
    const axe = Reflect.get(window, 'axe') as { run: (options: object) => Promise<AxeResults> };
    return axe.run({
      runOnly: { type: 'tag', values: ['wcag2a', 'wcag2aa', 'wcag21aa', 'wcag22aa'] },
    });
  });
  expect(results.violations.map((violation) => violation.id)).toEqual([]);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
});
