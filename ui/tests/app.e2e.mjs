import { chromium } from 'playwright';
import { createServer } from 'vite';
import assert from 'node:assert/strict';

const server = await createServer({ server: { host: '127.0.0.1', port: 0, strictPort: false }, logLevel: 'error' });
let browser;
const pages = ['dashboard','routes','overlay','streamdeck','setup','help','settings'];
const preferences = { language:'fr', mode:'instant', buffer_ms:60000, disk_bytes:2147483648, cache_bytes:67108864, api_port:49154, rtmp_port:49155, lan_ingest:false, startup:false, quit_on_close:false, onboarding_complete:true, hotkeys:{start_delay:null,go_live:null,toggle_overlay:null,custom_delay:null,custom_delay_ms:60000} };
const route = {id:'youtube',name:'YouTube',platform:'youtube',local_app:'youtube',upstream_url:'rtmp://a.rtmp.youtube.com/live2',enabled:true,include_in_group:true};
const runtime = {id:'youtube',is_connected:true,generation:1,transport_state:'connected',effective_mode:'instant_delayed',desired_delay_ms:30000,effective_delay_ms:30000,buffer:{fill_ms:60000,target_ms:60000,ready:true,degraded:false,size_bytes:5000000},bytes_received:10000000,bytes_sent:9000000,error:null};
const status = {sequence:1,version:'0.1.0',mode:'instant',effective_mode:'instant_delayed',is_connected:true,desired_delay_ms:30000,effective_delay_ms:30000,transition_in_progress:false,overlay_hidden:false,routes:[runtime],preferences,endpoints:{api_base:'http://127.0.0.1:49154',rtmp_port:49155}};
try {
  await server.listen(); const address=server.httpServer.address(); const origin=`http://127.0.0.1:${address.port}`;
  browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH||'C:/Program Files/Google/Chrome/Application/chrome.exe',headless:true,timeout:15000});
  for (const lang of ['fr','en']) for (const width of [320,640,1100]) {
    const page=await browser.newPage({viewport:{width,height:820},reducedMotion:'reduce'}); const errors=[]; const commands=[];
    page.on('pageerror', e=>errors.push(e.message));
    await page.addInitScript((value)=>{ localStorage.setItem('open-delay.language',value); localStorage.setItem('open-delay.intro-v1','seen'); },lang);
    await page.route('**/api/v1/**',async routeRequest=>{
      const url=new URL(routeRequest.request().url()); let data;
      if(url.pathname.endsWith('/session')) data={token:'isolated-test'};
      else if(url.pathname.endsWith('/overlay-control')) data={config:null,revision:0,source_recent:false,test:null};
      else if(url.pathname.endsWith('/events')) return routeRequest.fulfill({status:503,body:'unavailable'});
      else if(url.pathname.endsWith('/status')) data={...status,preferences:{...preferences,language:lang},endpoints:{...status.endpoints,api_base:origin}};
      else if(url.pathname.endsWith('/routes')) data={routes:[route]};
      else if(url.pathname.endsWith('/preferences')) data={...preferences,language:lang};
      else if(url.pathname.endsWith('/commands')) { const command=routeRequest.request().postDataJSON(); commands.push(command.action); if(command.action==='set_delay') await new Promise(resolve=>setTimeout(resolve,1200)); data={success:true,status:{...status,preferences:{...preferences,language:lang}},results:[]}; }
      else data={entries:[]};
      await routeRequest.fulfill({status:200,contentType:'application/json',body:JSON.stringify(data)});
    });
    for (const current of pages) {
      await page.goto(`${origin}/?page=${current}`,{waitUntil:'domcontentloaded'});
      await page.locator('main').waitFor(); await page.waitForTimeout(250);
      assert.equal(await page.locator('main h1').count(),1,`${lang}/${width}/${current}: heading`);
      const overflow=await page.evaluate(()=>document.documentElement.scrollWidth-document.documentElement.clientWidth);
      assert.ok(overflow<=1,`${lang}/${width}/${current}: horizontal overflow ${overflow}px`);
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
      if(lang==='fr' && width===1100 && current==='dashboard') { await page.locator('.presets button').nth(1).click(); await page.getByRole('button',{name:'Revenir au direct',exact:true}).click(); await page.waitForTimeout(100); assert.deepEqual(commands,['set_delay']); await page.waitForTimeout(1300); assert.deepEqual(commands.slice(0,2),['set_delay','go_live']); }
      console.log(`OK ${lang} ${width} ${current}`);
    }
    if (lang==='en' && width===1100) {
      status.is_connected=false; status.routes=[{...runtime,is_connected:false,transport_state:'error',error:'Output unavailable'}];
      await page.goto(`${origin}/?page=dashboard`); await page.getByRole('alert').first().waitFor();
      assert.equal(await page.locator('.signal-indicator').innerText(), 'Error');
      assert.equal(await page.getByRole('button',{name:'Go live',exact:true}).isDisabled(),true);
    }
    if (lang==='en' && width===1100) {
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

