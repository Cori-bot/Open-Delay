import { chromium } from 'playwright';
import { createServer } from 'vite';
import assert from 'node:assert/strict';

const server = await createServer({ server: { host: '127.0.0.1', port: 0, strictPort: false }, logLevel: 'error' });
let browser;
const pages = ['dashboard','routes','overlay','streamdeck','setup','help','settings'];
const preferences = { language:'fr', mode:'instant', buffer_ms:60000, disk_bytes:2147483648, cache_bytes:67108864, api_port:49154, rtmp_port:49155, lan_ingest:false, startup:false, quit_on_close:false, onboarding_complete:true, hotkeys:{start_delay:null,go_live:null,toggle_overlay:null,custom_delay:null,custom_delay_ms:60000} };
const route = {id:'youtube',name:'YouTube',platform:'youtube',local_app:'youtube',upstream_url:'rtmp://a.rtmp.youtube.com/live2',enabled:true,include_in_group:true};
let configuredRoutes = [route];
const runtime = {id:'youtube',is_connected:true,generation:1,transport_state:'connected',effective_mode:'instant_delayed',desired_delay_ms:30000,effective_delay_ms:30000,buffer:{fill_ms:60000,target_ms:60000,ready:true,degraded:false,size_bytes:5000000},bytes_received:10000000,bytes_sent:9000000,error:null};
const status = {sequence:1,version:'0.1.0',mode:'instant',effective_mode:'instant_delayed',is_connected:true,desired_delay_ms:30000,effective_delay_ms:30000,transition_in_progress:false,overlay_hidden:false,routes:[runtime],preferences,endpoints:{api_base:'http://127.0.0.1:49154',rtmp_port:49155}};
try {
  await server.listen(); const address=server.httpServer.address(); const origin=`http://127.0.0.1:${address.port}`;
  browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH||'C:/Program Files/Google/Chrome/Application/chrome.exe',headless:true,timeout:15000});
  for (const lang of ['fr','en']) for (const width of [320,640,721,760,800,1001,1100,1440,1920]) {
    preferences.onboarding_complete = true;
    status.is_connected = true;
    status.routes = [runtime];
    const page=await browser.newPage({viewport:{width,height:820},reducedMotion:'reduce'}); const errors=[]; const commands=[];
    let overlayConfig = null;
    page.on('pageerror', e=>errors.push(e.message));
    await page.addInitScript((value)=>{ localStorage.setItem('open-delay.language',value); localStorage.setItem('open-delay.intro-v1','seen'); },lang);
    await page.route('**/api/v1/**',async routeRequest=>{
      const url=new URL(routeRequest.request().url()); let data;
      if(url.pathname.endsWith('/session')) data={token:'isolated-test'};
      else if(url.pathname.endsWith('/overlay-control')) {
        if (routeRequest.request().method() === 'POST') overlayConfig = routeRequest.request().postDataJSON();
        data={config:overlayConfig,revision:0,source_recent:false,test:null};
      }
      else if(url.pathname.endsWith('/events')) return routeRequest.fulfill({status:503,body:'unavailable'});
      else if(url.pathname.endsWith('/status')) data={...status,preferences:{...preferences,language:lang},endpoints:{...status.endpoints,api_base:origin}};
      else if(url.pathname.endsWith('/routes')) data={routes:configuredRoutes};
      else if(url.pathname.endsWith('/preferences')) {
        if (routeRequest.request().method() === 'POST') {
          Object.assign(preferences, routeRequest.request().postDataJSON());
          data={success:true,restart_required:false,preferences:{...preferences,language:lang}};
        } else data={...preferences,language:lang};
      }
      else if(url.pathname.endsWith('/commands')) { const command=routeRequest.request().postDataJSON(); commands.push(command.action); if(command.action==='set_delay') await new Promise(resolve=>setTimeout(resolve,1200)); data={success:true,status:{...status,sequence:2,version:undefined,preferences:{...preferences,language:lang}},results:[]}; }
      else data={entries:[]};
      await routeRequest.fulfill({status:200,contentType:'application/json',body:JSON.stringify(data)});
    });
    for (const current of pages) {
      await page.goto(`${origin}/?page=${current}`,{waitUntil:'domcontentloaded'});
      await page.locator('main').waitFor(); await page.waitForTimeout(250);
      assert.equal(await page.locator('main h1').count(),1,`${lang}/${width}/${current}: heading`);
      const overflow=await page.evaluate(()=>document.documentElement.scrollWidth-document.documentElement.clientWidth);
      assert.ok(overflow<=1,`${lang}/${width}/${current}: horizontal overflow ${overflow}px`);
      if (lang === 'en' && width === 320 && current === 'dashboard') {
        const presets = await page.locator('.presets button').evaluateAll(buttons => buttons.map(button => {
          const bounds = button.getBoundingClientRect();
          const walker = document.createTreeWalker(button, NodeFilter.SHOW_TEXT);
          const textRects = [];
          let node;
          while ((node = walker.nextNode())) {
            if (!node.textContent.trim()) continue;
            const range = document.createRange();
            range.selectNodeContents(node);
            for (const rect of range.getClientRects()) textRects.push({left:rect.left,right:rect.right,top:rect.top,bottom:rect.bottom});
          }
          return {name:button.textContent.trim(),fits:button.scrollWidth <= button.clientWidth + 1 && button.scrollHeight <= button.clientHeight + 1,textFits:textRects.length > 0 && textRects.every(rect => rect.left >= bounds.left - 1 && rect.right <= bounds.right + 1 && rect.top >= bounds.top - 1 && rect.bottom <= bounds.bottom + 1)};
        }));
        assert.equal(presets.length, 4);
        for (const preset of presets) {
          assert.equal(preset.fits, true, `${preset.name}: preset content is not clipped at 320px`);
          assert.equal(preset.textFits, true, `${preset.name}: all preset text stays inside its button at 320px`);
        }
      }
      if (lang === 'fr' && width === 1100) {
        // Resize the same mounted page: this must preserve state and usable navigation.
        for (const size of [{width:760,height:400},{width:320,height:480},{width:1920,height:1080}]) {
          await page.setViewportSize(size);
          await page.waitForFunction(() => {
            const nav = document.querySelector('.sidebar nav');
            const selected = nav.querySelector('[aria-current="page"]').getBoundingClientRect();
            const bounds = nav.getBoundingClientRect();
            return selected.left >= bounds.left - 1 && selected.right <= bounds.right + 1;
          });
          assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1), `${current}: resize ${size.width}`);
          await page.locator('.language-switch').scrollIntoViewIfNeeded();
          assert.equal(await page.locator('.language-switch').isVisible(), true);
          const languageBox = await page.locator('.language-switch').boundingBox();
          assert.ok(languageBox.y >= 0 && languageBox.y + languageBox.height <= size.height, `${current}: language reachable`);
        }
        await page.setViewportSize({width,height:820});
      }
      if(current==='streamdeck') {
        const keys = page.locator('.deck-preview button[aria-pressed]');
        assert.ok(await keys.count() >= 4, 'Stream Deck action previews');
        const toggleKey = page.locator('button.deck-preview-key').nth(2);
        await toggleKey.focus(); await toggleKey.press('Space');
        assert.equal(await toggleKey.getAttribute('aria-pressed'), 'true');
        assert.match(await page.locator('.deck-action-detail').innerText(), /Toggle/);
        const sizes = await page.locator('.deck-preview-key').evaluateAll(els=>els.map(el=>[el.offsetWidth,el.offsetHeight]));
        assert.equal(sizes.length,15);
        assert.ok(sizes.every(([w,h])=>Math.abs(w-h)<=1 && Math.abs(w-sizes[0][0])<=1), 'All Stream Deck keys stay square and equal');
        assert.equal(await page.locator('.deck-model-link').count(),0);
      }
      if (lang === 'fr' && width === 1100 && current === 'settings') {
        await page.setViewportSize({width:375,height:667});
        const languageField = page.getByRole('combobox', {name:'Langue',exact:true});
        await languageField.scrollIntoViewIfNeeded();
        const hit = await languageField.evaluate(el => {
          const rect = el.getBoundingClientRect();
          const target = document.elementFromPoint(rect.x + rect.width / 2, rect.y + rect.height / 2);
          return {onScreen:rect.top >= 0 && rect.bottom <= innerHeight,unobstructed:target === el || el.contains(target)};
        });
        assert.equal(hit.onScreen, true, 'Language field stays in view at 375×667');
        assert.equal(hit.unobstructed, true, 'Save bar does not cover the language field after scrolling');
        await languageField.click();
        await page.getByRole('option', {name:'Français',exact:true}).waitFor();
        await page.keyboard.press('Escape');
        await page.setViewportSize({width,height:820});
      }
      if(lang==='fr' && width===1100 && current==='dashboard') { await page.locator('.presets button').nth(1).click(); await page.getByRole('button',{name:'Revenir au direct',exact:true}).click(); await page.waitForTimeout(100); assert.deepEqual(commands,['set_delay']); await page.waitForTimeout(1300); assert.deepEqual(commands.slice(0,2),['set_delay','go_live']); assert.equal(await page.locator('.version').innerText(), 'Open-Delay 0.1.0'); }
      console.log(`OK ${lang} ${width} ${current}`);
    }
    if (lang === 'fr' && width === 1100) {
      await page.goto(`${origin}/?page=settings`);
      const delayField = page.getByLabel('Durée du raccourci personnalisé', {exact:true});
      await delayField.fill('75');
      await page.getByRole('navigation').getByRole('button', {name:'Aide',exact:true}).click();
      await page.getByRole('button', {name:'Rester sur cette page',exact:true}).click();
      assert.equal(await delayField.inputValue(), '75');
      await page.getByRole('navigation').getByRole('button', {name:'Aide',exact:true}).click();
      await page.getByRole('button', {name:'Quitter sans enregistrer',exact:true}).click();
      assert.match(await page.locator('main h1').innerText(), /Aide/);
      await page.getByRole('navigation').getByRole('button', {name:'Paramètres',exact:true}).click();
      assert.equal(await delayField.inputValue(), '60');
      await delayField.fill('70');
      await page.getByRole('button', {name:'Enregistrer les réglages',exact:true}).click();
      await page.getByText('Réglages enregistrés.', {exact:true}).waitFor();
      await page.getByRole('navigation').getByRole('button', {name:'Aide',exact:true}).click();
      assert.equal(await page.getByRole('dialog').count(), 0);
      preferences.hotkeys.custom_delay_ms = 60000;
      await page.getByRole('navigation', {name:'Navigation principale'}).getByRole('button', {name:'Overlay',exact:true}).click();
      await page.locator('input[type=range]').fill('70');
      await page.getByRole('navigation', {name:'Navigation principale'}).getByRole('button', {name:'Aide',exact:true}).click();
      await page.getByRole('button', {name:'Rester sur cette page',exact:true}).click();
      assert.equal(await page.locator('input[type=range]').inputValue(), '70');
      await page.getByRole('button', {name:'Appliquer à la source',exact:true}).click();
      await page.getByText('Réglages appliqués à la source liée.', {exact:true}).waitFor();
      await page.getByRole('navigation', {name:'Navigation principale'}).getByRole('button', {name:'Aide',exact:true}).click();
      assert.equal(await page.getByRole('dialog').count(), 0);
      await page.goto(`${origin}/?page=routes`);
      await page.setViewportSize({width:760,height:400});
      await page.getByRole('button', {name:'Configurer',exact:true}).first().click();
      const dialog = page.getByRole('dialog');
      await dialog.getByRole('button', {name:'Enregistrer',exact:true}).scrollIntoViewIfNeeded();
      const saveBox = await dialog.getByRole('button', {name:'Enregistrer',exact:true}).boundingBox();
      assert.ok(saveBox.y >= 0 && saveBox.y + saveBox.height <= 400, 'Dialog save reachable in short window');
      await page.keyboard.press('Escape');
      assert.equal(await page.getByRole('dialog').count(), 0);
    }
    if (lang==='en' && width===1100) {
      status.is_connected=false; status.routes=[{...runtime,is_connected:false,transport_state:'error',error:'Output unavailable'}];
      await page.goto(`${origin}/?page=dashboard`); await page.waitForFunction(()=>document.querySelector('.signal-indicator')?.textContent === 'Error');
      assert.equal(await page.locator('.signal-indicator').innerText(), 'Error');
      assert.equal(await page.getByRole('button',{name:'Go live',exact:true}).isDisabled(),true);
    }
    if (lang==='en' && width===1100) {
      configuredRoutes = [route, {...route,id:'twitch',name:'Twitch',platform:'twitch',local_app:'my-twitch-output'}];
      await page.goto(`${origin}/?page=setup`);
      await page.getByRole('combobox', {name:'Output to connect',exact:true}).click();
      await page.getByRole('option', {name:'Twitch',exact:true}).click();
      assert.match(await page.locator('.copy-field').first().innerText(), /my-twitch-output/);
      const longName = 'A'.repeat(64);
      configuredRoutes = [route, {...route,id:'twitch',name:longName,platform:'twitch',local_app:'my-twitch-output'}];
      await page.setViewportSize({width:320,height:400});
      await page.goto(`${origin}/?page=setup`);
      const outputSelect = page.getByRole('combobox', {name:'Output to connect',exact:true});
      await outputSelect.click();
      const longOption = page.getByRole('option', {name:longName,exact:true});
      await longOption.scrollIntoViewIfNeeded();
      const optionLayout = await longOption.evaluate(el => {
        const bounds = el.getBoundingClientRect();
        const label = el.firstElementChild;
        const text = label.getBoundingClientRect();
        return {withinViewport:bounds.left >= 0 && bounds.right <= innerWidth + 1,textFits:text.left >= bounds.left && text.right <= bounds.right + 1 && label.scrollWidth <= label.clientWidth + 1 && label.scrollHeight <= label.clientHeight + 1};
      });
      assert.equal(optionLayout.withinViewport, true, 'Long output option stays within a 320px viewport');
      assert.equal(optionLayout.textFits, true, 'A 64-character unbroken output name wraps without clipping in the menu');
      await longOption.click();
      assert.equal(await outputSelect.innerText(), longName);
      const selectedLayout = await outputSelect.evaluate(el => {
        const bounds = el.getBoundingClientRect();
        const label = el.firstElementChild;
        const text = label.getBoundingClientRect();
        return {withinViewport:bounds.left >= 0 && bounds.right <= innerWidth + 1,textFits:text.left >= bounds.left && text.right <= bounds.right + 1 && label.scrollWidth <= label.clientWidth + 1 && label.scrollHeight <= label.clientHeight + 1};
      });
      assert.equal(selectedLayout.withinViewport, true, 'Selected output control stays within a 320px viewport');
      assert.equal(selectedLayout.textFits, true, 'Selected 64-character output name remains fully readable');
      await page.setViewportSize({width,height:820});
      configuredRoutes = [];
      await page.goto(`${origin}/?page=setup`);
      await page.getByText('Set up an output in the assistant to get your OBS address.', {exact:true}).waitFor();
      assert.equal(await page.getByRole('combobox', {name:'Output to connect',exact:true}).count(), 0);
      configuredRoutes = [route];
      route.include_in_group=false;
      status.routes=[runtime];
      await page.goto(`${origin}/?page=dashboard`);
      await page.getByRole('button',{name:'Set up my output',exact:true}).waitFor();
      route.include_in_group=true;
      status.routes=[{...runtime,is_connected:false,transport_state:'disconnected',error:null}];
      await page.goto(`${origin}/?page=dashboard`);
      await page.getByRole('button',{name:'View my OBS address',exact:true}).click();
      await page.waitForFunction(()=>document.activeElement?.id==='main');
      assert.match(await page.locator('main h1').innerText(), /Set|Connect|Configuration/i);
      preferences.onboarding_complete=false;
      await page.goto(`${origin}/?page=dashboard`);
      await page.getByRole('dialog').waitFor();
      await page.getByRole('button',{name:'Continue',exact:true}).click();
      await page.getByRole('button',{name:'Continue',exact:true}).click();
      await page.getByRole('button',{name:'Continue',exact:true}).click();
      await page.getByRole('button',{name:'Skip this step',exact:true}).click();
      assert.match(await page.locator('.onboarding-signal').innerText(), /Waiting for your signal/);
      await page.locator('.setup-progress button').first().click();
      assert.equal(await page.locator('.setup-progress [aria-current=step]').innerText(), '1\nSoftware');
      assert.equal(await page.locator('.onboarding h2').evaluate(el => el === document.activeElement), true);
    }
    assert.deepEqual(errors,[]); await page.close();
  }
} finally { await browser?.close(); await server.close(); }

