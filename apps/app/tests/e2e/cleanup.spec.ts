import { expect, test } from '@playwright/test';
import type { Page } from '@playwright/test';
import { wallet, address, secondMint } from '../../src/test/fixtures';
import { cleanupPlan, cleanupJob } from '../../src/test/cleanup';
import type { CleanupProgress, PrepareCleanupRequest } from '../../frontend-contract/cleanup';

async function desktop(page: Page, partial = false, devnet = false) {
  const job = cleanupJob();
  if (partial) {
    job.status = 'partial';
    job.report!.failed = 1;
    job.report!.accounting_complete = false;
    job.report!.known_net_wallet_lamports = '-5000';
    job.report!.results[0]!.uncertain_signature = 'pending-signature';
  }
  const analysis = wallet();
  const plan = cleanupPlan();
  if (devnet) {
    const network = 'EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG';
    analysis.categories = { network, dustThresholdUsd: 0.01, categories: {}, providers: {} };
    plan.network = network;
  }
  await page.addInitScript(
    ({ analysis, plan, job, devnet }) => {
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
            const entries = plan.entries
              .filter((entry) => mints.includes(entry.mint))
              .map((entry) => {
                if (!devnet) return entry;
                return {
                  ...entry,
                  action: request.policy === 'explicitDiscard' ? 'burn' : 'skip',
                  reasonCode:
                    request.policy === 'explicitDiscard'
                      ? 'explicit_discard'
                      : 'routing_unavailable',
                  reason:
                    request.policy === 'explicitDiscard'
                      ? 'Explicit discard; irreversible burn then close'
                      : 'Routing unavailable on this network',
                  expectedOutLamports: null,
                  minOutLamports: null,
                };
              });
            const executable = entries.filter((entry) => entry.action !== 'skip').length;
            return {
              ...plan,
              policy: request.policy ?? 'auto',
              revision: request.revision,
              planId: `plan-${request.revision}`,
              entries,
              canExecute: executable > 0,
              selectedAssets: mints.length,
              executableAccounts: executable,
              skippedAccounts: entries.length - executable,
              swapCount: entries.filter((e) => e.action === 'swap').length,
              estimatedSwapLamports: devnet ? '0' : plan.estimatedSwapLamports,
              estimatedReclaimedLamports: (BigInt(executable) * 2039280n).toString(),
              closeCount: executable,
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
              stage: devnet ? 'burn' : 'close',
              completed: 1,
              total: 2,
              operation: devnet ? 'Burn' : 'Close',
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
    { analysis, plan, job, devnet },
  );
  await page.goto('/');
  await page.getByRole('button', { name: 'CONNECT WALLET' }).click();
  await page.getByLabel('SEED BASE64').fill('AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=');
  await page.getByRole('button', { name: 'SCAN WALLET' }).click();
  await page.getByRole('button', { name: 'RECOVER SOL', exact: true }).click();
  await expect(page.getByRole('checkbox')).toHaveCount(2);
  if (!devnet)
    await expect(page.getByRole('button', { name: 'RECOVER SOL', exact: true })).toBeEnabled();
}

test('Cleanup_AllUncheckedSendsExplicitNoneAndCannotExecute', async ({ page }) => {
  await desktop(page);
  await page.getByRole('checkbox', { name: `Include Same name mint ${address}` }).uncheck();
  await page.getByRole('checkbox', { name: `Include Same name mint ${secondMint}` }).uncheck();
  await expect(page.getByRole('button', { name: 'RECOVER SOL', exact: true })).toBeDisabled();
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
    await page.getByRole('button', { name: 'RECOVER SOL', exact: true }).click();
    await expect(page.getByRole('dialog')).toContainText('This cannot be undone');
    await page.getByRole('button', { name: 'APPROVE SWAP, BURN AND CLOSE' }).click();
    await expect(page.getByRole('heading', { name: 'WALLET CLEANUP' })).toBeVisible();
    await expect(page.getByRole('status')).toContainText('1 / 2 accounts · Closing');
    await page.screenshot({ path: `.cache/desktop-cleanup/processing-${viewport.width}.png` });
    await page.evaluate(() => Reflect.get(window, '__finishCleanup')());
    await expect(
      page.getByRole('heading', { name: 'CLEANUP COMPLETE', exact: true }),
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
  await page.getByRole('button', { name: 'RECOVER SOL', exact: true }).click();
  await page.getByRole('button', { name: 'APPROVE SWAP, BURN AND CLOSE' }).click();
  await page.evaluate(() => Reflect.get(window, '__finishCleanup')());
  await expect(page.getByRole('heading', { name: 'PARTIAL CLEANUP' })).toBeVisible();
  await expect(page.locator('.returned-sol')).toContainText('ACCOUNTING INCOMPLETE');
  await expect(page.locator('.returned-sol .exact-amount')).toHaveAttribute(
    'title',
    '-0.000005 SOL',
  );
  await expect(page.locator('.medallion-check')).toHaveCount(0);
});

for (const viewport of [
  { width: 360, height: 800 },
  { width: 1440, height: 900 },
]) {
  test(`Devnet_DiscardRequiresExplicitPolicyAndBurnApproval_${viewport.width}`, async ({
    page,
  }) => {
    await page.setViewportSize(viewport);
    await desktop(page, false, true);
    await expect(page.getByRole('button', { name: 'RECOVER SOL', exact: true })).toBeDisabled();
    await expect(
      page.getByText('Routing unavailable on this network', { exact: true }),
    ).toHaveCount(2);
    await expect(page.getByText('Executable accounts: 0', { exact: false })).toBeVisible();
    await expect(page.locator('.asset-valuation > strong')).toHaveText([
      'NOT ESTIMATED',
      'NOT ESTIMATED',
    ]);
    await page.getByLabel('Cleanup policy').selectOption('explicitDiscard');
    await expect(page.getByRole('button', { name: 'RECOVER SOL', exact: true })).toBeEnabled();
    await expect(page.getByText('Burn: 2', { exact: false })).toBeVisible();
    await expect(page.locator('.asset-valuation > span')).toHaveText(['BURN', 'BURN']);
    const callCount = await page.evaluate(
      () =>
        (Reflect.get(window, '__ipcCalls') as { command: string }[]).filter(
          (call) => call.command === 'execute_cleanup',
        ).length,
    );
    expect(callCount).toBe(0);
    expect(
      await page.locator('main').evaluate((el) => el.scrollHeight - el.clientHeight),
    ).toBeLessThanOrEqual(1);
    const nav = await page.locator('.bottom-navigation').boundingBox();
    const action = await page
      .getByRole('button', { name: 'RECOVER SOL', exact: true })
      .boundingBox();
    expect(action!.y + action!.height).toBeLessThanOrEqual(nav!.y);
    await page.screenshot({ path: `.cache/desktop-cleanup/devnet-plan-${viewport.width}.png` });
    await page.getByRole('button', { name: 'RECOVER SOL', exact: true }).click();
    await expect(page.getByRole('dialog')).toContainText(
      'No swaps. Only account rent can be recovered.',
    );
    await expect(page.getByRole('dialog')).toContainText(
      'Burn the full balance of 2 accounts. This cannot be undone.',
    );
    await page.getByRole('button', { name: 'APPROVE SWAP, BURN AND CLOSE' }).click();
    await expect(page.getByRole('status')).toContainText('Burning');
    await page.screenshot({ path: `.cache/desktop-cleanup/devnet-discard-${viewport.width}.png` });
    await page.evaluate(() => Reflect.get(window, '__finishCleanup')());
    await expect(
      page.getByRole('heading', { name: 'CLEANUP COMPLETE', exact: true }),
    ).toBeVisible();
  });
}
