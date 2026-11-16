import { test } from 'node:test';
import assert from 'node:assert/strict';
import { harness } from './ui-harness.mjs';

const preview = { kind: 'preview', explanation: '<script>literal explanation</script>', meaning_id: 'metten_rules_2', notice: 'Source access unverified', requirements: { support_profile: 'rules_2_initial', planned_bytes: 1, shared_underlying_bytes: 0, per_market_bytes: 1, startup_ms: {} } };
async function reviewed(h, text = '{"exact":"text"}') { h.edit(text); h.click('strategy-preview'); const call = h.pending.shift(); assert.equal(call.args.json, text); call.resolve(preview); await h.flush(); }

test('exact editor text is previewed, rendered literally, and stale previews cannot enable save', async t => {
  const h = await harness(t); h.edit('old'); h.click('strategy-preview');
  h.edit('new'); h.pending.shift().resolve(preview); await h.flush();
  assert.equal(h.element('strategy-save').disabled, true);
  assert.equal(h.element('strategy-explanation').textContent, '');
  await reviewed(h, 'new');
  assert.equal(h.element('strategy-explanation').textContent, preview.explanation);
  assert.equal(h.element('strategy-save').disabled, false);
  h.edit('new '); assert.equal(h.element('strategy-save').disabled, true);
});

test('save captures reviewed bytes, blocks duplicates, and retains committed identity on refresh failure', async t => {
  const h = await harness(t); await reviewed(h);
  h.click('strategy-save'); h.click('strategy-save');
  assert.equal(h.pending.length, 1);
  const save = h.pending.shift(); assert.equal(save.args.json, save.args.reviewed_json);
  h.edit('correction during save');
  save.resolve({ kind: 'saved', version: { id: '9007199254740993', recorded_at: 'UTC' } }); await h.flush();
  assert.match(h.element('strategy-status').textContent, /Saved immutable version 9007199254740993/);
  h.pending.shift().reject('secret-marker'); await h.flush();
  assert.match(h.element('strategy-status').textContent, /Saved immutable version/);
  assert.match(h.element('strategy-list-status').textContent, /refresh failed/);
  assert.equal(h.element('strategy-json').value, 'correction during save');
});

test('Busy preserves current preview; uncertain save directs inspection without automatic retry', async t => {
  const h = await harness(t); await reviewed(h);
  h.click('strategy-preview'); h.pending.shift().reject({ code: 'busy' }); await h.flush();
  assert.equal(h.element('strategy-save').disabled, false);
  assert.equal(h.element('strategy-explanation').textContent, preview.explanation);
  h.click('strategy-save'); h.pending.shift().reject('private transport error'); await h.flush();
  assert.match(h.element('strategy-status').textContent, /Save not confirmed — inspect saved versions before retrying/);
  assert.equal(h.pending.length, 0);
  assert.doesNotMatch(h.element('strategy-status').textContent, /private/);
});

test('file reads and edit-copy invalidate previews; stale file and detail responses are discarded', async t => {
  const h = await harness(t); await reviewed(h);
  let finishFile;
  h.element('strategy-file').files = [{ size: 4, text: () => new Promise(resolve => { finishFile = resolve; }) }];
  h.element('strategy-file').handlers.change(); assert.equal(h.element('strategy-save').disabled, true);
  h.edit('typed while reading'); finishFile('file'); await h.flush();
  assert.equal(h.element('strategy-json').value, 'typed while reading');
  h.click('strategy-reload'); h.pending.shift().resolve({ kind: 'versions', versions: [{ id: '12', display_name: '<img onerror=bad>', status: 'legacy_read_only', recorded_at: 'old' }, { id: '13', display_name: null, status: 'unsupported', recorded_at: 'old' }], next_cursor: '12' }); await h.flush();
  const buttons = h.element('strategy-versions').children.map(item => item.children[0]);
  assert.match(buttons[0].textContent, /<img onerror=bad>/);
  buttons[0].handlers.click(); buttons[1].handlers.click();
  const old = h.pending.shift(); const latest = h.pending.shift();
  const detail = id => ({ kind: 'detail', version: { summary: { id, status: 'legacy_read_only' }, json: `original ${id}`, explanation: null, meaning_id: null, notice: 'No historical explanation' } });
  latest.resolve(detail('13')); await h.flush(); old.resolve(detail('12')); await h.flush();
  assert.match(h.element('strategy-detail').textContent, /Version 13/);
  h.click('strategy-copy'); assert.equal(h.element('strategy-json').value, 'original 13');
  assert.equal(h.element('strategy-save').disabled, true); assert.equal(h.element('strategy-json').focused, true);
  h.click('strategy-more'); assert.equal(h.pending.shift().args.before_id, '12');
});

test('invalid preview keeps correction text and oversized files never get read', async t => {
  const h = await harness(t); h.edit('bad text'); h.click('strategy-preview');
  h.pending.shift().resolve({ kind: 'rule_errors', errors: [{ path: '$.name', code: 'malformed', message: 'Invalid name', accepted_form: 'nonempty name' }], truncated: false }); await h.flush();
  assert.equal(h.element('strategy-json').value, 'bad text'); assert.equal(h.element('strategy-save').disabled, true);
  assert.match(h.element('strategy-status').textContent, /\$\.name/);
  h.element('strategy-file').files = [{ size: 262145, text() { assert.fail('must reject before reading'); } }];
  h.element('strategy-file').handlers.change(); assert.match(h.element('strategy-status').textContent, /256 KiB/);
});
