import { expect, test } from '@playwright/test';
import type { Page } from '@playwright/test';
import type { AxeResults } from 'axe-core';
import { screens } from '../../src/app/navigation';
import { address, wallet, pricedWallet, token } from '../../src/test/fixtures';
import type { WalletAnalysis } from '../../frontend-contract/types';

async function accessibility(page: Page) {
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
}
async function mockDesktop(page: Page, snapshot: WalletAnalysis) {
  await page.addInitScript((analysis) => {
    Reflect.set(window, '__ipcCalls', []);
    Reflect.set(window, '__TAURI_INTERNALS__', {
      invoke: async (command: string, payload: unknown) => {
        (Reflect.get(window, '__ipcCalls') as unknown[]).push({ command, payload });
        if (command === 'connect_wallet')
          return { walletAddress: analysis.owner, sourceKind: 'publicKey', canSign: false };
        if (command === 'disconnect_wallet') return null;
        if (command !== 'analyze_wallet') throw new Error('Unexpected mutation');
        return analysis;
      },
    });
  }, snapshot);
}
async function connect(page: Page) {
  await page.getByRole('button', { name: 'CONNECT WALLET' }).click();
  await page.getByRole('radio', { name: 'Public key (read only)' }).check();
  await page.getByRole('textbox', { name: 'WALLET PUBLIC KEY' }).fill(address);
  await page.getByRole('button', { name: 'SCAN WALLET' }).click();
  await expect(page.getByRole('button', { name: 'ПОВЕРНУТИ SOL', exact: true })).toBeVisible();
}

// Every XML page is exercised in readable production mode, including a 320px narrow viewport.
for (const width of [320, 360, 390, 393, 430, 480, 700, 1280]) {
  for (const descriptor of screens) {
    test(`Preview_${descriptor.id}_${width}px_LayoutAndAccessibility`, async ({ page }) => {
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
      await page.setViewportSize({ width, height: 1000 });
      await page.goto(`/?preview=1&width=${Math.min(width, 480)}#${descriptor.id}`);
      await page.evaluate(() => document.fonts.ready);
      await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
      const box = await page.getByRole('region', { name: 'Wallet app screen' }).boundingBox();
      expect(box!.width).toBe(width <= 600 ? width : 480);
      expect(box!.x + box!.width / 2).toBeCloseTo(width / 2, 0);
      if (width > 600) expect(box!.y + box!.height / 2).toBeCloseTo(500, 0);
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
        true,
      );
      await expect(page.locator('.station-frame, .station-header, .app-context')).toHaveCount(0);
      await expect(page.getByText('DESIGN PREVIEW · NO TRANSACTIONS')).toBeVisible();
      const nav = page.getByRole('navigation', { name: 'Основна навігація' });
      if (['main', 'cleanup', 'success'].includes(descriptor.id)) {
        expect(await nav.getByRole('button').allTextContents()).toEqual([
          'СТАНЦІЯ',
          'АНГАР',
          'ТРЮМИ',
          'МІСІЇ',
          'ПРОФІЛЬ',
        ]);
        await expect(nav.getByRole('button', { name: 'СТАНЦІЯ' })).toHaveAttribute(
          'aria-current',
          'page',
        );
        await page.locator('main').evaluate((el) => (el.scrollTop = el.scrollHeight));
        if (descriptor.id !== 'success') {
          const action = page.getByRole('button', { name: 'ПОВЕРНУТИ SOL', exact: true });
          await expect(action).toBeVisible();
          const a = await action.boundingBox();
          const b = await nav.boundingBox();
          expect(a!.y + a!.height).toBeLessThanOrEqual(b!.y);
        }
      } else await expect(nav).toHaveCount(0);
      await accessibility(page);
      expect(errors).toEqual([]);
      expect(remote).toEqual([]);
      await page.screenshot({
        path: `.cache/redesign-screenshots/${descriptor.id}-${width}.png`,
        fullPage: true,
        animations: 'disabled',
      });
    });
  }
}

