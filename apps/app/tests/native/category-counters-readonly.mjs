// Audit a running packaged WebKitGTK app through its loopback inspector.
// Run with WEBKIT_INSPECTOR_HTTP_SERVER=127.0.0.1:19223 and devnet RPC configured.
// This harness uses only public-key connection and scan commands, never cleanup.
import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';

const owner = '9FCR2PU1jZgCHyjWxzk2BNQHJxszAK24vBFiWmUyRNpv';
const endpoint = process.env.DOCK_NATIVE_INSPECTOR ?? 'http://127.0.0.1:19223';
const output = process.env.DOCK_NATIVE_REPORT ?? '/tmp/flints-category-native';
const keys = ['scam', 'nft', 'dust', 'dead_token'];
await mkdir(output, { recursive: true });
const listing = await (await fetch(endpoint)).text();
const socketPath = listing.match(/ws=([^"'<> ]+)/)?.[1] ?? '/socket/1/1/WebPage';
const ws = new WebSocket(`${endpoint.replace(/^http/, 'ws')}${socketPath}`);
let target;
let nextId = 1;
const pending = new Map();
ws.onmessage = ({ data }) => {
  const message = JSON.parse(data);
  if (message.method === 'Target.targetCreated') target = message.params.targetInfo.targetId;
  if (message.method === 'Target.dispatchMessageFromTarget') {
    const response = JSON.parse(message.params.message);
    const entry = pending.get(response.id);
    if (entry) {
      pending.delete(response.id);
      if (response.error) entry.reject(new Error(JSON.stringify(response.error)));
      else entry.resolve(response.result);
    }
  }
};
await new Promise((resolve, reject) => {
  ws.onopen = resolve;
  ws.onerror = reject;
});
const pause = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
async function until(check, label, timeout = 180_000) {
  const start = Date.now();
  while (Date.now() - start < timeout) {
    const value = await check();
    if (value) return value;
    await pause(200);
  }
  throw new Error(`Timed out: ${label}`);
}
await until(() => target, 'native inspector target', 5000);
function command(method, params = {}) {
  return new Promise((resolve, reject) => {
    const id = nextId++;
    const timer = setTimeout(() => {
      pending.delete(id);
      reject(new Error(`Inspector timeout: ${method}`));
    }, 20_000);
    pending.set(id, {
      resolve: (result) => {
        clearTimeout(timer);
        resolve(result);
      },
      reject: (error) => {
        clearTimeout(timer);
        reject(error);
      },
    });
    ws.send(
      JSON.stringify({
        id: nextId++,
        method: 'Target.sendMessageToTarget',
        params: { targetId: target, message: JSON.stringify({ id, method, params }) },
      }),
    );
  });
}
async function evaluate(expression) {
  const result = await command('Runtime.evaluate', { expression, returnByValue: true });
  if (result.wasThrown)
    throw new Error(`Native evaluation failed: ${JSON.stringify(result.result)}`);
  return result.result?.value;
}
async function click(label) {
  await evaluate(
    `(()=>{const button=[...document.querySelectorAll('button')].find(b=>b.textContent.trim()===${JSON.stringify(label)});if(!button)throw Error('Missing button');button.click();return true})()`,
  );
}
async function screenshot(name) {
  const viewport = await evaluate('({width:innerWidth,height:innerHeight})');
  const result = await command('Page.snapshotRect', {
    x: 0,
    y: 0,
    ...viewport,
    coordinateSystem: 'Viewport',
  });
  await writeFile(`${output}/${name}.png`, Buffer.from(result.dataURL.split(',')[1], 'base64'));
}
try {
  if (process.env.DOCK_NATIVE_REUSE_SCAN !== '1') {
    await until(() => evaluate("Boolean(document.querySelector('.page--welcome'))"), 'welcome');
    // Wrap the authentic registered IPC function to capture its DTO; never replace its results.
    await evaluate(
      `(()=>{const invoke=window.__TAURI_INTERNALS__.invoke.bind(window.__TAURI_INTERNALS__);window.__nativeCategoryAudit={calls:[],analysis:null};window.__TAURI_INTERNALS__.invoke=async(command,payload,options)=>{if(!['connect_wallet','analyze_wallet'].includes(command))throw Error('Unexpected command in scan-only audit: '+command);window.__nativeCategoryAudit.calls.push({command,payload});const result=await invoke(command,payload,options);if(command==='analyze_wallet')window.__nativeCategoryAudit.analysis=result;return result};return true})()`,
    );
    await click('CONNECT WALLET');
    await until(
      () => evaluate("Boolean(document.getElementById('wallet-identity'))"),
      'connection dialog',
    );
    await evaluate("document.querySelector('input[value=publicKey]').click();true");
    await pause(100);
    await evaluate(
      `(()=>{const input=document.getElementById('wallet-identity');Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(input,${JSON.stringify(owner)});input.dispatchEvent(new Event('input',{bubbles:true}));return true})()`,
    );
    await click('SCAN WALLET');
  }
  await until(async () => {
    const state = await evaluate(
      "({main:Boolean(document.querySelector('.page--main')),error:document.querySelector('[role=alert]')?.innerText})",
    );
    if (state.error && !state.main) throw new Error(`Live scan failed: ${state.error}`);
    return state.main;
  }, 'real analysis');
  await evaluate('document.fonts.ready.then(()=>true)');
  const raw = await evaluate('window.__nativeCategoryAudit');
  assert.equal(raw.analysis.owner, owner);
  assert.deepEqual(
    raw.calls.map((call) => call.command),
    ['connect_wallet', 'analyze_wallet'],
  );
  assert.equal(raw.calls[0].payload.request.source.kind, 'publicKey');
  const runtime = await evaluate(
    `({userAgent:navigator.userAgent,dpr:devicePixelRatio,viewport:[innerWidth,innerHeight],origin:location.origin,fontReady:document.fonts.status,mainOverflow:document.querySelector('main').scrollHeight-document.querySelector('main').clientHeight,counters:[...document.querySelectorAll('.category-card')].map(card=>{const number=card.querySelector('.type-numeric--count');const style=getComputedStyle(number);const rect=number.getBoundingClientRect();return {key:card.className.match(/category-card--(\\w+)/)[1],text:number.textContent,ariaLabel:card.getAttribute('aria-label'),font:style.font,lineHeight:style.lineHeight,textAlign:style.textAlign,height:rect.height,overflow:number.scrollWidth-number.clientWidth,tile:{width:card.clientWidth,height:card.clientHeight}}})})`,
  );
  for (const counter of runtime.counters) {
    assert.match(counter.text, /^(?:\d+|—)$/);
    const category = raw.analysis.categories?.categories[counter.key];
    const unique = new Set((category?.items ?? []).map((item) => item.id));
    const expected =
      unique.size > 0 ? String(unique.size) : category?.status.status === 'complete' ? '0' : '—';
    assert.equal(counter.text, expected);
    if (category) assert.equal(category.count, unique.size);
    assert.equal(counter.overflow, 0);
    assert.deepEqual(
      {
        font: counter.font,
        lineHeight: counter.lineHeight,
        textAlign: counter.textAlign,
        height: counter.height,
      },
      {
        font: runtime.counters[0].font,
        lineHeight: runtime.counters[0].lineHeight,
        textAlign: runtime.counters[0].textAlign,
        height: runtime.counters[0].height,
      },
    );
    assert.deepEqual(counter.tile, runtime.counters[0].tile);
  }
  assert.equal(runtime.mainOverflow, 0);
  await screenshot('main');
  const details = {};
  for (const key of keys) {
    await evaluate(`document.querySelector('.category-card--${key}').click();true`);
    await until(
      () => evaluate("Boolean(document.querySelector('dialog[open]'))"),
      `${key} details`,
    );
    details[key] = await evaluate(
      "({text:document.querySelector('dialog[open]').innerText,ids:[...document.querySelectorAll('.category-results > li > code')].map(e=>e.textContent)})",
    );
    const category = raw.analysis.categories?.categories[key];
    assert.deepEqual(
      new Set(details[key].ids),
      new Set((category?.items ?? []).map((item) => item.mint ?? item.id)),
    );
    await screenshot(`${key}-details`);
    await evaluate('document.querySelector(\'button[aria-label="Close dialog"]\').click();true');
  }
  await writeFile(
    `${output}/audit.json`,
    `${JSON.stringify({ owner, transactionsSubmitted: 0, authenticRegisteredIpc: true, raw, runtime, details }, null, 2)}\n`,
  );
  console.log(
    JSON.stringify({
      native: true,
      origin: runtime.origin,
      viewport: runtime.viewport,
      counters: runtime.counters.map(({ key, text }) => ({ key, text })),
      report: `${output}/audit.json`,
      transactionsSubmitted: 0,
    }),
  );
} finally {
  ws.close();
}
