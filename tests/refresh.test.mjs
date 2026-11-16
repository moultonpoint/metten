import { test } from 'node:test';
import assert from 'node:assert/strict';
import { harness } from './ui-harness.mjs';

test('failed refresh does not assert stopped and still lets the user attempt Pause', async t => {
  const controls = [];
  let reads = 0;
  const h = await harness(t, {
    control(args) { controls.push(args.action); },
    snapshot() {
      if (++reads > 1) throw new Error('Controlled status-read failure');
      return {
        running: true, pending: 0, recording_error: null,
        history: [{ status: 'running', tick: 1, reason: 'Started', recorded_at: '2026-11-13T00:00:00Z' }],
        writer_settings: {},
      };
    },
  });
  const { element } = h;
  await h.flush();
  assert.match(element('status').textContent, /Running/);
  h.timers.shift().callback();
  await h.flush();
  assert.equal(element('status').textContent, 'Session status unavailable');
  assert.equal(element('start').disabled, true);
  assert.equal(element('advance').disabled, true);
  assert.equal(element('pause').disabled, false);
  assert.deepEqual(controls, [], 'a failed read has not stopped the engine');
  element('pause').handlers.click();
  await h.flush();
  assert.deepEqual(controls, ['pause']);
});

for (const olderFails of [false, true]) {
  for (const newerFails of [false, true]) {
    test(`older status ${olderFails ? 'failure' : 'success'} cannot overwrite newer ${newerFails ? 'failure' : 'Pause'}`, async t => {
      const reads = [];
      const h = await harness(t, {
        snapshot: () => new Promise((resolve, reject) => reads.push({ resolve, reject })),
      });
      const { element, elements } = h;
      element('pause').handlers.click();
      await h.flush();
      assert.equal(reads.length, 2);
      const snapshot = status => ({
        running: status === 'running', pending: 0, recording_error: null,
        history: [{ status, tick: status === 'running' ? 1 : 2, reason: status, recorded_at: '2026-11-13T00:00:00Z' }],
        writer_settings: {},
      });
      if (newerFails) reads[1].reject('Latest status failure');
      else reads[1].resolve(snapshot('paused'));
      await h.flush();
      assert.equal(element('status').textContent, newerFails ? 'Session status unavailable' : 'Paused · no test events running');
      const displayed = [...elements].map(([id, value]) => [id, value.textContent, value.disabled]);
      if (olderFails) reads[0].reject('Outdated status failure');
      else reads[0].resolve(snapshot('running'));
      await h.flush();
      assert.deepEqual([...elements].map(([id, value]) => [id, value.textContent, value.disabled]), displayed);
      assert.equal(element('advance').disabled, true);
      assert.equal(element('pause').disabled, newerFails ? false : true);
    });
  }
}
