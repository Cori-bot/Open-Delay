import { chromium } from 'playwright';
import { createServer } from 'vite';
import assert from 'node:assert/strict';

const server = await createServer({ server: { host: '127.0.0.1', port: 0, strictPort: false }, logLevel: 'error' });
let browser;
try {
  console.log('Starting isolated Vite server');
  await server.listen();
  console.log('Launching Chromium');
  const address = server.httpServer.address();
  browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || 'C:/Program Files/Google/Chrome/Application/chrome.exe', headless: true, timeout: 15000 });
  console.log('Chromium launched');
  for (const width of [640, 1100]) {
    const page = await browser.newPage({ viewport: { width, height: 700 } });
    page.setDefaultTimeout(8000);
    console.log(`Opening fixture (${width}px)`);
    await page.goto(`http://127.0.0.1:${address.port}/tests/select.html`);
    const trigger = page.getByRole('combobox', { name: 'Choix' });
    await trigger.focus();
    await page.keyboard.press('Enter');
    await page.getByRole('option', { name: 'Deuxième choix' }).click();
    assert.equal(await page.locator('output').innerText(), 'two');
    assert.equal(await page.getByRole('combobox', { name: 'Indisponible' }).isDisabled(), true);
    await trigger.focus();
    await page.keyboard.press('Enter');
    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('Enter');
    assert.equal(await page.locator('output').innerText(), 'three');
    await page.getByRole('button', { name: 'Ouvrir dialogue' }).click();
    await page.getByRole('combobox', { name: 'Dans le dialogue' }).click();
    await page.getByRole('option', { name: 'Premier choix' }).click();
    assert.equal(await page.locator('output').innerText(), 'one');
    await page.keyboard.press('Escape');
    assert.equal(await page.getByRole('dialog').count(), 0);
    await page.close();
    console.log(`Select mouse, keyboard, disabled and dialog: OK (${width}px)`);
  }
} finally {
  await browser?.close();
  await server.close();
}