for (const descriptor of screens) {
  test(`XML_${descriptor.id}_ReferenceCanvasAndLiteralContent`, async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 1640 });
    await page.goto(`/?preview=1&layout=reference#${descriptor.id}`);
    await page.evaluate(() => document.fonts.ready);
    const canvas = page.getByRole('region', { name: 'Wallet app screen' });
    const box = await canvas.boundingBox();
    expect(box!.width).toBe(descriptor.id === 'main' ? 853 : 1024);
    expect(box!.height).toBe(descriptor.id === 'main' ? 1280 : 1536);
    if (descriptor.id === 'main') {
      await expect(page.getByText('142', { exact: true })).toBeVisible();
      await expect(page.getByText('0.052 SOL', { exact: true })).toBeVisible();
    }
    if (descriptor.id === 'scanning') {
      await expect(page.getByText('ЕТАП 3 ІЗ 5')).toBeVisible();
      await expect(page.getByRole('progressbar')).toHaveAttribute('aria-valuenow', '2');
      await expect(page.getByRole('listitem')).toHaveCount(5);
    }
    if (descriptor.id === 'cleanup') {
      await expect(page.getByRole('checkbox')).toHaveCount(6);
      await expect(page.getByText('4 АКТИВИ')).toBeVisible();
      await expect(page.getByText('≈ 0.428 SOL')).toBeVisible();
      await expect(page.getByText('МЕРТВИЙ', { exact: true })).toHaveCount(2);
    }
    if (descriptor.id === 'success') {
      await expect(page.getByText('+0.428 SOL', { exact: true })).toBeVisible();
      await expect(page.getByRole('heading', { name: 'ОЧИЩЕННЯ ЗАВЕРШЕНО' })).toBeVisible();
    }
    await canvas.screenshot({ path: `.cache/redesign-screenshots/reference-${descriptor.id}.png` });
  });
}

test('ConnectionDialog_EscapeAndKeyboardFocus', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'CONNECT WALLET' }).click();
  await expect(page.getByRole('dialog')).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'CONNECT WALLET' })).toBeFocused();
  await page.reload();
  await page.keyboard.press('Tab');
  await expect(page.getByRole('link', { name: 'Skip to content' })).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('main')).toBeFocused();
});

test('PreviewSelection_ClearMeansKeepAndEmptyDisablesCTA', async ({ page }) => {
  await page.goto('/?preview=1#cleanup');
  const checkbox = page.getByRole('checkbox', { name: /Include Bonk/ });
  await expect(checkbox).toBeChecked();
  await checkbox.uncheck();
  await expect(page.getByText('3 АКТИВІВ')).toBeVisible();
  await page.getByRole('button', { name: 'СТАНЦІЯ', exact: true }).click();
  await page.getByRole('button', { name: 'ПОВЕРНУТИ SOL', exact: true }).click();
  await expect(checkbox).not.toBeChecked();
  for (const box of await page.getByRole('checkbox').all()) await box.uncheck();
  await expect(page.getByRole('button', { name: 'ПОВЕРНУТИ SOL', exact: true })).toBeDisabled();
  await expect(page.getByText('≈ 0.428 SOL')).toHaveCount(0);
  await page.keyboard.press('Tab');
});

