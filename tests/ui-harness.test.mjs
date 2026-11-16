import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { harness } from './ui-harness.mjs';

test('startup finishes and fixtures isolate elements, calls, pending replies and timers', async t => {
  const first = await harness(t);
  assert.deepEqual(first.calls.map(call => call.command), ['snapshot', 'strategies']);
  assert.equal(first.calls[0].args, undefined);
  assert.equal(first.calls[1].args.operation, 'list');
  assert.equal(first.pending.length, 0);
  assert.equal(first.element('status').textContent, 'Stopped · ready when you are');
  assert.equal(first.element('strategy-list-status').textContent, 'No saved versions.');
  assert.equal(first.timers.length, 1);
  assert.equal(first.timers[0].delay, 200);
  first.edit('first fixture');
  first.click('strategy-preview');
  assert.equal(first.pending.length, 1);
  const second = await harness(t);
  assert.notEqual(first.element('strategy-json'), second.element('strategy-json'));
  assert.equal(second.element('strategy-json').value, '');
  assert.equal(second.calls.length, 2);
  assert.equal(second.pending.length, 0);
  assert.equal(second.timers.length, 1);
  first.timers.shift();
  assert.equal(second.timers.length, 1);
  first.pending.shift().resolve({ kind: 'rule_errors', errors: [], truncated: false });
  await first.flush();
  assert.equal(second.element('strategy-status').textContent, '');
});

test('deferred native replies and polling are controlled by the test', async t => {
  const h = await harness(t);
  h.edit('exact bytes'); h.click('strategy-preview');
  const preview = h.pending.shift();
  assert.equal(preview.command, 'strategies');
  assert.equal(preview.args.operation, 'preview');
  assert.equal(preview.args.json, 'exact bytes');
  assert.equal(h.element('strategy-preview').disabled, true);
  preview.reject({ code: 'busy' }); await h.flush();
  assert.match(h.element('strategy-status').textContent, /already in progress/);
  h.timers.shift().callback(); await h.flush();
  assert.equal(h.calls.filter(call => call.command === 'snapshot').length, 2);
  assert.equal(h.timers.length, 1);
});

test('an unknown native command fails the owning test even if its rejection is caught', () => {
  const script = `import { test } from 'node:test';
    import { harness } from ${JSON.stringify(new URL('./ui-harness.mjs', import.meta.url).href)};
    test('unknown command probe', async t => {
      const h = await harness(t);
      await h.invoke('unexpected-native-command').catch(() => {});
    });`;
  const result = spawnSync(process.execPath, ['--input-type=module', '--eval', script], { encoding: 'utf8', timeout: 10000 });
  assert.ifError(result.error);
  assert.equal(result.status, 1, result.stdout + result.stderr);
  assert.match(result.stdout + result.stderr, /Unexpected native command: unexpected-native-command/);
});
