// Real service + real browser-source document, isolated data and dynamic ports.
import { chromium } from 'playwright';
import { spawn } from 'node:child_process';
import { mkdtemp, readFile, mkdir } from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';

const root=path.resolve('..');
await mkdir(path.join(root,'.tmp'),{recursive:true});
const dataDir=await mkdtemp(path.join(root,'.tmp','overlay-live-'));
const service=spawn(path.join(root,'target/debug/open-delay-service.exe'),['--data-dir',dataDir,'--api-port','0','--rtmp-port','0','--ui-dir',path.join(root,'ui/dist')],{cwd:root,windowsHide:true,stdio:['ignore','pipe','pipe']});
let browser; let origin; let token; let serviceError;
service.on('error',error=>{serviceError=error;});
try {
  for(let attempt=0;attempt<100;attempt++) {
    if(serviceError) throw serviceError;
    try { origin=JSON.parse(await readFile(path.join(dataDir,'runtime.json'),'utf8')).api_base; break; } catch { await new Promise(resolve=>setTimeout(resolve,200)); }
  }
  assert.ok(origin,'Isolated service starts');
  const session=await fetch(`${origin}/api/v1/session`,{headers:{Origin:origin}});
  assert.equal(session.status,200);
  token=(await session.json()).token;
  const headers={'Content-Type':'application/json','X-OpenDelay-Token':token};
  const prefs=await (await fetch(`${origin}/api/v1/preferences`,{headers})).json();
  const prefResponse=await fetch(`${origin}/api/v1/preferences`,{method:'POST',headers,body:JSON.stringify({...prefs,language:'en',onboarding_complete:true})});
  assert.equal(prefResponse.status,200);
  browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH||'C:/Program Files/Google/Chrome/Application/chrome.exe',headless:true});
  const context=await browser.newContext({viewport:{width:1100,height:900},reducedMotion:'reduce'});
  await context.addInitScript(()=>localStorage.setItem('open-delay.language','en'));
  const errors=[]; const commands=[];
  const editor=await context.newPage(); editor.on('pageerror',error=>errors.push(error.message));
  editor.on('request',request=>{if(new URL(request.url()).pathname==='/api/v1/commands') commands.push(request.postData());});
  await editor.goto(`${origin}/?page=overlay`);
  await editor.getByRole('button',{name:'Content & behavior'}).click();
  await editor.getByRole('textbox',{name:'Custom text'}).fill('REAL SOURCE');
  await editor.getByRole('checkbox',{name:'Use my text in the badge'}).check();
  await editor.getByRole('button',{name:'Apply to source',exact:true}).click();
  await editor.getByText('Settings applied to the linked source.',{exact:true}).waitFor();
  assert.equal(JSON.parse(await readFile(path.join(dataDir,'overlay.json'),'utf8')).text,'REAL SOURCE');
  const source=await context.newPage(); source.on('pageerror',error=>errors.push(error.message));
  await source.goto(`${origin}/overlay?managed=true`);
  await editor.getByText('Source recently active',{exact:true}).waitFor();
  await editor.getByRole('button',{name:'Test delay',exact:true}).click();
  await source.locator('.overlay-popup').waitFor();
  assert.match(await source.locator('.overlay-popup').innerText(),/REAL SOURCE/);
  await editor.getByRole('button',{name:'Stop test',exact:true}).click();
  await source.locator('.overlay-canvas').waitFor({state:'detached'});
  await editor.getByRole('button',{name:'Test building',exact:true}).click();
  await source.locator('.overlay-canvas').waitFor();
  await source.locator('.overlay-canvas').waitFor({state:'detached',timeout:8000});
  assert.deepEqual(commands,[]);
  assert.deepEqual(errors,[]);
  console.log('OK REAL SERVICE: editor → authenticated API → persisted settings → independent browser source; test, stop, expiry; no media commands');
} finally {
  await browser?.close();
  if(origin && token) await fetch(`${origin}/api/v1/shutdown`,{method:'POST',headers:{'X-OpenDelay-Token':token}}).catch(()=>undefined);
  if(service.exitCode===null) service.kill();
}
