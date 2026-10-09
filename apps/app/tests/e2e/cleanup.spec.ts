import { expect, test } from '@playwright/test';
import type { Page } from '@playwright/test';
import { wallet, address, secondMint } from '../../src/test/fixtures';
import { cleanupPlan, cleanupJob } from '../../src/test/cleanup';
import type { CleanupProgress, PrepareCleanupRequest } from '../../frontend-contract/cleanup';

async function desktop(page: Page, partial = false) {
  const job = cleanupJob();
  if (partial) {
    job.status = 'partial';
    job.report!.failed = 1;
    job.report!.accounting_complete = false;
    job.report!.known_net_wallet_lamports = '-5000';
    job.report!.results[0]!.uncertain_signature = 'pending-signature';
  }
  await page.addInitScript(
    ({ analysis, plan, job }) => {
      const calls: { command: string; payload: unknown }[] = [];
      Reflect.set(window, '__ipcCalls', calls);
      Reflect.set(window, '__TAURI_INTERNALS__', {
        transformCallback: () => 1,
        invoke: async (command: string, payload: unknown) => {
          calls.push({ command, payload });
          if (command === 'connect_wallet')
            return {
              sessionId: 'session',
              walletAddress: analysis.owner,
              sourceKind: 'seed',
              canSign: true,
            };
          if (command === 'disconnect_wallet') return null;
          if (command === 'analyze_wallet') return analysis;
          if (command === 'prepare_cleanup') {
            const request = (payload as { request: PrepareCleanupRequest }).request;
            const mints = request.selection.mode === 'selected' ? request.selection.mints : [];
            const entries = plan.entries.filter((entry) => mints.includes(entry.mint));
            return {
              ...plan,
              revision: request.revision,
              planId: `plan-${request.revision}`,
              entries,
              canExecute: entries.length > 0,
              closeCount: entries.length,
              burnCount: entries.filter((e) => e.action === 'burn').length,
              requiresBurn: entries.some((e) => e.action === 'burn'),
            };
          }
          if (command === 'execute_cleanup') {
            const channel = (
              payload as { progress?: { onmessage: (event: CleanupProgress) => void } }
            ).progress;
            channel?.onmessage({
              jobId: 'job',
              sessionId: 'session',
              sequence: 1,
              stage: 'close',
              completed: 1,
              total: 2,
              operation: 'Close',
              account: 'source',
              status: 'running',
            });
            return new Promise((resolve) => {
              Reflect.set(window, '__finishCleanup', () => resolve(job));
            });
          }
          if (command === 'get_cleanup_job') return job;
          throw new Error(`Unexpected ${command}`);
        },
      });
    },
    { analysis: wallet(), plan: cleanupPlan(), job },
  );
  await page.goto('/');
  await page.getByRole('button', { name: 'CONNECT WALLET' }).click();
  await page.getByLabel('SEED BASE64').fill('AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=');
  await page.getByRole('button', { name: 'SCAN WALLET' }).click();
  await page.getByRole('button', { name: 'ПОВЕРНУТИ SOL', exact: true }).click();
  await expect(page.getByRole('checkbox')).toHaveCount(2);
  await expect(page.getByRole('button', { name: 'ПОВЕРНУТИ SOL', exact: true })).toBeEnabled();
}

test('Cleanup_AllUncheckedSendsExplicitNoneAndCannotExecute', async ({ page }) => {
  await desktop(page);
  await page.getByRole('checkbox', { name: `Include Same name mint ${address}` }).uncheck();
  await page.getByRole('checkbox', { name: `Include Same name mint ${secondMint}` }).uncheck();
  await expect(page.getByRole('button', { name: 'ПОВЕРНУТИ SOL', exact: true })).toBeDisabled();
  const calls = await page.evaluate(
    () =>
      Reflect.get(window, '__ipcCalls') as {
        command: string;
        payload: { request: PrepareCleanupRequest };
      }[],
  );
  expect(
    calls.filter((call) => call.command === 'prepare_cleanup').at(-1)!.payload.request.selection,
  ).toEqual({ mode: 'none' });
  expect(calls.some((call) => call.command === 'execute_cleanup')).toBe(false);
});
for (const viewport of [
  { width: 360, height: 800 },
  { width: 1440, height: 900 },
]) {
  test(`Cleanup_ApprovedRealProgressAndExactNet_${viewport.width}`, async ({ page }) => {
    await page.setViewportSize(viewport);
    await desktop(page);
    await page.getByRole('button', { name: 'ПОВЕРНУТИ SOL', exact: true }).click();
    await expect(page.getByRole('dialog')).toContainText('Цю дію неможливо скасувати');
    await page.getByRole('button', { name: 'ПОГОДЖУЮ СВАП, BURN ТА CLOSE' }).click();
    await expect(page.getByRole('heading', { name: 'ОЧИЩЕННЯ ГАМАНЦЯ' })).toBeVisible();
    await expect(page.getByRole('status')).toContainText('1 / 2 акаунтів · Закриття');
    await page.screenshot({ path: `.cache/desktop-cleanup/processing-${viewport.width}.png` });
    await page.evaluate(() => Reflect.get(window, '__finishCleanup')());
    await expect(
      page.getByRole('heading', { name: 'ОЧИЩЕННЯ ЗАВЕРШЕНО', exact: true }),
    ).toBeVisible();
    await expect(page.locator('.returned-sol .exact-amount')).toHaveAttribute(
      'title',
      '+9,007,199.258804553 SOL',
    );
    await expect(page.locator('.returned-sol')).not.toContainText('0.428');
    await page.locator('.cleanup-report summary').click();
    await expect(page.locator('.cleanup-report')).toContainText('fixture-signature');
    const nav = await page.locator('.bottom-navigation').boundingBox();
    const result = await page.locator('.cleanup-report').boundingBox();
    expect(result!.y + result!.height).toBeLessThanOrEqual(nav!.y);
    expect(
      await page.locator('main').evaluate((el) => el.scrollHeight - el.clientHeight),
    ).toBeLessThanOrEqual(1);
    await page.screenshot({ path: `.cache/desktop-cleanup/success-${viewport.width}.png` });
    expect(
      await page.evaluate(
        () =>
          (Reflect.get(window, '__ipcCalls') as { command: string }[]).filter(
            (call) => call.command === 'execute_cleanup',
          ).length,
      ),
    ).toBe(1);
  });
}
test('Cleanup_PartialAndMissingMetadataAreNeverFullSuccess', async ({ page }) => {
  await desktop(page, true);
  await page.getByRole('button', { name: 'ПОВЕРНУТИ SOL', exact: true }).click();
  await page.getByRole('button', { name: 'ПОГОДЖУЮ СВАП, BURN ТА CLOSE' }).click();
  await page.evaluate(() => Reflect.get(window, '__finishCleanup')());
  await expect(page.getByRole('heading', { name: 'ОЧИЩЕННЯ ЧАСТКОВЕ' })).toBeVisible();
  await expect(page.locator('.returned-sol')).toContainText('ОБЛІК НЕПОВНИЙ');
  await expect(page.locator('.returned-sol .exact-amount')).toHaveAttribute(
    'title',
    '-0.000005 SOL',
  );
  await expect(page.locator('.medallion-check')).toHaveCount(0);
});
