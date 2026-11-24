import { test } from 'node:test';
import assert from 'node:assert/strict';
import { harness } from './ui-harness.mjs';
import { paperMoney, paperQuantity, paperReceipt } from '../src/view.ts';
const run = overrides => ({ pending_holdings: true, run_id: '1', start_uuid: '00000000-0000-4000-8000-000000000001', strategy_version_id: '1', revision: '1', history_revision: '1', control_state: 'active', cash: '9736870000', reserved: '0', virtual_frontier_ms: '61000', last_evaluated_through_ms: '61000', completed_ordinal: '8', pending_ordinal: null, error_code: null, quantity: '10', minimum_price: '15.123456', maximum_price: '90', starting_balance: '10000000000', fee_model: 'controlled_general_taker_2026_07_07_m1', simulation_model: 'controlled_best_ask_finite_ioc_v2', scenario_revision: 'controlled_learning_v1', go_utc_ms: '1795046400000', ...overrides });
const snapshot = (current, overrides = {}) => ({ kind: 'snapshot', run: current, effective_paused: false, effective_generation: '0', origin: 'synthetic_controlled', clock_multiplier: '20', ...overrides });
const receipt = overrides => ({ run_id: '1', strategy_version_id: '1', paper_market_id: '2', source_market_id: 'KXBTC15M-PAPER-NEXT', source_contract_id: 'paper-next', series_id: 'btc15m', open_utc_ms: '1795046401000', close_utc_ms: '1795047301000', close_day: '20776', reason: 'confirmed_full_fill_pending_settlement', requested_quantity: '10', attempt_state: 'full', side: 'no', price: '25000000', available_millionths: '100000000', attempt_virtual_ms: '61000', attempt_count: '1', filled_quantity: '10', cost: '250000000', fee: '13130000', debit: '263130000', payout: null, settled_net: null, winning_side: null, settlement_virtual_ms: null, settlement_recorded_at: null, ...overrides });
const history = rows => ({ kind: 'history', page: { markets: rows, next_cursor: null } });
async function open(h) { h.element('paper-panel').open = true; h.element('paper-panel').handlers.toggle(); await h.flush(); }

test('paper formatting preserves fractional cents and unknown outcomes never assert zero financial effects', () => {
  assert.equal(paperMoney('263130000'), '$2.6313');
  assert.equal(paperMoney('10736870000'), '$107.3687');
  assert.equal(paperMoney('9223372036854775807'), '$92233720368.54775807');
  assert.equal(paperMoney('-263130000'), '−$2.6313');
  assert.equal(paperQuantity('9999999'), '9.999999');
  const pending = paperReceipt(receipt());
  assert.match(pending.title, /pending settlement/);
  assert.equal(pending.facts.find(([label]) => label === 'Settled net')[1], 'Pending settlement');
  const unknown = paperReceipt(receipt({ attempt_state: 'intent', cost: '0', fee: '0', debit: '0', filled_quantity: '0' }));
  assert.match(unknown.title, /unconfirmed/);
  assert.equal(unknown.facts.find(([label]) => label === 'Confirmed quantity')[1], 'Unconfirmed');
  assert.equal(unknown.facts.some(([label]) => label === 'Confirmed cost'), false);
  assert.equal(unknown.facts.some(([label]) => label === 'Confirmed total debit'), false);
});

test('partial and multi-attempt receipts show confirmed totals and label latest-attempt facts', () => {
  const partial = paperReceipt(receipt({ attempt_state: 'partial', reason: 'confirmed_partial_fill_pending_settlement', attempt_count: '2', filled_quantity: '7', cost: '175000000', fee: '9190000', debit: '184190000' }));
  assert.equal(partial.title, 'Bought 7 of 10 NO · pending settlement');
  assert.match(partial.reason, /unfilled remainder/);
  const facts = Object.fromEntries(partial.facts);
  assert.equal(facts['Attempts'], '2');
  assert.equal(facts['Latest attempt price per contract'], '$0.25');
  assert.equal(facts['Confirmed fee'], '$0.0919');
  assert.equal(facts['Payout'], 'Pending settlement');
  // A later unresolved attempt never hides contracts already confirmed.
  const later = Object.fromEntries(paperReceipt(receipt({ attempt_state: 'intent', attempt_count: '2', filled_quantity: '7', cost: '175000000', fee: '9190000', debit: '184190000' })).facts);
  assert.equal(later['Confirmed quantity'], '7');
  assert.equal(later['Confirmed cost'], '$1.75');
  assert.equal(Object.hasOwn(Object.fromEntries(paperReceipt(receipt({ attempt_state: null, attempt_count: '0', price: null })).facts), 'Attempts'), false);
});