test('PreviewMenu_AllFiveScreensWithoutAutomaticCompletion', async ({ page }) => {
  await page.goto('/?preview=1#scanning');
  await page.waitForTimeout(1300);
  await expect(page).toHaveURL(/#scanning$/);
  await page.getByRole('button', { name: 'DESIGN PREVIEW · NO TRANSACTIONS' }).click();
  await expect(
    page.getByRole('navigation', { name: 'Station screens' }).getByRole('button'),
  ).toHaveCount(5);
  await page.getByRole('button', { name: 'Очищення', exact: true }).click();
  await page.getByRole('button', { name: 'ПОВЕРНУТИ SOL', exact: true }).click();
  await expect(page.getByText('+0.428 SOL', { exact: true })).toBeVisible();
});

test('DesktopContract_ConnectSelectPreservePricesAndDisconnect', async ({ page }) => {
  await mockDesktop(page, pricedWallet());
  await page.goto('/');
  await connect(page);
  await page.getByRole('button', { name: 'ПРОФІЛЬ' }).click();
  await expect(page.getByText('1 SOL = $120.50')).toBeVisible();
  await expect(page.getByText('≈ $1,487.65')).toBeVisible();
  await page.getByRole('button', { name: 'Close dialog' }).click();
  await page.getByRole('button', { name: 'ТРЮМИ' }).click();
  await expect(page.getByText('$1.23E-10')).toBeVisible();
  await expect(page.getByText('$1.0022')).toBeVisible();
  await expect(page.getByText('$10.02')).toBeVisible();
  await accessibility(page);
  await page.getByRole('button', { name: 'Close dialog' }).click();
  await page.getByRole('button', { name: 'ПОВЕРНУТИ SOL', exact: true }).click();
  await expect(page.getByRole('button', { name: /CLEANUP API REQUIRED/ })).toBeDisabled();
  await page.getByRole('checkbox', { name: /mint EPjFW/ }).uncheck();
  await expect(page.getByText('ЗАЛИШИТИ', { exact: true })).toBeVisible();
  await expect(page.getByText('МЕРТВИЙ')).toHaveCount(0);
  await page.getByRole('button', { name: 'ПРОФІЛЬ' }).click();
  await page.getByRole('button', { name: 'DISCONNECT WALLET' }).click();
  await expect(page.getByRole('button', { name: 'CONNECT WALLET' })).toBeVisible();
  const calls = (await page.evaluate(() => Reflect.get(window, '__ipcCalls'))) as {
    command: string;
    payload: unknown;
  }[];
  expect(calls.map((call) => call.command)).toEqual([
    'connect_wallet',
    'analyze_wallet',
    'disconnect_wallet',
  ]);
  expect(calls[1]!.payload).toEqual({
    request: {
      walletAddress: address,
      noPrices: false,
      selection: { balance: true, tokens: true, allTokens: true, nfts: true, cnfts: true },
    },
  });
});

test('DesktopSources_ValidationLabelsAndFocus', async ({ page }) => {
  await mockDesktop(page, wallet());
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('/');
  await page.getByRole('button', { name: 'CONNECT WALLET' }).click();
  for (const label of ['Seed (base64)', 'File (keypair)', 'Public key (read only)']) {
    await page.getByRole('radio', { name: label }).check();
    await page.getByRole('button', { name: 'SCAN WALLET' }).click();
    await expect(page.getByRole('alert')).toBeVisible();
    await accessibility(page);
  }
  expect(await page.evaluate(() => Reflect.get(window, '__ipcCalls'))).toEqual([]);
  await page.keyboard.press('Escape');
  await expect(page.getByRole('button', { name: 'CONNECT WALLET' })).toBeFocused();
});

test('PartialAndLongInventory_ScrollAndNavigationRemainUsable', async ({ page }) => {
  const snapshot = wallet({
    tokens: {
      status: { status: 'partial', reason: 'Some metadata unavailable.' },
      items: Array.from({ length: 150 }, (_, index) => token(`mint-${index}`)),
    },
  });
  await mockDesktop(page, snapshot);
  await page.setViewportSize({ width: 360, height: 844 });
  await page.goto('/');
  await connect(page);
  await page.getByRole('button', { name: 'ПОВЕРНУТИ SOL', exact: true }).click();
  await expect(page.getByText('Some metadata unavailable.')).toBeVisible();
  const last = page.getByRole('checkbox', { name: 'Include Same name mint mint-149' });
  await last.scrollIntoViewIfNeeded();
  await last.uncheck();
  await expect(last).not.toBeChecked();
  await page.locator('main').evaluate((el) => (el.scrollTop = el.scrollHeight));
  const cta = await page.getByRole('button', { name: /CLEANUP API REQUIRED/ }).boundingBox();
  const nav = await page.getByRole('navigation', { name: 'Основна навігація' }).boundingBox();
  expect(cta!.y + cta!.height).toBeLessThanOrEqual(nav!.y);
  await page.getByRole('button', { name: 'СТАНЦІЯ', exact: true }).click();
  await page.getByRole('button', { name: 'ТРЮМИ' }).click();
  await expect(page.getByText('PRICE UNAVAILABLE')).toHaveCount(50);
  await page.getByRole('button', { name: 'NEXT', exact: true }).click();
  await expect(page.getByText('mint-50', { exact: true })).toBeVisible();
  await accessibility(page);
  await page.screenshot({
    path: '.cache/redesign-screenshots/live-long-inventory.png',
    fullPage: true,
  });
});

for (const width of [360, 430]) {
  test(`Prices_${width}px_NarrowAndUnavailableQuote`, async ({ page }) => {
    await mockDesktop(page, pricedWallet());
    await page.setViewportSize({ width, height: 1000 });
    await page.goto('/');
    await connect(page);
    await page.getByRole('button', { name: 'ТРЮМИ' }).click();
    await expect(page.getByText('$1.23E-10')).toBeVisible();
    await accessibility(page);
    await page.screenshot({
      path: `.cache/redesign-screenshots/live-prices-${width}.png`,
      fullPage: true,
    });
  });
}

test('LiveHashes_CannotCreateScanOrCleanupResults', async ({ page }) => {
  await page.goto('/');
  await page.evaluate(() => {
    location.hash = 'scanning';
  });
  await expect(page.getByRole('button', { name: 'CONNECT WALLET' })).toBeVisible();
  await expect(page.getByText('ГАМАНЕЦЬ ПІДКЛЮЧЕНО', { exact: true })).toHaveCount(0);
  await page.evaluate(() => {
    location.hash = 'success';
  });
  await expect(page.getByText('ОЧИЩЕННЯ ЗАВЕРШЕНО', { exact: true })).toHaveCount(0);
});

test('DesktopFailure_RetryShowsRealPendingStateThenActualSnapshot', async ({ page }) => {
  await page.addInitScript((analysis) => {
    let calls = 0;
    Reflect.set(window, '__TAURI_INTERNALS__', {
      invoke: async (command: string) => {
        if (command === 'connect_wallet')
          return { walletAddress: analysis.owner, sourceKind: 'publicKey', canSign: false };
        if (command !== 'analyze_wallet') throw new Error('Unexpected command');
        if (++calls === 1)
          throw {
            code: 'invalid_configuration',
            message: 'RPC configuration is missing.',
            details: { secret: 'do-not-display' },
          };
        return new Promise((resolve) => {
          Reflect.set(window, '__resolveAnalysis', () => resolve(analysis));
        });
      },
    });
  }, wallet());
  await page.goto('/');
  await page.getByRole('button', { name: 'CONNECT WALLET' }).click();
  await page.getByRole('radio', { name: 'Public key (read only)' }).check();
  await page.getByRole('textbox', { name: 'WALLET PUBLIC KEY' }).fill(address);
  await page.getByRole('button', { name: 'SCAN WALLET' }).click();
  await expect(page.getByRole('alert')).toContainText('RPC configuration is missing.');
  await expect(page.getByText('do-not-display')).toHaveCount(0);
  await page.getByRole('button', { name: 'RETRY SCAN' }).click();
  await expect(page.getByRole('progressbar')).not.toHaveAttribute('aria-valuenow');
  await expect(page.getByText('ЕТАП 3 ІЗ 5')).toHaveCount(0);
  await page.evaluate(() => (Reflect.get(window, '__resolveAnalysis') as () => void)());
  await expect(page.getByText('АНАЛІЗ ЗАВЕРШЕНО', { exact: true })).toBeVisible();
});
