// Real engine + built UI, isolated first-run data and loopback ports. No media ingestion.
import { chromium } from 'playwright';
import { spawn } from 'node:child_process';
import { mkdtemp, readFile, mkdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import assert from 'node:assert/strict';

const root = fileURLToPath(new URL('../../', import.meta.url));
await mkdir(path.join(root, '.tmp'), { recursive: true });
const dataDir = await mkdtemp(path.join(root, '.tmp', 'ux-live-'));
const service = spawn(path.join(root, 'target/debug/open-delay-service.exe'), [
  '--data-dir', dataDir, '--api-port', '0', '--rtmp-port', '0', '--ui-dir', path.join(root, 'ui/dist'),
], { cwd: root, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
let browser;
let origin;
let token;
let serviceError;
let serviceOutput = '';
service.on('error', error => { serviceError = error; });
for (const stream of [service.stdout, service.stderr]) stream.on('data', chunk => { serviceOutput = (serviceOutput + chunk).slice(-4000); });
const errors = [];
const mediaCommands = [];
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));

async function api(endpoint, body) {
  const response = await fetch(`${origin}/api/v1${endpoint}`, {
    method: body === undefined ? 'GET' : 'POST',
    headers: { Origin: origin, 'Content-Type': 'application/json', ...(token ? { 'X-OpenDelay-Token': token } : {}) },
    ...(body === undefined ? {} : { body: JSON.stringify(body) }),
  });
  assert.equal(response.status, 200, `${endpoint}: HTTP ${response.status}`);
  return response.json();
}

function watch(page) {
  page.on('pageerror', error => errors.push(error.message));
  page.on('request', request => {
    if (new URL(request.url()).pathname === '/api/v1/commands') mediaCommands.push(request.postData());
  });
  return page;
}

async function assertOnScreen(locator, viewport, message) {
  const rect = await locator.boundingBox();
  assert.ok(rect && rect.x >= -1 && rect.y >= -1 && rect.x + rect.width <= viewport.width + 1 && rect.y + rect.height <= viewport.height + 1, message);
}

try {
  for (let attempt = 0; attempt < 100; attempt++) {
    if (serviceError) throw serviceError;
    assert.equal(service.exitCode, null, `Service exited: ${serviceOutput}`);
    try { origin = JSON.parse(await readFile(path.join(dataDir, 'runtime.json'), 'utf8')).api_base; break; }
    catch { await sleep(200); }
  }
  assert.ok(origin, `Isolated service starts: ${serviceOutput}`);
  token = (await api('/session')).token;
  const initial = await api('/status');
  const routes = (await api('/routes')).routes;
  assert.equal(routes.length, 6, 'Real first-run fixture has six platform presets');
  assert.equal(routes.filter(route => route.enabled).length, 1, 'Only one preset is enabled');
  assert.match(initial.version, /^\d+\.\d+\.\d+/, 'Status exposes the engine version');

  browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || 'C:/Program Files/Google/Chrome/Application/chrome.exe', headless: true });
  // Each fresh context exercises an actual first-run presentation, not simulated API data.
  for (const viewport of [{ width: 1280, height: 720 }, { width: 320, height: 480 }]) {
    const context = await browser.newContext({ viewport, reducedMotion: 'reduce', locale: 'fr-FR' });
    const intro = watch(await context.newPage());
    await intro.goto(origin);
    const dialog = intro.getByRole('dialog', { name: 'Bienvenue dans Open-Delay', exact: true });
    await dialog.waitFor();
    await assertOnScreen(dialog.locator('.dialog-footer'), viewport, 'Introduction footer stays inside the viewport');
    await assertOnScreen(dialog.getByRole('button', { name: 'Découvrir la suite', exact: true }), viewport, 'Introduction next action is visible without scrolling');
    await dialog.getByRole('button', { name: 'Découvrir la suite', exact: true }).click();
    await assertOnScreen(dialog.getByRole('button', { name: 'Précédent', exact: true }), viewport, 'Introduction back action stays visible on slide two');
    await assertOnScreen(dialog.getByRole('button', { name: 'Découvrir la suite', exact: true }), viewport, 'Introduction next action stays visible on slide two');
    await dialog.getByRole('button', { name: 'Découvrir la suite', exact: true }).click();
    await assertOnScreen(dialog.getByRole('button', { name: 'Configurer mon stream', exact: true }), viewport, 'Setup action stays visible on slide three');
    await dialog.getByRole('button', { name: 'Précédent', exact: true }).click();
    await dialog.getByRole('button', { name: 'Précédent', exact: true }).click();
    await dialog.getByRole('button', { name: 'Passer la présentation', exact: true }).click();
    const setup = intro.getByRole('dialog', { name: 'Open-Delay', exact: true });
    await setup.waitFor();
    await setup.getByRole('button', { name: 'Fermer / Close', exact: true }).click();
    await intro.locator('.command-scope').waitFor();
    assert.match(await intro.locator('.command-scope').innerText(), /1 sortie\(s\) activée\(s\), 0 connectée\(s\)/);
    if (viewport.width === 1280) assert.match(await intro.locator('.version').innerText(), new RegExp(initial.version.replaceAll('.', '\\.')));
    assert.ok(await intro.evaluate(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1), 'First-run dashboard reflows without page overflow');
    await context.close();
  }
  console.log('OK REAL UX: first-run introduction at 1280×720 / 320×480, accessible actions, one enabled output, engine version');

  const youtube = routes.find(route => route.id === 'youtube');
  assert.equal(youtube.enabled, false, 'YouTube starts as a disabled platform preset');
  await api('/routes', { ...youtube, upstream_url: 'rtmp://127.0.0.1:19359/ux-only', include_in_group: false });
  const viewport = { width: 760, height: 400 };
  const context = await browser.newContext({ viewport, reducedMotion: 'reduce', locale: 'fr-FR' });
  await context.addInitScript(() => localStorage.setItem('open-delay.intro-v1', 'seen'));
  const page = watch(await context.newPage());
  await page.goto(origin);
  const setup = page.getByRole('dialog', { name: 'Open-Delay', exact: true });
  await setup.waitFor();
  await assertOnScreen(setup.locator('.dialog-footer'), viewport, 'Setup footer remains visible in a short window');
  for (const name of ['OBS Studio', 'Streamlabs']) {
    const choice = setup.getByRole('button', { name, exact: true });
    await choice.scrollIntoViewIfNeeded();
    const choiceRect = await choice.boundingBox();
    const footerRect = await setup.locator('.dialog-footer').boundingBox();
    assert.ok(choiceRect && footerRect && choiceRect.y >= 0 && choiceRect.y + choiceRect.height <= footerRect.y + 1, `${name} stays above the footer after scrolling`);
    await choice.click();
    assert.equal(await choice.getAttribute('aria-pressed'), 'true', `${name} can be selected`);
  }
  await setup.getByRole('button', { name: 'Continuer', exact: true }).click();
  await setup.getByRole('combobox', { name: 'Plateforme', exact: true }).click();
  await page.getByRole('option', { name: 'YouTube', exact: true }).click();
  const serverInput = setup.getByRole('textbox', { name: 'Serveur', exact: true });
  await serverInput.fill('not-a-server');
  await setup.getByRole('button', { name: 'Continuer', exact: true }).click();
  assert.equal(await serverInput.inputValue(), 'not-a-server', 'Invalid server input is preserved');
  assert.equal(await serverInput.getAttribute('aria-invalid'), 'true');
  assert.equal(await serverInput.getAttribute('aria-describedby'), 'onboarding-server-error');
  assert.match(await setup.getByRole('alert').innerText(), /adresse de serveur commençant par rtmp:\/\/ ou rtmps:\/\//);
  assert.doesNotMatch(await setup.getByRole('alert').innerText(), /Failed to construct|Invalid URL/);
  await serverInput.fill('rtmp://127.0.0.1:19359/ux-only');
  await setup.getByRole('button', { name: 'Continuer', exact: true }).click();
  await setup.getByText('Relier votre sortie', { exact: true }).waitFor();
  const selectedOutput = (await api('/routes')).routes.find(route => route.id === 'youtube');
  assert.equal(selectedOutput.enabled, true, 'Choosing a disabled preset enables the selected output even when its server is unchanged');
  assert.equal(selectedOutput.include_in_group, false, 'Existing group inclusion is preserved');
  assert.match(await setup.locator('.copy-field').innerText(), new RegExp(`rtmp://127.0.0.1:${initial.endpoints.rtmp_port}/youtube`));
  await setup.getByRole('button', { name: 'Continuer', exact: true }).click();
  await setup.getByRole('button', { name: 'Passer cette étape', exact: true }).click();
  assert.match(await setup.locator('.onboarding-signal').innerText(), /^En attente de votre signal\./);
  await setup.getByRole('button', { name: 'Ouvrir ma régie', exact: true }).click();
  await setup.waitFor({ state: 'detached' });
  assert.equal((await api('/preferences')).onboarding_complete, true);
  assert.equal((await api('/status')).is_connected, false, 'Setup completion never invents an incoming stream');
  console.log('OK REAL UX: short-window software choices, readable validation, preserved input, corrected loopback server, honest waiting state');

  await page.setViewportSize({ width: 1280, height: 720 });
  const nav = page.getByRole('navigation', { name: 'Navigation principale', exact: true });
  await nav.getByRole('button', { name: 'Paramètres', exact: true }).click();
  const customDelay = page.getByLabel('Durée du raccourci personnalisé', { exact: true });
  await customDelay.fill('75');
  await nav.getByRole('button', { name: 'Aide', exact: true }).click();
  await page.getByRole('button', { name: 'Rester sur cette page', exact: true }).click();
  assert.equal(await customDelay.inputValue(), '75', 'Staying preserves the settings draft');
  await page.getByRole('button', { name: 'Enregistrer les réglages', exact: true }).click();
  await page.getByText('Réglages enregistrés.', { exact: true }).waitFor();
  assert.equal((await api('/preferences')).hotkeys.custom_delay_ms, 75000, 'Settings save reaches the real engine');
  await nav.getByRole('button', { name: 'Aide', exact: true }).click();
  assert.equal(await page.getByRole('dialog').count(), 0, 'Saved settings no longer block navigation');
  await page.reload();
  await nav.getByRole('button', { name: 'Paramètres', exact: true }).click();
  assert.equal(await customDelay.inputValue(), '75', 'Settings survive reload');
  console.log('OK REAL UX: settings draft preserved, saved to the engine and retained after reload');

  await nav.getByRole('button', { name: 'Overlay', exact: true }).click();
  await page.getByRole('button', { name: 'Contenu et comportement', exact: true }).click();
  await page.getByRole('textbox', { name: 'Texte personnalisé', exact: true }).fill('TEST UX LOCAL');
  await page.getByRole('checkbox', { name: 'Utiliser mon texte dans le badge', exact: true }).check();
  await page.getByRole('button', { name: 'Appliquer à la source', exact: true }).click();
  await page.getByText('Réglages appliqués à la source liée.', { exact: true }).waitFor();
  assert.equal(JSON.parse(await readFile(path.join(dataDir, 'overlay.json'), 'utf8')).text, 'TEST UX LOCAL');
  await page.reload();
  await page.getByRole('button', { name: 'Contenu et comportement', exact: true }).click();
  assert.equal(await page.getByRole('textbox', { name: 'Texte personnalisé', exact: true }).inputValue(), 'TEST UX LOCAL');
  const source = watch(await context.newPage());
  await source.goto(`${origin}/overlay?managed=true`);
  await page.getByText('Source active récemment', { exact: true }).waitFor();
  await page.getByRole('button', { name: 'Test délai', exact: true }).click();
  await source.locator('.overlay-popup').waitFor();
  assert.match(await source.locator('.overlay-popup').innerText(), /TEST UX LOCAL/);
  await page.getByRole('button', { name: 'Arrêter le test', exact: true }).click();
  await source.locator('.overlay-canvas').waitFor({ state: 'detached' });
  assert.equal((await api('/status')).is_connected, false, 'Overlay test never creates a media connection');
  assert.deepEqual(mediaCommands, [], 'UX test sends no media-control command');
  assert.deepEqual(errors, [], 'No browser exception');
  console.log('OK REAL UX: overlay persisted/reloaded, independent browser source displays and stops the test; no external stream or media command');
} finally {
  await browser?.close();
  if (origin && token) await fetch(`${origin}/api/v1/shutdown`, { method: 'POST', headers: { Origin: origin, 'X-OpenDelay-Token': token } }).catch(() => undefined);
  if (service.exitCode === null) {
    await Promise.race([new Promise(resolve => service.once('exit', resolve)), sleep(3000)]);
    if (service.exitCode === null) service.kill();
  }
}
