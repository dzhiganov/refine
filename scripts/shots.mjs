// Screenshots of the main screens against the browser mock (needs `pnpm dev` running).
//   node scripts/shots.mjs [outDir] [--light]
import { chromium } from '@playwright/test';

const out = process.argv[2] ?? 'shots';
const theme = process.argv.includes('--light') ? 'light' : 'dark';
const url = process.env.REFINE_URL ?? 'http://localhost:1420';

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1180, height: 820 }, deviceScaleFactor: 2, colorScheme: theme });
const errors = [];
page.on('pageerror', (e) => errors.push(e.message));
page.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
await page.addInitScript((t) => localStorage.setItem('refine-theme', t), theme);
await page.goto(url);
await page.waitForSelector('textarea');
await page.waitForTimeout(500);
const shot = async (name) => {
  await page.waitForTimeout(700);
  await page.screenshot({ path: `${out}/${theme}-${name}.png` });
};

await shot('1-write-empty');
await page.fill('textarea', 'Hallo zusammen, ich habe gestern mit der Kunde gesprochen. Wir machen eine Entscheidung am Freitag und ich schicke euch dann die Zusammenfassung.');
await shot('2-write-draft');
await page.keyboard.press('Control+Enter');
await page.waitForTimeout(300);
await shot('3-checking');
await page.waitForSelector('text=Fixed text');
await page.waitForTimeout(900);
await shot('4-result');
await page.hover('li:has-text("dative")');
await shot('5-result-hover');
await page.mouse.move(5, 5);
await page.click('textarea');
await page.keyboard.press('End');
await page.keyboard.type(' Danke euch!');
await shot('5b-edited');
await page.click('[aria-label="History"]');
await shot('6-history');
await page.click('[aria-label="Progress"]');
await shot('7-progress');
await page.mouse.move(700, 500);
await page.mouse.wheel(0, 900);
await shot('8-progress-scrolled');
await page.click('[aria-label="Settings"]');
await shot('9-settings');
await page.mouse.move(700, 500);
await page.mouse.wheel(0, 700);
await shot('9b-settings-languages');

console.log(errors.length ? `PAGE ERRORS:\n${errors.join('\n')}` : 'no page errors');
await browser.close();
