// Screenshots of the landing page (desktop dark/light, mobile, hero mid-animation).
//   node scripts/site-shots.mjs <outDir> [url]
import { chromium } from '@playwright/test';
const out = process.argv[2] ?? 'site-shots';
const url = process.argv[3] ?? 'http://127.0.0.1:4321/';
const b = await chromium.launch();
const errors = [];
for (const [name, opts] of [
  ['desktop-dark', { viewport: { width: 1360, height: 900 }, colorScheme: 'dark' }],
  ['desktop-light', { viewport: { width: 1360, height: 900 }, colorScheme: 'light' }],
  ['mobile-dark', { viewport: { width: 390, height: 844 }, colorScheme: 'dark', isMobile: true, hasTouch: true }],
]) {
  const p = await b.newPage({ ...opts, deviceScaleFactor: 2 });
  p.on('pageerror', (e) => errors.push(`${name}: ${e.message}`));
  p.on('console', (m) => m.type() === 'error' && errors.push(`${name}: ${m.text()}`));
  await p.goto(url, { waitUntil: 'networkidle' });
  await p.waitForTimeout(3600); // mid-animation
  await p.screenshot({ path: `${out}/${name}-hero.png` });
  await p.screenshot({ path: `${out}/${name}-full.png`, fullPage: true });
  await p.close();
}
console.log(errors.length ? errors.join('\n') : 'no page errors');
await b.close();
