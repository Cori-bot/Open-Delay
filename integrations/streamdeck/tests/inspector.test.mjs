import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { runInNewContext } from 'node:vm';

test('property inspector reports malformed registration without throwing or opening a socket', async () => {
  const nodes=new Map(['delay','route','message'].map(id=>[id,{value:'',textContent:'',addEventListener(){}}]));
  const window={};let sockets=0;
  const WebSocket=class { constructor(){sockets++;} };
  const source=await readFile(new URL('../dev.opendelay.sdPlugin/ui/settings.js',import.meta.url),'utf8');
  runInNewContext(source,{window,document:{getElementById:id=>nodes.get(id)},WebSocket});
  for (const input of ['null','{','{}','{"context":"key","payload":{"settings":null}}']) {
    assert.doesNotThrow(()=>window.connectElgatoStreamDeckSocket(49155,'uuid','registerPropertyInspector','{}',input));
    assert.match(nodes.get('message').textContent,/Invalid/);
  }
  assert.equal(sockets,0);
});
