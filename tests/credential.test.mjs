import { test } from 'node:test';
import assert from 'node:assert/strict';
import { harness } from './ui-harness.mjs';

test('credential submission clears input immediately and hides unexpected framework errors', async t => {
  const h = await harness(t);
  const { element } = h;
  const marker = 'metten-dummy-test-only-marker';
  element('credential-value').value = marker;
  element('credential-save').handlers.click();
  const request = h.pending.shift();
  assert.equal(request.command, 'credential');
  const submitted = request.args;
  assert.equal(element('credential-value').value, '');
  assert.equal(submitted.value, marker);
  assert.equal(element('credential-save').disabled, true);
  request.reject(`unexpected framework failure with ${marker}`);
  await h.flush();
  assert.ok(!element('credential-status').textContent.includes(marker));
  assert.match(element('credential-status').textContent, /reopen/);
  assert.equal(element('credential-save').disabled, false);
});
