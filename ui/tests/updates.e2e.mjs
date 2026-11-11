import { chromium } from 'playwright';
import { createServer } from 'vite';
import assert from 'node:assert/strict';
const server=await createServer({server:{host:'127.0.0.1',port:0,strictPort:false},logLevel:'error'});
let browser;
const preferences={language:'en',mode:'instant',buffer_ms:60000,disk_bytes:2147483648,cache_bytes:67108864,api_port:49154,rtmp_port:49155,lan_ingest:false,startup:false,quit_on_close:false,onboarding_complete:true,hotkeys:{start_delay:null,go_live:null,toggle_overlay:null,custom_delay:null,custom_delay_ms:60000}};
const runtime={id:'custom-id',is_connected:true,generation:1,transport_state:'connected',effective_mode:'live',desired_delay_ms:0,effective_delay_ms:0,buffer:{fill_ms:60000,target_ms:60000,ready:true,degraded:false,size_bytes:0},bytes_received:0,bytes_sent:0,error:null};
const status={sequence:1,version:'0.2.0',mode:'instant',effective_mode:'live',is_connected:true,desired_delay_ms:0,effective_delay_ms:0,transition_in_progress:false,overlay_hidden:false,routes:[runtime],preferences};
const output={id:'custom-id',name:'Custom Twitch',platform:'twitch',local_app:'custom-path',upstream_url:'rtmp://live.twitch.tv/app',enabled:true,include_in_group:false};
try {
  await server.listen(); const origin=`http://127.0.0.1:${server.httpServer.address().port}`;
  browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH||'C:/Program Files/Google/Chrome/Application/chrome.exe',headless:true});
  const context=await browser.newContext({viewport:{width:1100,height:820},reducedMotion:'reduce'});
  context.setDefaultTimeout(10000);
  await context.addInitScript(({origin,status})=>{
    localStorage.setItem('open-delay.language','en'); localStorage.setItem('open-delay.intro-v1','seen');
    const callbacks=new Map(); const listeners=new Map(); let nextId=1;
    let update={configured:true,phase:'idle',current_version:'0.2.0',version:null,notes:null,downloaded:0,total:null,error:null};
    window.nativeCalls=[];
    window.emitNative=(event,payload)=>{for(const [id,entry] of listeners) if(entry.event===event) callbacks.get(entry.handler)?.({event,id,payload});};
    window.__TAURI_EVENT_PLUGIN_INTERNALS__={unregisterListener:(_event,id)=>listeners.delete(id)};
    window.__TAURI_INTERNALS__={
      transformCallback:callback=>{const id=nextId++;callbacks.set(id,callback);return id;},
      unregisterCallback:id=>callbacks.delete(id),
      invoke:async(command,args)=>{
        window.nativeCalls.push(command);
        if(command==='plugin:event|listen'){const id=nextId++;listeners.set(id,args);return id;}
        if(command==='plugin:event|unlisten') return;
        if(command==='service_info') return {api_base:origin,error:null,native_errors:[]};
        if(command==='desktop_status') return {errors:[]};
        if(command==='update_status') return update;
        if(command==='check_update'){update={...update,phase:'available',version:'0.3.0',notes:'<script id="unsafe">not executable</script>'};window.emitNative('update-status',update);const response=update;await new Promise(resolve=>setTimeout(resolve,600));return response;}
        if(command==='install_update'){update={...update,phase:'downloading',downloaded:1024,total:2048};window.emitNative('update-status',update);await new Promise(resolve=>setTimeout(resolve,1200));update={...update,phase:'error',error:'Signature verification failed (test)'};window.emitNative('update-status',update);return update;}
        throw new Error(`Unexpected native call: ${command}`);
      },
    };
    window.nativeStatusFixture=status;
  },{origin,status});
  await context.route('**/api/v1/**',async route=>{
    const url=new URL(route.request().url());let data;
    if(url.pathname.endsWith('/session')) data={token:'isolated'};
    else if(url.pathname.endsWith('/events')) return route.fulfill({status:503,body:'unavailable'});
    else if(url.pathname.endsWith('/status')) data={...status,endpoints:{api_base:origin,rtmp_port:49155}};
    else if(url.pathname.endsWith('/preferences')) data=preferences;
    else if(url.pathname.endsWith('/routes')) data={routes:[output]};
    else throw new Error(`Unexpected API ${url.pathname}`);
    await route.fulfill({status:200,contentType:'application/json',body:JSON.stringify(data)});
  });
  const page=await context.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));
  await page.goto(`${origin}/?page=settings`);
  const install=page.getByRole('button',{name:'Download and install',exact:true});await install.waitFor();
  await page.getByText('Stop all outputs before installing an update.',{exact:true}).waitFor();
  assert.equal(await install.isDisabled(),true,'Even an output outside the group blocks installing');
  assert.equal(await page.evaluate(()=>window.nativeCalls.includes('install_update')),false);
  await page.evaluate(()=>window.emitNative('desktop-status',{errors:['Shortcut already registered']}));
  await page.getByText('Shortcut already registered',{exact:true}).waitFor();
  await page.evaluate(()=>window.emitNative('desktop-status',{errors:[]}));
  await page.evaluate(()=>window.emitNative('command-result',{success:false,reason:'buffer_not_ready',status:window.nativeStatusFixture,results:[]}));
  await page.getByText('buffer_not_ready',{exact:true}).waitFor();
  runtime.is_connected=false;status.is_connected=false;
  await page.goto(`${origin}/?page=settings`);
  await install.waitFor();await install.click();
  await page.getByRole('button',{name:'Install and restart',exact:true}).click();
  await page.waitForTimeout(700);
  assert.equal(await page.locator('.update-progress').isVisible(),true,'A delayed check response must not rewind download progress');
  await page.getByText('Signature verification failed (test)',{exact:true}).waitFor();
  assert.equal(await page.locator('#unsafe').count(),0,'Release notes are rendered as text');
  // The existing custom route must retain its real ingest path in setup.
  preferences.onboarding_complete=false;
  await page.goto(`${origin}/?page=dashboard`);
  await page.getByRole('button',{name:'Continue',exact:true}).click();
  await page.getByRole('button',{name:'Continue',exact:true}).click();
  await page.getByText('rtmp://127.0.0.1:49155/custom-path',{exact:true}).waitFor();
  assert.deepEqual(errors,[]);
  console.log('OK native UI bridge: startup check, active-output install guard, errors/events, install confirmation, safe notes, custom route ingest path');
}finally{await browser?.close();await server.close();}