test('paper reads only while visible and settings reflect the immutable active run', async t => {
  let reads = 0;
  const h = await harness(t, { paper(args) { if (args.operation === 'snapshot') { reads++; return snapshot(run()); } return history([receipt()]); } });
  assert.equal(reads, 0); await open(h); assert.equal(reads, 1);
  assert.equal(h.element('paper-settings').disabled, true);
  assert.equal(h.element('paper-minimum').value, '15.123456');
  assert.equal(h.element('paper-go').disabled, true);
  h.document.hidden = true; h.document.handlers.visibilitychange();
  for (const timer of h.timers.slice()) timer.callback(); await h.flush();
  assert.equal(reads, 1);
});

test('unconfirmed Go retries preserve exact settings and start UUID', async t => {
  const starts = [];
  const h = await harness(t, { paper(args) {
    if (args.operation === 'snapshot') return snapshot(null);
    if (args.operation === 'history') return history([]);
    return new Promise((resolve, reject) => starts.push({ args, resolve, reject }));
  } });
  await open(h); h.element('paper-version').value = '1'; h.element('paper-version').handlers.change();
  for (const [id, value] of [['quantity', '10'], ['minimum', '25.000001'], ['maximum', '30'], ['balance', '100.00000001']]) h.element(`paper-${id}`).value = value;
  h.element('paper-loss-limit').value = '12.345';
  h.element('paper-profit-target').value = '5';
  h.element('paper-next-day').checked = true;
  h.click('paper-go'); await h.flush();
  assert.match(h.element('paper-status').textContent, /waiting for confirmation/);
  assert.equal(h.element('paper-settings').disabled, true);
  starts[0].reject({ code: 'storage', detail: 'private-marker' }); await h.flush();
  h.element('paper-quantity').value = '999'; h.click('paper-go'); await h.flush();
  assert.equal(JSON.stringify(starts[1].args), JSON.stringify(starts[0].args));
  assert.equal(starts[1].args.settings.starting_balance, '100.00000001');
  assert.equal(starts[1].args.settings.maximum_loss, '12.345');
  assert.equal(starts[1].args.settings.profit_target, '5');
  assert.equal(starts[1].args.settings.resume_next_day, true);
  starts[1].reject({ code: 'busy' }); await h.flush();
  assert.doesNotMatch(h.element('paper-error').textContent, /private-marker/);
});

test('an empty snapshot begun during Go cannot erase its confirmed run or disable Pause', async t => {
  let readCount = 0, stale, start;
  let current = snapshot(null);
  const h = await harness(t, { paper(args) {
    if (args.operation === 'snapshot') { if (++readCount === 2) return new Promise(resolve => { stale = resolve; }); return current; }
    if (args.operation === 'history') return history([]);
    return new Promise(resolve => { start = resolve; });
  } });
  await open(h); h.element('paper-version').value = '1'; h.element('paper-version').handlers.change();
  h.click('paper-go'); await h.flush(); h.click('paper-refresh'); await h.flush();
  current = snapshot(run({ control_state: 'waiting' })); start(current); await h.flush();
  assert.equal(h.element('paper-pause').disabled, false);
  stale(snapshot(null)); await h.flush();
  assert.match(h.element('paper-status').textContent, /Waiting for the next full market/);
  assert.equal(h.element('paper-pause').disabled, false);
});

