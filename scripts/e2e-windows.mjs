// End-to-end run of the real Windows app with a real model, driven over WebView2's DevTools port.
// Start the app with WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9223 (and
// REFINE_DATA_DIR pointing at a scratch folder), then: node e2e-windows.mjs <outDir> <provider label> <model id>
import { chromium } from 'playwright-core';

const [out = '.', providerLabel = 'LM Studio', modelId = 'qwen'] = process.argv.slice(2);
const browser = await chromium.connectOverCDP('http://127.0.0.1:9223', { timeout: 30000 });
const page = browser.contexts()[0].pages()[0];
const errors = [];
page.on('pageerror', (e) => errors.push(e.message));
page.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
const shot = async (name) => {
  await page.waitForTimeout(600);
  await page.screenshot({ path: `${out}/${name}.png` });
  console.log('shot', name);
};
const timed = async (label, fn) => {
  const t = Date.now();
  await fn();
  console.log(`${label}: ${((Date.now() - t) / 1000).toFixed(1)} s`);
};

// 1. Point the app at the local model.
await page.click('[aria-label="Settings"]');
await page.click(`[role="group"][aria-label="Provider"] button:has-text("${providerLabel}")`);
await page.click('button[aria-expanded]:near(:text("Model"))');
await page.waitForSelector('[role="option"]', { timeout: 30000 });
await page.click(`[role="option"]:has-text("${modelId}")`);
await shot('1-settings');

// 2. A German Slack message with real mistakes.
const check = async (name, tone, text) => {
  await page.click('[aria-label="Write"]');
  // Either the composer or a previous result (page transitions are animated).
  await page.waitForSelector('textarea, button:has-text("New text")');
  if (await page.isVisible('button:has-text("New text")')) await page.click('button:has-text("New text")');
  await page.waitForSelector('textarea');
  await page.click('button[aria-label^="Tone"]');
  await page.click(`[role="option"]:has-text("${tone}")`);
  await page.fill('textarea', text);
  await timed(`check "${name}"`, async () => {
    await page.keyboard.press('Control+Enter');
    await page.waitForFunction(
      () => document.body.innerText.includes('Fixed text') || document.querySelector('[role="alert"]'),
      null,
      { timeout: 300000 },
    );
  });
  const alert = await page.$('[role="alert"]');
  if (alert) console.log('ERROR shown:', await alert.innerText());
  const mistakes = await page.$$eval('[aria-label="Mistakes"] li', (els) => els.map((e) => e.innerText.replace(/\s+/g, ' ')));
  const ideas = await page.$$eval('[aria-label="Suggestions"] li', (els) => els.map((e) => e.innerText.replace(/\s+/g, ' ')));
  console.log(`  mistakes (${mistakes.length}):\n   - ${mistakes.join('\n   - ')}`);
  console.log(`  suggestions (${ideas.length}):\n   - ${ideas.join('\n   - ')}`);
  await shot(name);
};

await check('2-german-slack', 'Slack', 'Hallo zusammen, ich habe gestern mit der Kunde gesprochen und er hat gesagt, dass er braucht mehr Zeit für die Tests. Wir machen dann am Freitag eine Entscheidung.');
await check('3-english-email', 'Formal email', 'Dear Mr. Weber, thx for the quick reply. Yesterday I have sent the contract, pls let me know until Friday if everything is fine.');
await check('4-english-slack-clean', 'Slack', 'lgtm, just one nit: can u rename the var to something more descriptive? thx');

// 3. The other pages with real data.
await page.click('[aria-label="History"]');
await shot('5-history');
await page.click('[aria-label="Progress"]');
await shot('6-progress');

console.log(errors.length ? `PAGE ERRORS:\n${errors.join('\n')}` : 'no page errors');
// Don't browser.close(): over CDP that closes the WebView's browser side, not just this client.
process.exit(0);
