import { chromium } from 'playwright';
import { createServer } from 'vite';
import assert from 'node:assert/strict';

const server = await createServer({ server: { host: '127.0.0.1', port: 0, strictPort: false }, logLevel: 'error' });
let browser;
const prefs = { language:'en',mode:'instant',buffer_ms:60000,disk_bytes:2147483648,cache_bytes:67108864,api_port:49154,rtmp_port:49155,lan_ingest:false,startup:false,quit_on_close:false,onboarding_complete:true,hotkeys:{start_delay:null,go_live:null,toggle_overlay:null,custom_delay:null,custom_delay_ms:60000} };
const status = { sequence:1,version:'0.1.0',mode:'instant',effective_mode:'live',is_connected:false,desired_delay_ms:0,effective_delay_ms:0,transition_in_progress:false,overlay_hidden:false,routes:[],preferences:prefs };
const control = { config:null,revision:0,source_recent:false,test:null };
const commands = [];
let holdNextRead=false; let releaseRead; let heldRead; let failNextSave=false;
const errors = [];
try {
  await server.listen();
  const origin = `http://127.0.0.1:${server.httpServer.address().port}`;
  browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || 'C:/Program Files/Google/Chrome/Application/chrome.exe', headless:true });
  const context = await browser.newContext({ viewport:{ width:1100,height:900 },reducedMotion:'reduce' });
  context.setDefaultTimeout(10000);
  await context.addInitScript(()=>localStorage.setItem('open-delay.language','en'));
  await context.route('**/api/v1/**', async route=>{
    const request = route.request(); const url = new URL(request.url()); let data;
    if(control.test && control.test.expires_at_ms<=Date.now()) control.test=null;
    if(url.pathname.endsWith('/session')) data={token:'isolated-test'};
    else if(url.pathname.endsWith('/overlay-control')) {
      if(request.method()==='POST') {
        if(failNextSave) { failNextSave=false; return route.fulfill({status:500,contentType:'application/json',body:JSON.stringify({message:'Save rejected for test'})}); }
        assert.equal(request.headers()['x-opendelay-token'],'isolated-test');
        control.config=request.postDataJSON(); control.revision++;
      }
      if(request.method()==='GET' && holdNextRead) {
        holdNextRead=false; heldRead=structuredClone(control);
        await new Promise(resolve=>{releaseRead=resolve;});
        return route.fulfill({status:200,contentType:'application/json',body:JSON.stringify(heldRead)});
      }
      data=control;
    } else if(url.pathname.endsWith('/overlay-test')) {
      assert.equal(request.headers()['x-opendelay-token'],'isolated-test');
      const body=request.postDataJSON();
      control.test=body.state==='stop'?null:{id:crypto.randomUUID(),state:body.state,delay_ms:body.delay_ms,expires_at_ms:Date.now()+5000};
      data=control;
    } else if(url.pathname.endsWith('/overlay-status')) {
      if(url.searchParams.has('viewer')) control.source_recent=true;
      data={...status,...control};
    } else if(url.pathname.endsWith('/events')) return route.fulfill({status:503,body:'unavailable'});
    else if(url.pathname.endsWith('/status')) data={...status,endpoints:{api_base:origin,rtmp_port:49155}};
    else if(url.pathname.endsWith('/preferences')) data=prefs;
    else if(url.pathname.endsWith('/routes')) data={routes:[]};
    else if(url.pathname.endsWith('/commands')) { commands.push(request.postDataJSON()); data={success:true,status}; }
    else throw new Error(`Unexpected request ${url.pathname}`);
    await route.fulfill({status:200,contentType:'application/json',body:JSON.stringify(data)});
  });
  console.log('Starting overlay interactions');
  const page=await context.newPage(); page.on('pageerror',e=>errors.push(e.message));
  await page.goto(`${origin}/?page=overlay`);
  await page.getByRole('button',{name:'Apply to source',exact:true}).waitFor();
  await page.getByRole('button',{name:'Content & behavior'}).click();
  await page.getByRole('textbox',{name:'Custom text'}).fill('SOURCE TEST');
  await page.getByRole('checkbox',{name:'Use my text in the badge'}).check();
  await page.getByRole('button',{name:'Apply to source',exact:true}).click();
  await page.getByRole('button',{name:'Test delay',exact:true}).waitFor({state:'visible'});
  assert.equal(control.config.text,'SOURCE TEST');
  const source=await context.newPage(); source.on('pageerror',e=>errors.push(e.message));
  await source.goto(`${origin}/overlay?managed=true`);
  await page.getByText('Source recently active',{exact:true}).waitFor();
  await page.getByRole('button',{name:'Test delay',exact:true}).click();
  await source.locator('.overlay-badge').waitFor();
  assert.match(await source.locator('.overlay-badge').innerText(),/SOURCE TEST/);
  assert.match(await source.locator('.overlay-popup').innerText(),/SOURCE TEST/);
  assert.equal(commands.length,0,'Visual tests never send media commands');
  await page.getByRole('button',{name:'Stop test',exact:true}).click();
  await source.locator('.overlay-canvas').waitFor({state:'detached'});
  await page.getByRole('button',{name:'Test building',exact:true}).click();
  await source.locator('.overlay-canvas').waitFor();
  await source.locator('.overlay-canvas').waitFor({state:'detached',timeout:8000});
  // An independent URL must not adopt managed test or styling.
  await page.getByRole('button',{name:'Test live',exact:true}).click();
  const legacy=await context.newPage();
  await legacy.goto(`${origin}/overlay?theme=studio`); await legacy.waitForTimeout(700);
  assert.equal(await legacy.locator('.overlay-canvas').count(),0);
  await legacy.close();
  console.log('Checking async responses');
  // A read started before a mutation must not undo its Stop-test state.
  holdNextRead=true;
  for(let tries=0;!releaseRead && tries<100;tries++) await new Promise(resolve=>setTimeout(resolve,30));
  assert.ok(releaseRead);
  await page.getByRole('button',{name:'Test delay',exact:true}).click();
  releaseRead();
  await page.waitForTimeout(100);
  assert.equal(await page.getByRole('button',{name:'Stop test',exact:true}).isEnabled(),true);
  await page.getByRole('button',{name:'Stop test',exact:true}).click();
  await page.getByRole('textbox',{name:'Custom text'}).fill('RETRY SAVE');
  failNextSave=true;
  await page.getByRole('button',{name:'Apply to source',exact:true}).click();
  await page.getByRole('alert').waitFor();
  await page.waitForTimeout(1800);
  assert.match(await page.getByRole('alert').innerText(),/Save rejected/);
  await page.getByRole('button',{name:'Apply to source',exact:true}).click();
  // First launch presentation, immediate skip, then real setup wizard.
  prefs.onboarding_complete=false;
  await page.goto(`${origin}/?page=dashboard`);
  await page.locator('.welcome-intro').waitFor();
  await page.getByRole('button',{name:'Keep exploring',exact:true}).click();
  assert.match(await page.locator('.welcome-story h2').innerText(),/right distance/);
  await page.getByRole('button',{name:'Skip introduction',exact:true}).click();
  await page.locator('.onboarding').waitFor();
  assert.equal(await page.evaluate(()=>localStorage.getItem('open-delay.intro-v1')),'seen');
  console.log('Checking intro layouts');
  prefs.onboarding_complete=true;
  for (const reducedMotion of ['reduce','no-preference']) for (const width of [320,640,1100]) {
    console.log(`Intro ${width} ${reducedMotion}`);
    await page.emulateMedia({reducedMotion});
    await page.setViewportSize({width,height:820});
    await page.goto(`${origin}/?page=setup`);
    await page.getByRole('button',{name:'Replay introduction',exact:true}).click();
    await page.locator('.welcome-intro').waitFor();
    assert.ok(await page.locator('.dialog').evaluate(el=>el.scrollWidth<=el.clientWidth+1),`Intro fits ${width}/${reducedMotion}`);
    await page.getByRole('button',{name:'Keep exploring',exact:true}).click();
    await page.getByRole('button',{name:'Keep exploring',exact:true}).click();
    await page.getByRole('button',{name:'Set up my stream',exact:true}).click();
    await page.locator('.onboarding').waitFor();
  }
  assert.deepEqual(errors,[]);
  console.log('OK overlay: applied settings, linked source heartbeat, remote test, stop, expiry, legacy isolation, no media command; intro skip and setup');
} finally { await browser?.close(); await server.close(); }