for (const fails of [false, true]) test(`stale paper read ${fails ? 'failure' : 'success'} cannot undo Pause`, async t => {
  let count = 0, stale; let current = snapshot(run());
  const h = await harness(t, { paper(args) {
    if (args.operation === 'snapshot') { if (++count === 2) return new Promise((resolve, reject) => { stale = { resolve, reject }; }); return current; }
    if (args.operation === 'history') return history([receipt()]);
    if (args.operation === 'pause') { current = snapshot(run({ revision: '2', control_state: 'paused' }), { effective_paused: true, effective_generation: '1' }); return current; }
    throw new Error('Unexpected operation');
  } });
  await open(h); h.click('paper-refresh'); await h.flush(); h.click('paper-pause'); await h.flush();
  if (fails) stale.reject({ code: 'storage', detail: 'old private-marker' }); else stale.resolve(snapshot(run()));
  await h.flush();
  assert.match(h.element('paper-status').textContent, /Purchases paused/);
  assert.equal(h.element('paper-pause').disabled, false);
  assert.doesNotMatch(h.element('paper-error').textContent, /private-marker|Recording/);
});

test('journal selection retains its button and evidence cursors expose bounded older pages', async t => {
  const entry = receipt({ source_market_id: '<img src=x onerror=privateMarker>' });
  const cursors = [];
  const h = await harness(t, { paper(args) {
    if (args.operation === 'snapshot') return snapshot(run());
    if (args.operation === 'history') return history([entry]);
    if (args.operation === 'market') return { kind: 'market', receipt: entry, assumptions: { origin: 'synthetic_controlled' } };
    if (args.operation === 'evidence') { cursors.push(args.before_evidence_id); return { kind: 'evidence', page: { evidence: [{ evidence_id: args.before_evidence_id ? '1' : '22', witness: { literal: '<script>privateMarker</script>' } }], next_cursor: args.before_evidence_id ? null : '22' } }; }
    throw new Error('Unexpected operation');
  } });
  await open(h); const button = h.element('paper-history').children[0].children[0]; button.handlers.click(); await h.flush();
  assert.equal(h.element('paper-history').children[0].children[0], button);
  assert.match(button.textContent, /<img src=x onerror=privateMarker>/);
  assert.equal(h.element('paper-evidence-more').disabled, false);
  h.click('paper-evidence-more'); await h.flush();
  assert.deepEqual(cursors, [undefined, '22']);
  assert.equal(h.element('paper-evidence-more').disabled, true);
  assert.match(h.element('paper-detail').textContent, /Older retained evidence/);
});

test('expanded paper details refresh when the selected receipt settles', async t => {
  let current = receipt(); let revision = '1';
  const h = await harness(t, { paper(args) {
    if (args.operation === 'snapshot') return snapshot(run({ revision, history_revision: revision }));
    if (args.operation === 'history') return history([current]);
    if (args.operation === 'market') return { kind: 'market', receipt: current, assumptions: {} };
    if (args.operation === 'evidence') return { kind: 'evidence', page: { evidence: [], next_cursor: null } };
    throw new Error('Unexpected operation');
  } });
  await open(h); h.element('paper-details').open = true; h.element('paper-details').handlers.toggle(); await h.flush();
  assert.match(h.element('paper-detail').textContent, /"settled_net": null/);
  current = receipt({ reason: 'settled', settled_net: '736870000', payout: '1000000000' }); revision = '2';
  h.click('paper-refresh'); await h.flush();
  assert.match(h.element('paper-result-title').textContent, /settled/);
  assert.match(h.element('paper-detail').textContent, /"settled_net": "736870000"/);
});

