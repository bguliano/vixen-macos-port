// Uses the owner's exact bundled client, Node's native fetch/WebSocket, and loopback only.
// Usage: node scripts/test_web_signalr.cjs http://127.0.0.1:PORT
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
let pingCount = 0;
class ObservedWebSocket extends WebSocket {
  constructor(...args) {
    super(...args);
    this.addEventListener('message', event => {
      for (const record of String(event.data).split('\x1e').filter(Boolean)) {
        if (JSON.parse(record).type === 6) ++pingCount;
      }
    });
  }
}
const sandbox = {console, WebSocket: ObservedWebSocket, fetch, Headers, Request, Response, AbortController, URL, TextEncoder, TextDecoder, setTimeout, clearTimeout, setInterval, clearInterval, performance, navigator: {userAgent: 'Vixen local test'}, document: {addEventListener() {}, removeEventListener() {}}};
sandbox.self = sandbox; sandbox.window = sandbox;
vm.runInNewContext(fs.readFileSync(require.resolve('../app/Resources/wwwroot/js/signalr.js'), 'utf8'), sandbox);
const signalR = sandbox.signalR;
const base = process.argv[2];
assert.equal(new URL(base).hostname, '127.0.0.1');
async function run() {
  const make = () => new signalR.HubConnectionBuilder().withUrl(base + '/hubs/contextStates', {
    WebSocket: ObservedWebSocket, fetch, transport: signalR.HttpTransportType.WebSockets
  }).configureLogging(signalR.LogLevel.Warning).build();
  const first = make(), second = make();
  const received = [[], []];
  [first, second].forEach((connection, index) => connection.on('updatePlayingContextStates', states => {
    assert.equal(states[0].Sequence.Name, 'Loopback Sequence');
    received[index].push(Date.now());
  }));
  try {
    await first.start(); await second.start();
    await new Promise(resolve => setTimeout(resolve, 16500));
    for (const events of received) {
      assert.ok(events.length >= 15 && events.length <= 17, `one broadcast per second: ${events.length}`);
      for (let i = 1; i < events.length; ++i) assert.ok(events[i] - events[i-1] > 800, 'no duplicate scoped timers');
    }
    assert.equal(first.state, signalR.HubConnectionState.Connected);
    assert.equal(second.state, signalR.HubConnectionState.Connected);
    assert.ok(pingCount >= 2, 'both clients receive SignalR 15-second pings');
    await first.stop();
    const before = received[1].length;
    await new Promise(resolve => setTimeout(resolve, 1200));
    assert.ok(received[1].length > before, 'remaining client still receives broadcasts');
    console.log(JSON.stringify({client: signalR.VERSION, clients: 2, counts: received.map(v => v.length), pings: pingCount, status: 'PASS'}));
  } finally { await first.stop(); await second.stop(); }
}
run().catch(error => { console.error(error); process.exitCode = 1; });
