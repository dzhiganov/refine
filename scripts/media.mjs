// Records the landing-page media from the real UI (browser mock with real model answers).
// Needs `pnpm dev` running. Writes PNG screenshots and screencast frames to <out>/<theme>/.
//   node scripts/media.mjs <out> [dark|light]
import { chromium } from '@playwright/test';
import { mkdirSync, writeFileSync } from 'node:fs';

const out = process.argv[2] ?? 'media';
const theme = process.argv[3] ?? 'dark';
const url = process.env.REFINE_URL ?? 'http://localhost:1420';
const EMAIL =
  'Dear Mr. Weber, thx for the quick reply. Yesterday I have sent the contract, pls let me know until Friday if everything is fine.';
const dir = `${out}/${theme}`;
mkdirSync(`${dir}/frames`, { recursive: true });

// The flag (not deviceScaleFactor) is what makes the screencast frames 2x in headless Chromium.
const browser = await chromium.launch({ args: ['--force-device-scale-factor=2'] });
const page = await browser.newPage({ viewport: { width: 1200, height: 760 }, colorScheme: theme });
await page.addInitScript((t) => {
  localStorage.setItem('refine-theme', t);
  localStorage.setItem('refine-fixed-view', 'clean');
}, theme);

// A small cursor so viewers can follow the pointer in the recording.
const cursor = theme === 'dark' ? '#eef0f5' : '#0d0e11';
await page.addInitScript((color) => {
  addEventListener('DOMContentLoaded', () => {
    const c = document.createElement('div');
    c.id = 'demo-cursor';
    c.innerHTML = `<svg width="18" height="18" viewBox="0 0 24 24"><path d="M5 3l14 8-6 2-3 6z" fill="${color}" stroke="${color === '#eef0f5' ? '#14161c' : '#fff'}" stroke-width="1.5" stroke-linejoin="round"/></svg>`;
    Object.assign(c.style, { position: 'fixed', left: '-40px', top: '-40px', zIndex: 99999, pointerEvents: 'none', transition: 'left 60ms linear, top 60ms linear' });
    document.body.appendChild(c);
    addEventListener('mousemove', (e) => {
      c.style.left = `${e.clientX - 3}px`;
      c.style.top = `${e.clientY - 2}px`;
    });
  });
}, cursor);

await page.goto(url);
await page.waitForSelector('textarea');
await page.waitForTimeout(600);

// Move the pointer smoothly to an element's center.
let mouse = { x: 600, y: 700 };
async function glide(selector, steps = 18) {
  const box = await page.locator(selector).first().boundingBox();
  const to = { x: box.x + box.width / 2, y: box.y + box.height / 2 };
  await page.mouse.move(to.x, to.y, { steps });
  mouse = to;
}

// Formal email tone, before recording.
await page.click('button[aria-label^="Tone"]');
await page.click('[role="option"]:has-text("Formal email")');
await page.mouse.move(mouse.x, mouse.y);
await page.waitForTimeout(400);

// ---- recording -----------------------------------------------------------------------
const cdp = await page.context().newCDPSession(page);
const frames = [];
cdp.on('Page.screencastFrame', async ({ data, metadata, sessionId }) => {
  frames.push({ data, t: metadata.timestamp });
  await cdp.send('Page.screencastFrameAck', { sessionId }).catch(() => {});
});
await cdp.send('Page.startScreencast', { format: 'jpeg', quality: 92, everyNthFrame: 1, maxWidth: 2400, maxHeight: 1520 });

await page.waitForTimeout(700);
await glide('textarea');
await page.click('textarea');
await page.keyboard.type(EMAIL, { delay: 22 });
await page.waitForTimeout(500);
await glide('button:has-text("Check")');
await page.waitForTimeout(250);
await page.click('button:has-text("Check")');
await page.waitForSelector('text=Fixed text');
await page.waitForTimeout(1600);
await glide('[aria-label="Mistakes"] li:nth-child(4)');
await page.waitForTimeout(1700);
await glide('[aria-label="Mistakes"] li:nth-child(2)');
await page.waitForTimeout(1500);
await glide('[aria-label="Suggestions"] li:nth-child(1)', 24);
await page.waitForTimeout(1500);
await page.mouse.move(1150, 120, { steps: 20 });
await page.waitForTimeout(900);

await cdp.send('Page.stopScreencast');
await page.waitForTimeout(300);

// Frames → PNGs plus an ffconcat list with each frame's real duration.
let list = 'ffconcat version 1.0\n';
frames.forEach((f, i) => {
  const name = `f${String(i).padStart(5, '0')}.jpg`;
  writeFileSync(`${dir}/frames/${name}`, Buffer.from(f.data, 'base64'));
  const next = frames[i + 1]?.t ?? f.t + 0.5;
  list += `file '${name}'\nduration ${Math.max(0.016, next - f.t).toFixed(3)}\n`;
});
list += `file 'f${String(frames.length - 1).padStart(5, '0')}.jpg'\n`;
writeFileSync(`${dir}/frames/list.ffconcat`, list);
console.log(`${theme}: ${frames.length} frames over ${(frames.at(-1).t - frames[0].t).toFixed(1)} s`);

// ---- screenshots (via CDP, which returns device pixels: 2x) -------------------------------
const snap = async (path) => {
  const { data } = await cdp.send('Page.captureScreenshot', { format: 'png' });
  writeFileSync(path, Buffer.from(data, 'base64'));
};

await page.evaluate(() => document.getElementById('demo-cursor')?.remove());
await page.evaluate(() => document.querySelector('main')?.scrollTo(0, 0));
await page.waitForTimeout(500);
await snap(`${dir}/result.png`);

// Progress: the stat tiles and the watch list (taller viewport so the cards aren't cut off).
await page.setViewportSize({ width: 1200, height: 1000 });
await page.click('[aria-label="Progress"]');
await page.waitForSelector('text=Watching');
await page.waitForTimeout(900);
{
  const tiles = await page.locator('[class*="tiles"]').first().boundingBox();
  const watch = await page.locator('section:has(h2:text("Watching"))').first().boundingBox();
  const pad = 28;
  const clip = { x: tiles.x - pad, y: tiles.y - pad, width: tiles.width + pad * 2, height: watch.y + watch.height - tiles.y + pad * 2 };
  const { data } = await cdp.send('Page.captureScreenshot', { format: 'png', clip: { ...clip, scale: 1 } });
  writeFileSync(`${dir}/progress.png`, Buffer.from(data, 'base64'));
}
await page.setViewportSize({ width: 1200, height: 760 });

await page.click('[aria-label="History"]');
await page.waitForTimeout(900);
await snap(`${dir}/history.png`);

await browser.close();