test('main reasons name retained checks, unavailable inputs and compound results without evaluating rules', () => {
  const witness = { option_id: 'opposite', checkpoint_id: null, path: 'purchase_options.0.when.conditions.1', root_path: 'purchase_options.0.when', root_kind: 'any', root_status: 'false', side: 'no', kind: 'compare', status: 'false', interval_state: 'within', interval_start_seconds: '60', interval_end_seconds: '120', blocked_by_checkpoint: null, left_reference: 'ask', right_reference: null, observed: '45', required: '30', comparison: 'at_most', unit: 'contract_cents', evaluated_at_ms: '61000', transaction: '7' };
  const view = paperReceipt(receipt({ attempt_state: null, reason: 'condition_false', primary_witness: witness }));
  assert.match(view.reason, /Option opposite/); assert.match(view.reason, /45¢; required at most 30¢/); assert.match(view.reason, /enclosing any-of check was false/); assert.match(view.reason, /61000 ms/);
  const unavailable = paperReceipt(receipt({ attempt_state: null, reason: 'unavailable', primary_witness: { ...witness, status: 'unavailable', observed: null } }));
  assert.match(unavailable.reason, /could not evaluate.*ask was unavailable/);
  const checkpoint = paperReceipt(receipt({ reason: 'unavailable', primary_witness: { ...witness, blocked_by_checkpoint: 'early' } }));
  assert.match(checkpoint.reason, /checkpoint early was not latched/);
  assert.doesNotMatch(paperReceipt(receipt({ primary_witness: witness })).reason, /45¢/);
});

test('failed current reads retain receipts and Pause without claiming purchases stopped', async t => {
  let failed = false;
  const h = await harness(t, { paper(args) {
    if (args.operation === 'snapshot') { if (failed) throw { code: 'storage', detail: 'private-marker' }; return snapshot(run()); }
    return history([receipt()]);
  } });
  await open(h); failed = true; h.click('paper-refresh'); await h.flush();
  assert.match(h.element('paper-status').textContent, /unavailable/);
  assert.equal(h.element('paper-pause').disabled, false);
  assert.equal(h.element('paper-go').disabled, true);
  assert.match(h.element('paper-result-title').textContent, /Bought 10 NO/);
  assert.match(h.element('paper-error').textContent, /Pause purchases and refresh/);
  assert.doesNotMatch(h.element('paper-error').textContent, /fenced|stopped|private-marker/);
});

test('purchase checks display exact retained prices, funding and observation time', () => {
  const check = { evaluated_at_ms: '61000', offered_price: '25', minimum_price: '26', maximum_price: '100', available_funds: '100000000', required_debit: '263130000' };
  const view = paperReceipt(receipt({ attempt_state: null, reason: 'insufficient_funds', purchase_check: check }));
  assert.match(view.reason, /Full debit required \$2.6313; available \$1.00/);
  assert.match(view.reason, /61000 ms.*25¢; permitted range 26–100¢/);
  assert.doesNotMatch(paperReceipt(receipt({ purchase_check: check })).reason, /Full debit required/);
});

test('an unconfirmed recording operation preserves a separately confirmed purchase receipt', async t => {
  const h = await harness(t, { paper(args) { return args.operation === 'snapshot' ? snapshot(run({ control_state: 'unresolved', pending_ordinal: '9', error_code: 'storage' })) : history([receipt()]); } });
  await open(h);
  assert.match(h.element('paper-status').textContent, /operation remains unconfirmed/);
  assert.match(h.element('paper-fixed').textContent, /input remains unconfirmed/);
  assert.match(h.element('paper-result-title').textContent, /Bought 10 NO/);
});


test('current pending holding remains actionable when older history replaces its journal page', async t => {
  const h = await harness(t, { paper(args) {
    if (args.operation === 'snapshot') return snapshot(run({ run_id:'2',control_state:'stopped', pending_holdings:true }));
    if (args.operation === 'history') return args.before_paper_market_id
      ? history([receipt({run_id:'1',paper_market_id:'1',attempt_state:'zero',settled_net:'0'})])
      : {kind:'history',page:{markets:[receipt({run_id:'2'})],next_cursor:'2'}};
    throw new Error('Unexpected operation');
  } });
  await open(h);
  assert.equal(h.element('paper-settle').disabled,false);
  h.click('paper-more'); await h.flush();
  assert.match(h.element('paper-result-title').textContent,/pending settlement/);
  assert.equal(h.element('paper-settle').disabled,false);
  assert.equal(h.element('paper-go').disabled,true);
});

