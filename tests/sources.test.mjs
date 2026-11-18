import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { stripTypeScriptTypes } from 'node:module';
import { runInNewContext } from 'node:vm';
import { setImmediate } from 'node:timers/promises';
import { presentation, strategyFailure, sourcePresentation, sourceFailure } from '../src/view.ts';
async function harness() {
  const elements = new Map(), calls = [], timers = new Map(); let timer = 0;
  const element = id => {
    if (!elements.has(id)) elements.set(id, { value: '', textContent: '', disabled: false, open: false, handlers: {}, addEventListener(event, handler) { this.handlers[event] = handler; }, append() {}, replaceChildren() {} });
    return elements.get(id);
  };
  const document = { hidden: false, getElementById: element, createElement: element, addEventListener(event, handler) { this[event] = handler; } };
  const source = stripTypeScriptTypes(await readFile(new URL('../src/main.ts', import.meta.url), 'utf8')).replace(/^import .*;\n/gm, '');
  runInNewContext(source, { document, window: { setTimeout(fn, ms) { timers.set(++timer, { fn, ms }); return timer; }, clearTimeout(id) { timers.delete(id); } }, TextEncoder, presentation, strategyFailure, sourcePresentation, sourceFailure,
    invoke(command, args) {
      if (command === 'snapshot') return Promise.resolve({ running: false, pending: 0, recording_error: null, history: [{ status: 'stopped', tick: 0, reason: 'test', recorded_at: 'test' }], writer_settings: {} });
      if (command === 'strategies') return Promise.resolve({ kind: 'versions', versions: [], next_cursor: null });
      return new Promise((resolve, reject) => calls.push({ command, args, resolve, reject }));
    },
  });
  await setImmediate();
  return { element, calls, timers, document, open() { element('sources-panel').open = true; element('sources-panel').handlers.toggle(); }, click(id) { element(id).handlers.click(); } };
}
const snapshot = (revision = '1', owner = '1') => ({ kind: 'snapshot', owner, revision, current_market: null, environment: 'demo', markets: [], bindings: [], sources: ['kalshi', 'brti', 'coinbase', 'kraken'].map(source => ({ source, enabled: false, connection: 'disconnected' })) });
test('Sources reads are active-only, serial and stale replies cannot overwrite disconnect', async () => {
  const h = await harness(); assert.equal(h.calls.length, 0); h.open();
  assert.equal(h.calls[0].args.operation, 'snapshot'); const old = h.calls.shift();
  h.click('source-brti-details'); assert.equal(h.calls.length, 0);
  h.click('source-brti-disconnect'); assert.equal(h.calls.length, 1); const action = h.calls.shift();
  action.resolve({ kind: 'action', source: 'brti', accepted: true }); await setImmediate();
  old.reject({ code: 'denied', message: 'private-marker' }); await setImmediate();
  assert.equal(h.calls.length, 1); assert.equal(h.calls[0].args.operation, 'snapshot');
  assert.equal(h.element('source-status').textContent, 'brti disconnected.');
  h.calls.shift().resolve(snapshot('9007199254740993')); await setImmediate();
  const poll = [...h.timers.values()].find(t => t.ms === 1000); assert.ok(poll);
  h.document.hidden = true; h.document.visibilitychange(); poll.fn(); assert.equal(h.calls.length, 0);
});
test('provider keys clear synchronously and unexpected errors never render submitted bytes', async () => {
  const h = await harness(); const marker = 'private-key-marker';
  h.element('provider-key-id').value = '<img src=x>';
  h.element('provider-private-key').value = marker; h.click('provider-save');
  assert.equal(h.element('provider-private-key').value, ''); assert.equal(h.calls[0].args.private_key, marker);
  assert.equal(h.element('provider-save').disabled, true);
  h.calls.shift().reject(new Error(marker)); await setImmediate();
  assert.ok(!h.element('provider-status-text').textContent.includes(marker)); assert.equal(h.element('provider-save').disabled, false);
  assert.equal(h.calls.length, 0, 'credential completion does not start hidden source reads');
});
test('details and requirements share the read slot and dynamic text remains literal', async () => {
  const h = await harness(); h.open(); h.calls.shift().resolve(snapshot()); await setImmediate();
  h.click('source-kalshi-details'); const details = h.calls.shift();
  h.element('source-version').value = '9007199254740993'; h.click('source-inspect'); assert.equal(h.calls.length, 0);
  details.resolve({ kind: 'details', source: 'kalshi', values: ['<script>old</script>'], notice: 'old' }); await setImmediate();
  assert.equal(h.element('source-details').textContent, '');
  assert.equal(h.calls[0].args.version_id, '9007199254740993');
  h.calls.shift().resolve({ kind: 'requirements', version_id: '9007199254740993', requirements: [{ input_id: '<script>literal</script>', source: 'kalshi', instrument: 'current_contract', field: 'buyable_ask', unit: 'contract_cents', side: 'yes', startup_ms: '0', implicit: false }] });
  await setImmediate(); assert.match(h.element('source-requirements').textContent, /<script>literal<\/script>/);
});
test('received values do not grant freshness, history or entitlement', () => {
  const s = snapshot(); s.sources[1] = { source: 'brti', enabled: true, connection: 'open' };
  s.bindings = [{ source: 'brti', field: 'index_value', value: '<b>68000</b>', unit: 'usd_per_btc', market_id: null, side: null, identity: 'BRTI:1', age_ms: '123', usable: false, history_ready: false, reason: 'unknown_freshness', capabilities: { current: 'unknown_freshness', history: 'unsupported_history' } }];
  const text = sourcePresentation(s, 'brti'); assert.match(text, /<b>68000<\/b>/); assert.match(text, /Freshness has not been verified/); assert.match(text, /Complete history is unverified/); assert.doesNotMatch(text, /entitlement|Current input usable/);
});
test('revision comparison stays exact above the JavaScript safe-integer limit', async () => {
  const h = await harness(); h.open();
  const newest = snapshot('9007199254740993'); newest.sources[0].connection = 'access_refused';
  h.calls.shift().resolve(newest); await setImmediate();
  const poll = [...h.timers.values()].find(t => t.ms === 1000); poll.fn();
  h.calls.shift().resolve(snapshot('9007199254740992')); await setImmediate();
  assert.match(h.element('source-kalshi-status').textContent, /explicitly refused/);
});
test('ignored Connect clicks preserve the pending request failure', async () => {
  const h = await harness(); h.open(); h.calls.shift().resolve(snapshot()); await setImmediate();
  h.click('source-kalshi-connect'); const connect = h.calls.shift();
  h.click('source-kalshi-connect');
  assert.equal(h.calls.length, 0, 'the second click sends no native request');
  connect.reject({ code: 'missing' }); await setImmediate();
  assert.equal(h.element('source-status').textContent, sourceFailure({ code: 'missing' }));
});
test('disconnect remains available during pending Connect and late action success is discarded', async () => {
  const h = await harness(); h.open(); h.calls.shift().resolve(snapshot()); await setImmediate();
  h.click('source-kalshi-connect'); const connect = h.calls.shift();
  h.click('source-kalshi-disconnect'); const disconnect = h.calls.shift();
  assert.equal(disconnect.args.operation, 'disconnect');
  disconnect.resolve({ kind: 'action', source: 'kalshi', accepted: true }); await setImmediate();
  connect.resolve({ kind: 'action', source: 'kalshi', accepted: true }); await setImmediate();
  assert.equal(h.element('source-status').textContent, 'kalshi disconnected.');
});

