// Captures the Web UI screenshot used on the home page. Called by
// scripts/render-website-examples.sh while `typrest serve` runs the invoice example.
//
// Usage: node capture-webui.mjs <chrome-binary> <webui-url> <payload.json> <output.png>
//
// Drives headless Chrome over the DevTools protocol: opens the URL, enters the sample payload
// through the Raw JSON tab, returns to the form, clicks Render (the Web UI never renders on its
// own, REQ-WEB-025), waits for the preview, and saves a PNG.
import { spawn } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';

const [chromeBin, url, payloadFile, output] = process.argv.slice(2);
if (!chromeBin || !url || !payloadFile || !output) {
  console.error('usage: capture-webui.mjs <chrome-binary> <webui-url> <payload.json> <output.png>');
  process.exit(2);
}
const payload = readFileSync(payloadFile, 'utf8');

const WIDTH = 1440;
const HEIGHT = 900;
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

const profile = mkdtempSync(path.join(tmpdir(), 'typrest-chrome-'));
const chrome = spawn(
  chromeBin,
  [
    '--headless=new',
    '--disable-gpu',
    '--hide-scrollbars',
    '--no-first-run',
    '--no-default-browser-check',
    '--remote-debugging-port=0',
    `--user-data-dir=${profile}`,
    `--window-size=${WIDTH},${HEIGHT}`,
    'about:blank',
  ],
  { stdio: ['ignore', 'ignore', 'pipe'] }
);

const browserUrl = await new Promise((resolve, reject) => {
  let stderr = '';
  chrome.stderr.on('data', (chunk) => {
    stderr += chunk;
    const match = /DevTools listening on (ws:\/\/\S+)/.exec(stderr);
    if (match) resolve(match[1]);
  });
  chrome.on('exit', (code) => reject(new Error(`Chrome exited with ${code}:\n${stderr}`)));
});

const { port } = new URL(browserUrl);
const targets = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
const page = targets.find((target) => target.type === 'page');

const socket = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((resolve) => socket.addEventListener('open', resolve, { once: true }));

let nextId = 0;
const pending = new Map();
const listeners = new Map();
socket.addEventListener('message', ({ data }) => {
  const message = JSON.parse(data);
  if (message.id !== undefined) {
    const { resolve, reject } = pending.get(message.id);
    pending.delete(message.id);
    if (message.error) reject(new Error(message.error.message));
    else resolve(message.result);
  } else {
    listeners.get(message.method)?.();
  }
});

const send = (method, params = {}) =>
  new Promise((resolve, reject) => {
    const id = ++nextId;
    pending.set(id, { resolve, reject });
    socket.send(JSON.stringify({ id, method, params }));
  });

const evaluate = async (expression) =>
  (await send('Runtime.evaluate', { expression, returnByValue: true })).result.value;

const waitFor = async (expression, what, timeoutMs = 15000) => {
  for (let waited = 0; waited < timeoutMs; waited += 200) {
    if (await evaluate(expression)) return;
    await sleep(200);
  }
  throw new Error(`timed out waiting for ${what}`);
};

try {
  await send('Page.enable');
  await send('Emulation.setDeviceMetricsOverride', {
    width: WIDTH,
    height: HEIGHT,
    deviceScaleFactor: 1,
    mobile: false,
  });
  const loaded = new Promise((resolve) => listeners.set('Page.loadEventFired', resolve));
  await send('Page.navigate', { url });
  await loaded;

  const button = (pattern) =>
    `[...document.querySelectorAll('button')].find((b) => ${pattern}.test(b.textContent.trim()))`;
  const rawTab = button('/^Raw JSON$/');
  const formTab = button('/^Form$/');
  const renderButton = button('/^Render/');

  await waitFor(`Boolean(${renderButton})`, 'the Render button');
  await evaluate(`${rawTab}.click()`);
  await waitFor(`Boolean(document.querySelector('textarea'))`, 'the Raw JSON editor');
  await evaluate(`(() => {
    const editor = document.querySelector('textarea');
    editor.value = ${JSON.stringify(payload)};
    editor.dispatchEvent(new Event('input', { bubbles: true }));
  })()`);
  await evaluate(`${formTab}.click()`);
  await sleep(500);
  await evaluate(`${renderButton}.click()`);
  await waitFor(`Boolean(document.querySelector('iframe'))`, 'the PDF preview');
  // Give the PDF viewer time to paint the first page.
  await sleep(3000);

  const { data } = await send('Page.captureScreenshot', { format: 'png' });
  writeFileSync(output, Buffer.from(data, 'base64'));
  console.log(`Wrote ${output}`);
} finally {
  socket.close();
  chrome.kill();
  await new Promise((resolve) => chrome.once('exit', resolve));
  rmSync(profile, { recursive: true, force: true });
}