test('an actual objective or ambiguous-side blocker remains the main explanation beside true witnesses', () => {
  const primary_witness={option_id:'opposite_side_first',path:'option.first',root_path:'option.first',kind:'compare',status:'true',interval_state:'within',evaluated_at_ms:'61000',left_reference:'ask',observed:'25',required:'30',comparison:'at_most',unit:'contract_cents'};
  assert.match(paperReceipt(receipt({attempt_state:null,reason:'objective_blocked',primary_witness})).reason,/objective safeguards/i);
  assert.match(paperReceipt(receipt({attempt_state:null,reason:'ambiguous_side',primary_witness})).reason,/unique purchase side/i);
});

const recovery = overrides => ({ paper_market_id: '2', source_market_id: 'KXBTC15M-PAPER-NEXT', side: 'no', requested_quantity: '6', state: 'dispatched', filled_quantity: '0', confirmed_quantity: '4', interrupted: 'no_outcome', ...overrides });

test('an uncertain purchase shows the exact paused wording, keeps confirmed holdings and blocks Resume', async t => {
  const checking = recovery();
  const h = await harness(t, { paper(args) { return args.operation === 'snapshot' ? snapshot(run({ reserved: '157880000', recovery: checking }), { effective_paused: true }) : history([receipt({ attempt_state: 'dispatched', reason: 'checking_purchase', attempt_count: '2', filled_quantity: '4', cost: '100000000', fee: '5250000', debit: '105250000', recovery: checking })]); } });
  await open(h);
  assert.equal(h.element('paper-status').textContent, 'Trading paused — checking a purchase');
  assert.equal(h.element('paper-recovery').textContent, 'Checking a purchase of 6 NO in KXBTC15M-PAPER-NEXT. Its reply was not recorded. No other purchase is sent, and the funds reserved for this purchase stay held until its outcome is confirmed. Confirmed in this market: 4.');
  assert.equal(h.element('paper-resume').disabled, true);
  assert.equal(h.element('paper-result-title').textContent, 'Purchase outcome unconfirmed');
  assert.match(h.element('paper-reason').textContent, /outcome is being checked/);
});

test('a recovered attempt reports its actual result in Now and in the market history', async t => {
  const done = recovery({ state: 'partial', filled_quantity: '3', confirmed_quantity: '7' });
  const h = await harness(t, { paper(args) { return args.operation === 'snapshot' ? snapshot(run({ control_state: 'stopped', recovery: done })) : history([receipt({ attempt_state: 'partial', reason: 'confirmed_partial_fill_pending_settlement', attempt_count: '2', filled_quantity: '7', cost: '175000000', fee: '9190000', debit: '184190000', recovery: done })]); } });
  await open(h);
  assert.equal(h.element('paper-status').textContent, 'Stopped · purchases remain stopped');
  const result = 'Purchase check complete: 3 of 6 NO in KXBTC15M-PAPER-NEXT filled. Its reply was not recorded. Confirmed in this market: 7.';
  assert.equal(h.element('paper-recovery').textContent, result);
  const facts = Object.fromEntries(paperReceipt(receipt({ attempt_state: 'partial', filled_quantity: '7', recovery: done })).facts);
  assert.equal(facts['Purchase check'], result);
  assert.equal(facts['Confirmed quantity'], '7');
  assert.equal(h.element('paper-settle').disabled, false);
});

test('a purchase still unknown after restart stays paused and offers only the explicit check', async t => {
  const h = await harness(t, { paper(args) { return args.operation === 'snapshot' ? snapshot(run({ control_state: 'unresolved', pending_holdings: false, recovery: recovery({ requested_quantity: '10', confirmed_quantity: '0', interrupted: 'restart' }) })) : history([]); } });
  await open(h);
  assert.equal(h.element('paper-status').textContent, 'Trading paused — checking a purchase');
  assert.match(h.element('paper-recovery').textContent, /Metten restarted before its outcome was recorded/);
  assert.equal(h.element('paper-settle').disabled, false);
  assert.equal(h.element('paper-go').disabled, true);
  assert.equal(h.element('paper-resume').disabled, true);
  const canceled = paperReceipt(receipt({ attempt_state: 'canceled', filled_quantity: '0', recovery: recovery({ state: 'canceled', requested_quantity: '10', confirmed_quantity: '0', interrupted: 'restart' }) }));
  assert.match(Object.fromEntries(canceled.facts)['Purchase check'], /was never sent and was canceled/);
});

