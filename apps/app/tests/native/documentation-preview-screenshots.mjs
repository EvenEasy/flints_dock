// Capture the explicit browser design preview. No wallet IPC or chain calls are made.
// Start npm run dev first, then run this file from apps/app.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { chromium } from '@playwright/test';

const base = process.env.DOCK_PREVIEW_URL ?? 'http://127.0.0.1:1420';
const output =
  process.env.DOCK_PREVIEW_SCREENSHOTS ??
  fileURLToPath(new URL('../../../../docs/screenshots/preview/', import.meta.url));
const screens = ['welcome', 'scanning', 'main', 'cleanup', 'processing', 'success'];
const captures = [];
await mkdir(output, { recursive: true });
const browser = await chromium.launch({ headless: true });
try {
  for (const width of [430, 360]) {
    const viewport = { width, height: Math.round((width * 844) / 390) };
    const page = await browser.newPage({ viewport, deviceScaleFactor: 2 });
    page.setDefaultTimeout(30_000);
    for (const screen of screens) {
      const url = new URL(base);
      url.search = `?preview=1&backdrop=0&width=${width}`;
      url.hash = screen;
      await page.goto(url.href);
      await page.evaluate(() => document.fonts.ready);
      await page.locator(`.dock-screen--${screen}`).waitFor();
      await page.getByRole('button', { name: 'DESIGN PREVIEW · NO TRANSACTIONS' }).waitFor();
      await page.waitForFunction(() =>
        [...document.images]
          .filter((image) => image.getClientRects().length > 0)
          .every((image) => image.complete && image.naturalWidth > 0),
      );
      const metrics = await page.evaluate(() => ({
        viewport: [innerWidth, innerHeight],
        dpr: devicePixelRatio,
        origin: location.origin,
        numericFields: [...document.querySelectorAll('.category-card .type-numeric--count')].map(
          (field) => field.textContent,
        ),
        horizontalOverflow: document.documentElement.scrollWidth - innerWidth,
      }));
      assert.equal(metrics.horizontalOverflow, 0);
      for (const field of metrics.numericFields) assert.match(field, /^(?:\d+|—)$/);
      const filename = `${screen}${width === 360 ? '-360' : ''}.png`;
      await page.screenshot({ path: `${output}/${filename}`, animations: 'disabled' });
      captures.push({ filename, screen, url: url.href, ...metrics });
    }
    await page.close();
  }
  const report = {
    mode: 'browser-design-preview',
    walletResults: false,
    transactionsSubmitted: 0,
    sampleAmounts: true,
    capturedAt: new Date().toISOString(),
    sourceRevision: execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim(),
    browserVersion: browser.version(),
    captures,
  };
  await writeFile(`${output}/manifest.json`, `${JSON.stringify(report, null, 2)}\n`);
  console.log(JSON.stringify({ mode: report.mode, output, captures: captures.length }));
} finally {
  await browser.close();
}
