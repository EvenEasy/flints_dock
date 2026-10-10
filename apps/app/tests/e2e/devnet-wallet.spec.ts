import { expect, test } from '@playwright/test';
import captured from '../fixtures/devnet-wallet.json' with { type: 'json' };
import type { PrepareCleanupRequest } from '../../frontend-contract/cleanup';

// Replay actual read-only IPC DTOs, not invented balances or category totals.
for (const viewport of [
  { width: 360, height: 800 },
  { width: 1440, height: 900 },
]) {
  test(`Devnet_RecordedInventoryAccountsSurviveDTOAndReact_${viewport.width}`, async ({ page }) => {
    await page.setViewportSize(viewport);
    await page.addInitScript((fixture) => {
      const calls: { command: string; payload: unknown }[] = [];
      Reflect.set(window, '__ipcCalls', calls);
      Reflect.set(window, '__TAURI_INTERNALS__', {
        transformCallback: () => 1,
        invoke: async (command: string, payload: unknown) => {
          calls.push({ command, payload });
          if (command === 'connect_wallet')
            return {
              walletAddress: fixture.wallet,
              sessionId: 'readonly-devnet-fixture',
              sourceKind: 'publicKey',
              canSign: false,
            };
          if (command === 'analyze_wallet') return fixture.analysis;
          if (command === 'prepare_cleanup') {
            const request = (payload as { request: PrepareCleanupRequest }).request;
            const source =
              request.policy === 'explicitDiscard' ? fixture.explicitDiscardPreview : fixture.auto;
            const selected = new Set(
              request.selection.mode === 'selected'
                ? request.selection.mints
                : request.selection.mode === 'all'
                  ? source.entries.map((e) => e.mint)
                  : [],
            );
            const entries = source.entries.map((entry) =>
              request.ignoredMints.includes(entry.mint) || !selected.has(entry.mint)
                ? {
                    ...entry,
                    action: 'skip',
                    reasonCode: request.ignoredMints.includes(entry.mint)
                      ? 'ignored_mint'
                      : 'not_selected',
                    reason: 'Protected by current selection',
                  }
                : entry,
            );
            return {
              ...source,
              entries,
              revision: request.revision,
              planId: `readonly-${request.revision}`,
              canExecute: false,
              executableAccounts: entries.filter((e) => e.action !== 'skip').length,
              skippedAccounts: entries.filter((e) => e.action === 'skip').length,
              burnCount: entries.filter((e) => e.action === 'burn').length,
              closeCount: entries.filter((e) => e.action !== 'skip').length,
            };
          }
          throw new Error(`Unexpected mutation: ${command}`);
        },
      });
    }, captured);
    await page.goto('/');
    await page.getByRole('button', { name: 'CONNECT WALLET' }).click();
    await page.getByRole('radio', { name: 'Public key (read only)' }).check();
    await page.getByRole('textbox', { name: 'WALLET PUBLIC KEY' }).fill(captured.wallet);
    await page.getByRole('button', { name: 'SCAN WALLET' }).click();
    await expect(page.getByRole('button', { name: /^NFT:/ })).toHaveAttribute(
      'aria-label',
      'NFT: —',
    );
    await page.getByRole('button', { name: /^NFT:/ }).click();
    await expect(page.getByRole('dialog')).toContainText('compressed NFT inventory unavailable');
    await page.getByRole('button', { name: 'Close dialog' }).click();
    await page.getByRole('button', { name: 'RECOVER SOL', exact: true }).click();
    const mints = new Set(captured.analysis.tokenAccounts.map((account) => account.mint));
    await expect(page.getByRole('checkbox')).toHaveCount(mints.size);
    await expect(
      page.getByText(`Executable accounts: ${captured.auto.executableAccounts}`, { exact: false }),
    ).toBeVisible();
    await expect(
      page.getByText(`Skipped accounts: ${captured.auto.skippedAccounts}`, { exact: false }),
    ).toBeVisible();
    await expect(
      page.getByText('Routing unavailable on this network', { exact: true }),
    ).toHaveCount(
      captured.auto.entries.filter((e) => e.reasonCode === 'routing_unavailable').length,
    );
    await page.getByLabel('Cleanup policy').selectOption('explicitDiscard');
    await expect(
      page.getByText(`Executable accounts: ${captured.explicitDiscardPreview.executableAccounts}`, {
        exact: false,
      }),
    ).toBeVisible();
    await expect(page.getByRole('button', { name: 'RECOVER SOL', exact: true })).toBeDisabled();
    await expect(page.getByRole('status')).toContainText('Read-only wallet');
    const last = page.getByRole('checkbox').last();
    await last.scrollIntoViewIfNeeded();
    await expect(last).toBeVisible();
    expect(
      await page.locator('main').evaluate((el) => el.scrollHeight - el.clientHeight),
    ).toBeLessThanOrEqual(1);
    const nav = await page.locator('.bottom-navigation').boundingBox();
    const action = await page
      .getByRole('button', { name: 'RECOVER SOL', exact: true })
      .boundingBox();
    expect(action!.y + action!.height).toBeLessThanOrEqual(nav!.y);
    await page.screenshot({ path: `.cache/desktop-cleanup/devnet-recorded-${viewport.width}.png` });
    expect(
      await page.evaluate(() =>
        (Reflect.get(window, '__ipcCalls') as { command: string }[]).some(
          (call) => call.command === 'execute_cleanup',
        ),
      ),
    ).toBe(false);
  });
}