test('Check settlement is enabled as soon as a current snapshot is accepted, before history returns', async t => {
  const h = await harness(t, { paper(args) {
    if (args.operation === 'snapshot') return snapshot(run({ control_state: 'stopped', pending_holdings: true }));
    if (args.operation === 'history') return new Promise(() => {});
    throw new Error('Unexpected operation');
  } });
  await open(h);
  assert.equal(h.element('paper-status').textContent, 'Stopped · purchases remain stopped');
  assert.equal(h.element('paper-settle').disabled, false);
});

test('a history read failure keeps the current snapshot usable and its settlement check enabled', async t => {
  let historyReads = 0;
  const h = await harness(t, { paper(args) {
    if (args.operation === 'snapshot') return snapshot(run({ run_id: '3', control_state: 'stopped', pending_holdings: true }));
    if (args.operation === 'history') { historyReads++; throw { code: 'storage', detail: 'private-marker' }; }
    if (args.operation === 'check_settlement') return new Promise(() => {});
    throw new Error('Unexpected operation');
  } });
  await open(h);
  assert.equal(h.element('paper-status').textContent, 'Stopped · purchases remain stopped');
  assert.match(h.element('paper-journal-error').textContent, /history could not be read/i);
  assert.doesNotMatch(h.element('paper-journal-error').textContent, /private-marker/);
  assert.equal(h.element('paper-settle').disabled, false);
  h.timers.at(-1).callback(); await h.flush();
  assert.equal(historyReads, 2, 'the same history key is retried on the next summary');
  h.click('paper-settle'); await h.flush();
  assert.equal(JSON.stringify(h.calls.filter(c => c.args?.operation === 'check_settlement').map(c => c.args)), JSON.stringify([{ operation: 'check_settlement', run_id: '3' }]));
});

test('a history read failure leaves the recording error visible and reports itself beside the journal', async t => {
  let failing = true;
  const h = await harness(t, { paper(args) {
    if (args.operation === 'snapshot') return snapshot(run({ error_code: 'storage' }));
    if (args.operation === 'history') { if (failing) throw { code: 'storage', detail: 'private-marker' }; return history([receipt()]); }
    throw new Error('Unexpected operation');
  } });
  await open(h);
  assert.match(h.element('paper-error').textContent, /Recording could not be confirmed. Purchases are fenced/);
  assert.match(h.element('paper-journal-error').textContent, /history could not be read/i);
  failing = false; h.timers.at(-1).callback(); await h.flush();
  assert.equal(h.element('paper-journal-error').textContent, '');
  assert.match(h.element('paper-error').textContent, /Recording could not be confirmed/);
});

test('repeated refreshes write each live region once per distinct message', async t => {
  let failing = true, recording = 'storage';
  const h = await harness(t, { paper(args) {
    if (args.operation === 'snapshot') return snapshot(run({ error_code: recording }));
    if (args.operation === 'history') { if (failing) throw { code: 'storage' }; return history([receipt()]); }
    throw new Error('Unexpected operation');
  } });
  const writes = {};
  for (const id of ['paper-error', 'paper-journal-error']) {
    const region = h.element(id); let text = region.textContent; writes[id] = [];
    Object.defineProperty(region, 'textContent', { get: () => text, set: value => { writes[id].push(value); text = value; } });
  }
  await open(h);
  for (let i = 0; i < 3; i++) { h.timers.at(-1).callback(); await h.flush(); }
  failing = false; recording = null;
  for (let i = 0; i < 2; i++) { h.timers.at(-1).callback(); await h.flush(); }
  assert.equal(writes['paper-error'].length, 2); assert.match(writes['paper-error'][0], /Recording could not be confirmed/); assert.equal(writes['paper-error'][1], '');
  assert.equal(writes['paper-journal-error'].length, 2); assert.match(writes['paper-journal-error'][0], /history could not be read/i); assert.equal(writes['paper-journal-error'][1], '');
});