test('invalid first data is visible without inventing a received value', () => {
  const s = snapshot(); s.sources[2] = { source: 'coinbase', enabled: true, connection: 'disconnected' };
  s.bindings = [{ source: 'coinbase', field: 'last_trade', value: null, unit: 'usd_per_btc', market_id: null, side: null, identity: null, age_ms: null, usable: false, history_ready: false, reason: 'invalid_value', capabilities: { current: 'unknown_freshness' } }];
  const text = sourcePresentation(s, 'coinbase');
  assert.match(text, /Invalid source data/);
  assert.doesNotMatch(text, /Awaiting verified data|USD\/BTC|Last value retained/);
  s.bindings[0].value = '68000.12';
  assert.match(sourcePresentation(s, 'coinbase'), /Invalid source data/);
});

test('expired data is distinct from awaiting data and unknown freshness', () => {
  const s = snapshot(); s.sources[2] = { source: 'coinbase', enabled: true, connection: 'open' };
  s.bindings = [{ source: 'coinbase', field: 'last_trade', value: '68000.12', unit: 'usd_per_btc', market_id: null, side: null, identity: '10', age_ms: '2000', usable: false, history_ready: false, reason: 'stale_data', capabilities: { current: null } }];
  const text = sourcePresentation(s, 'coinbase');
  assert.match(text, /68000.12 USD\/BTC/);
  assert.match(text, /Stale data/);
  assert.doesNotMatch(text, /Awaiting|Freshness has not been verified/);
  s.bindings[0].reason = 'market_closed';
  assert.match(sourcePresentation(s, 'coinbase'), /Market closed/);
});

test('source environment comes from native state and credential requests cannot select it', async () => {
  const h = await harness(); h.open();
  const s = snapshot(); s.environment = 'production';
  h.calls.shift().resolve(s); await setImmediate();
  assert.match(h.element('source-environment').textContent, /Production market data.*Read-only/);
  assert.match(h.element('provider-environment').textContent, /Production API key/);
  h.element('provider-key-id').value = 'controlled'; h.element('provider-private-key').value = 'controlled';
  h.click('provider-save');
  assert.equal(Object.hasOwn(h.calls[0].args, 'environment'), false);
});
