import test from 'node:test';
import assert from 'node:assert/strict';
import { localBase, normalizeSettings, commandFor, buttonState, OpenDelayClient, parseStreamDeckEvent, StatusCursor } from '../src/client.mjs';

const route = { id:'custom',is_connected:true,include_in_group:true,transport_state:'connected',effective_mode:'live',effective_delay_ms:0,buffer:{degraded:false} };
const status = { sequence:1,is_connected:true,effective_mode:'live',effective_delay_ms:0,transition_in_progress:false,routes:[route] };
test('settings enforce integer bounds and zero always means live', () => {
  for (const value of [-1,601,1.5,NaN,'',null,true,false,'0x10','1e2']) assert.throws(() => normalizeSettings({ delay_seconds:value }));
  for (const value of [null, [], 'bad']) assert.throws(() => normalizeSettings(value));
  assert.deepEqual(commandFor('dev.opendelay.setdelay',{ delay_seconds:0 },'a'),{ id:'a',action:'go_live' });
  assert.deepEqual(commandFor('dev.opendelay.setdelay',{ delay_seconds:30,route_id:'custom' },'b'),{id:'b',action:'prepare_delay',delay_ms:30000,route_ids:['custom']});
  assert.throws(() => normalizeSettings({ route_id:'../bad' }));
});
test('only loopback discovery origins are admitted', () => {
  assert.equal(localBase('http://127.0.0.1:54123'),'http://127.0.0.1:54123');
  for (const value of ['https://127.0.0.1:54123','http://example.com:54123','http://127.0.0.1:54123/path','http://user@127.0.0.1:54123','http://127.0.0.1:54123/?token=x']) assert.throws(() => localBase(value));
});

test('malformed Stream Deck events cannot terminate the plugin', () => {
  for (const raw of ['null','[]','"bad"','{}','{','{"event":"keyDown","context":"key","action":"not-ours"}']) {
    assert.equal(parseStreamDeckEvent(raw),null);
  }
  assert.deepEqual(parseStreamDeckEvent('{"event":"keyDown","context":"key","action":"dev.opendelay.golive"}'),
    {event:'keyDown',context:'key',action:'dev.opendelay.golive'});
});

test('obsolete session completion cannot replace a newer discovery epoch at the same address', async () => {
  let runtime={name:'Open-Delay',api_base:'http://127.0.0.1:54123',pid:1};
  const requests=[];
  const client=new OpenDelayClient({readFileImpl:async()=>JSON.stringify(runtime),fetchImpl:url=>new Promise(resolve=>requests.push({url,resolve}))});
  const firstBase=await client.discover();const old=client.session(firstBase);const oldRejected=assert.rejects(old,/Engine changed/);
  runtime={...runtime,api_base:'http://127.0.0.1:54124',pid:2};await client.discover();
  runtime={...runtime,api_base:firstBase,pid:3};await client.discover();
  const fresh=client.session(firstBase);const active=client.sessionPromise;
  requests[0].resolve({ok:true,json:async()=>({token:'obsolete-token'})});
  await oldRejected;
  assert.equal(client.sessionPromise,active);
  requests[1].resolve({ok:true,json:async()=>({token:'current-token'})});
  assert.equal(await fresh,'current-token');assert.equal(client.token,'current-token');
});
test('button titles reflect actual selected output state', () => {
  assert.equal(buttonState(null).title,'OFF');
  assert.equal(buttonState(status).title,'LIVE');
  assert.equal(buttonState({...status,routes:[{...route,effective_mode:'building_delay'}]}).title,'BUILD');
  assert.equal(buttonState({...status,routes:[{...route,transport_state:'recovering'}]}).title,'REC');
  assert.equal(buttonState({...status,routes:[{...route,buffer:{degraded:true}}]}).title,'DEG');
  assert.equal(buttonState({...status,routes:[{...route,effective_delay_ms:15000}]}).title,'15s');
  assert.equal(buttonState(status,{route_id:'missing'}).title,'OFF');
});

test('late polls cannot regress newer command status, while a new engine may reset its sequence', () => {
  const cursor=new StatusCursor();
  assert.equal(cursor.accept({...status,sequence:10},1),true);
  assert.equal(cursor.accept({...status,sequence:9},1),false);
  assert.equal(cursor.accept({...status,sequence:1},2),true);
  assert.equal(cursor.accept({...status,sequence:99},1),false);
});
test('dynamic discovery resets sessions, uses header tokens, does not retry commands', async () => {
  let base='http://127.0.0.1:54123'; const calls=[];
  const client=new OpenDelayClient({readFileImpl:async()=>JSON.stringify({name:'Open-Delay',api_base:base,pid:1}),fetchImpl:async(url,options)=>{
    calls.push({url,options});
    if(url.endsWith('/session')) return {ok:true,json:async()=>({token:'session-secret'})};
    return {ok:true,json:async()=>status};
  }});
  await client.status(); base='http://127.0.0.1:54124'; await client.status();
  assert.equal(calls.filter(call=>call.url.endsWith('/session')).length,2);
  assert(calls.every(call=>!call.url.includes('secret')));
  assert.equal(calls[1].options.headers['X-OpenDelay-Token'],'session-secret');
  let attempts=0;client.fetchImpl=async()=>{attempts++;throw new Error('transport lost');};
  await assert.rejects(client.command({id:'one',action:'toggle_delay',delay_ms:15000}));assert.equal(attempts,1);
});