test('each failed More markets click is announced again, with a clear between them', async t => {
  const h = await harness(t, { paper(args) {
    if (args.operation === 'snapshot') return snapshot(run());
    if (args.operation === 'history') { if (args.before_paper_market_id) throw { code: 'storage' }; return { kind: 'history', page: { markets: [receipt()], next_cursor: '2' } }; }
    throw new Error('Unexpected operation');
  } });
  await open(h);
  const region = h.element('paper-journal-error'); let text = region.textContent; const writes = [];
  Object.defineProperty(region, 'textContent', { get: () => text, set: value => { writes.push(value); text = value; } });
  for (let i = 0; i < 2; i++) { h.click('paper-more'); await h.flush(); }
  assert.equal(writes.length, 3);
  assert.match(writes[0], /More markets could not be read/); assert.equal(writes[1], ''); assert.equal(writes[2], writes[0]);
});

test('a failed More markets read keeps the previous page and reports itself beside the journal', async t => {
  const h = await harness(t, { paper(args) {
    if (args.operation === 'snapshot') return snapshot(run({ control_state: 'stopped', pending_holdings: true }));
    if (args.operation === 'history') { if (args.before_paper_market_id) throw { code: 'storage', detail: 'private-marker' }; return { kind: 'history', page: { markets: [receipt()], next_cursor: '2' } }; }
    throw new Error('Unexpected operation');
  } });
  await open(h);
  h.click('paper-more'); await h.flush();
  assert.match(h.element('paper-history').children[0].children[0].textContent, /KXBTC15M-PAPER-NEXT/);
  assert.match(h.element('paper-journal-error').textContent, /More markets could not be read/);
  assert.doesNotMatch(h.element('paper-journal-error').textContent, /private-marker/);
  assert.equal(h.element('paper-error').textContent, '');
  assert.equal(h.element('paper-status').textContent, 'Stopped · purchases remain stopped');
  assert.equal(h.element('paper-settle').disabled, false);
  assert.equal(h.element('paper-more').disabled, false);
});


test('Now distinguishes daily settlement, holdings, reservations and the entry stop', async t => {
  const daily_risk = { mode: 'paper', utc_day: '20776', settled_net: '-500000000',
    pending_holdings: '263130000', pending_attempts: '200000000', maximum_loss: '500000000',
    profit_target: null, resume_next_day: false, stop_reason: 'daily_loss' };
  const h = await harness(t, { paper(args) {
    if (args.operation === 'snapshot') return snapshot(run({ daily_risk, current_risk_block: 'daily_budget' }));
    return history([]);
  } });
  await open(h);
  const status = h.element('paper-risk').textContent;
  assert.match(status, /settled −\$5\.00/);
  assert.match(status, /pending holdings \$2\.6313/);
  assert.match(status, /reserved attempts \$2\.00/);
  assert.match(status, /finishing the current market/);
  assert.match(status, /Final losses can exceed the threshold/);
  assert.match(status, /whole purchase would exceed the remaining daily loss allowance/);

  const blocked = paperReceipt(receipt({ participated: false, risk_block: 'daily_entry_block',
    attempt_state: null, reason: 'prepared' }));
  assert.match(blocked.reason, /threshold blocked entry/);
  const settled = paperReceipt(receipt({ risk_block: 'daily_budget', reason: 'settled',
    settled_net: '736870000', payout: '1000000000' }));
  assert.match(Object.fromEntries(settled.facts)['Risk decision'], /remaining daily loss allowance/);
  assert.equal(Object.fromEntries(settled.facts)['Settled net'], '$7.3687');
});
