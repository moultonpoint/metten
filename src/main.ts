import { invoke } from '@tauri-apps/api/core';
import type { Action, Snapshot, StrategyRequest, StrategyResponse, StrategyDetail, SourceName, SourceRequest, SourceResponse, SourceSnapshot, ProviderCredentialResponse } from './contract';
import { presentation, strategyFailure, sourceFailure, sourcePresentation, paperMoney, paperFailure, paperReason, paperReceipt, paperChecking, paperRecovery } from './view';
import './style.css';
import learningExample from '../examples/learning.json?raw';
import type { PaperRequest, PaperResponse, PaperSnapshot, PaperReceipt } from './contract';

function element<T extends HTMLElement>(id: string): T {
  const value = document.getElementById(id);
  if (!value) throw new Error(`Missing interface element: ${id}`);
  return value as T;
}
const start = element<HTMLButtonElement>('start');
const pause = element<HTMLButtonElement>('pause');
const advance = element<HTMLButtonElement>('advance');
const error = element('error');
let requestError = '';
let refreshSequence = 0;

function render(snapshot: Snapshot) {
  const view = presentation(snapshot);
  element('status').textContent = view.status;
  element('recording').textContent = view.recording;
  element('clock').textContent = `Saved test clock: ${snapshot.history[0].tick} seconds`;
  start.disabled = view.startDisabled;
  pause.disabled = view.pauseDisabled;
  advance.disabled = view.advanceDisabled;
  error.textContent = snapshot.recording_error ?? requestError;
  const items = snapshot.history.map(record => {
    const item = document.createElement('li');
    item.textContent = `${record.reason} · ${record.status} · clock ${record.tick}s`;
    const time = document.createElement('time');
    time.dateTime = record.recorded_at;
    time.textContent = record.recorded_at;
    item.append(time);
    return item;
  });
  element('history').replaceChildren(...items);
  const settings = snapshot.writer_settings;
  element('settings').textContent = `SQLite ${settings.sqlite_version} · journal ${settings.journal_mode} · synchronous ${settings.synchronous} (FULL) · fullfsync ${settings.fullfsync} · checkpoint_fullfsync ${settings.checkpoint_fullfsync}. Verified on the writer connection.`;
}

async function refresh() {
  const sequence = ++refreshSequence;
  try {
    const snapshot = await invoke<Snapshot>('snapshot');
    if (sequence === refreshSequence) render(snapshot);
  } catch (failure) {
    if (sequence !== refreshSequence) return;
    start.disabled = advance.disabled = true;
    pause.disabled = false;
    element('status').textContent = 'Session status unavailable';
    element('recording').textContent = 'Cannot confirm session state. Try Pause now, or quit to stop the session.';
    error.textContent = String(failure);
  }
}
async function control(action: Action) {
  start.disabled = advance.disabled = true;
  requestError = '';
  try { await invoke('control', { action }); }
  catch (failure) { requestError = String(failure); }
  await refresh();
}
start.addEventListener('click', () => void control('start'));
pause.addEventListener('click', () => void control('pause'));
advance.addEventListener('click', () => void control('advance'));
async function poll() { await refresh(); window.setTimeout(() => void poll(), 200); }
void poll();

for (const operation of ['save', 'replace', 'remove', 'check']) {
  element(`credential-${operation}`).addEventListener('click', () => {
    const input = element<HTMLInputElement>('credential-value');
    const value = input.value;
    input.value = '';
    const buttons = ['save', 'replace', 'remove', 'check'].map(o => element<HTMLButtonElement>(`credential-${o}`));
    buttons.forEach(b => { b.disabled = true; });
    element('credential-status').textContent = 'Checking local credential action…';
    void invoke<string>('credential', { operation, ...(['save', 'replace'].includes(operation) ? { value } : {}) })
      .then(status => { element('credential-status').textContent = status; })
      .catch(failure => {
        // Only native finite messages are shown; unexpected framework errors stay private.
        element('credential-status').textContent = typeof failure === 'string' && /^(Unavailable|Locked|Verification unavailable|Already saved|Missing|Canceled|Authentication failed|Denied|Invalid input|Invalid request|Invalid credential|Quitting|Credential access unavailable|Mismatch) —?/.test(failure)
          ? failure : 'Credential action failed — reopen the isolated verification setup and retry.';
      })
      .finally(() => { buttons.forEach(b => { b.disabled = false; }); });
  });
}

const editor = element<HTMLTextAreaElement>('strategy-json');
const previewButton = element<HTMLButtonElement>('strategy-preview');
const saveButton = element<HTMLButtonElement>('strategy-save');
const moreButton = element<HTMLButtonElement>('strategy-more');
const strategyStatus = element('strategy-status');
let editorGeneration = 0;
let reviewedText: string | null = null;
let previewPending = false;
let savePending = false;
let listGeneration = 0;
let detailGeneration = 0;
let nextCursor: string | null = null;
let inspected: StrategyDetail | null = null;
function strategyControls() {
  previewButton.disabled = previewPending || savePending;
  saveButton.disabled = previewPending || savePending || reviewedText === null || reviewedText !== editor.value;
}
function changedText() {
  editorGeneration++;
  reviewedText = null;
  element('strategy-explanation').textContent = '';
  strategyStatus.textContent = 'Text changed. Preview before saving.';
  strategyControls();
}
editor.addEventListener('input', changedText);
element('strategy-file').addEventListener('change', () => {
  changedText();
  const generation = editorGeneration;
  const file = element<HTMLInputElement>('strategy-file').files?.[0];
  if (!file) return;
  if (file.size > 256 * 1024) { strategyStatus.textContent = 'Strategy exceeds 256 KiB.'; return; }
  void file.text().then(text => {
    if (generation !== editorGeneration) return;
    if (new TextEncoder().encode(text).length > 256 * 1024) { strategyStatus.textContent = 'Strategy exceeds 256 KiB.'; return; }
    editor.value = text; changedText();
  }).catch(() => { if (generation === editorGeneration) strategyStatus.textContent = 'Could not read file. Choose a local JSON file again.'; });
});
function strategyRequest(request: StrategyRequest) { return invoke<StrategyResponse>('strategies', request); }
function ruleErrors(result: Extract<StrategyResponse, { kind: 'rule_errors' }>) {
  return result.errors.map(error => `${error.path}: ${error.message} (${error.code})\nAccepted: ${error.accepted_form}`).join('\n\n') + (result.truncated ? '\nAdditional errors omitted.' : '');
}
previewButton.addEventListener('click', () => {
  if (previewPending || savePending) return;
  const generation = editorGeneration;
  const json = editor.value;
  if (new TextEncoder().encode(json).length > 256 * 1024) { strategyStatus.textContent = 'Strategy exceeds 256 KiB.'; strategyControls(); return; }
  previewPending = true; strategyControls();
  strategyStatus.textContent = 'Checking the current text…';
  void strategyRequest({ operation: 'preview', json }).then(result => {
    if (generation !== editorGeneration || json !== editor.value) return;
    if (result.kind === 'rule_errors') { reviewedText = null; element('strategy-explanation').textContent = ''; strategyStatus.textContent = ruleErrors(result); return; }
    if (result.kind !== 'preview') throw new Error('Unexpected response');
    reviewedText = json;
    element('strategy-explanation').textContent = result.explanation;
    strategyStatus.textContent = `${result.notice}\n${result.meaning_id} · ${result.requirements.support_profile}\nPlanned memory: ${result.requirements.planned_bytes} bytes; shared underlying: ${result.requirements.shared_underlying_bytes}; per market: ${result.requirements.per_market_bytes}.\nRequired history: ${Object.entries(result.requirements.startup_ms).map(([id, ms]) => `${id}: ${ms / 1000}s`).join(', ') || 'none'}.`;
  }).catch(failure => { if (generation === editorGeneration) strategyStatus.textContent = strategyFailure(failure); })
    .finally(() => { previewPending = false; strategyControls(); });
});
saveButton.addEventListener('click', () => {
  if (savePending || reviewedText === null || reviewedText !== editor.value) return;
  const json = editor.value;
  const reviewed_json = reviewedText;
  savePending = true; strategyControls();
  strategyStatus.textContent = 'Saving reviewed text…';
  void strategyRequest({ operation: 'save', json, reviewed_json }).then(result => {
    if (result.kind === 'rule_errors') { reviewedText = null; element('strategy-explanation').textContent = ''; strategyStatus.textContent = ruleErrors(result); return; }
    if (result.kind !== 'saved') throw new Error('Unexpected response');
    reviewedText = null;
    strategyStatus.textContent = `Saved immutable version ${result.version.id} · ${result.version.recorded_at}.`;
    void loadStrategies();
  }).catch(failure => { strategyStatus.textContent = strategyFailure(failure, true); })
    .finally(() => { savePending = false; strategyControls(); });
});
async function loadStrategies(before_id?: string) {
  const generation = ++listGeneration;
  element('strategy-list-status').textContent = 'Reading saved versions…';
  moreButton.disabled = true;
  try {
    const result = await strategyRequest({ operation: 'list', ...(before_id ? { before_id } : {}) });
    if (generation !== listGeneration) return;
    if (result.kind !== 'versions') throw new Error('Unexpected response');
    const items = result.versions.map(version => {
      const item = document.createElement('li');
      const button = document.createElement('button');
      button.textContent = `Inspect version ${version.id} · ${version.display_name ?? 'Unknown name'} · ${version.status} · ${version.recorded_at}`;
      button.addEventListener('click', () => void inspectStrategy(version.id));
      item.append(button); return item;
    });
    if (before_id) element('strategy-versions').append(...items);
    else element('strategy-versions').replaceChildren(...items);
    const priorPaperVersion = paperVersion.value;
    const choices = result.versions.filter(v => v.status === 'supported').map(version => { const option = document.createElement('option'); option.value = version.id; option.textContent = `Version ${version.id} · ${version.display_name ?? 'Saved strategy'}`; return option; });
    if (before_id) paperVersion.append(...choices); else { const empty = document.createElement('option'); empty.value = ''; empty.textContent = 'Choose a saved version'; paperVersion.replaceChildren(empty, ...choices); }
    if (priorPaperVersion) paperVersion.value = priorPaperVersion;
    paperControls();
    nextCursor = result.next_cursor;
    element('strategy-list-status').textContent = result.versions.length ? 'Saved versions. Select one to inspect.' : 'No saved versions.';
  } catch (failure) {
    if (generation === listGeneration) element('strategy-list-status').textContent = `Version refresh failed. ${strategyFailure(failure)} Any confirmed save above remains saved.`;
  } finally { if (generation === listGeneration) moreButton.disabled = nextCursor === null; }
}
async function inspectStrategy(version_id: string) {
  const generation = ++detailGeneration;
  inspected = null;
  element<HTMLButtonElement>('strategy-copy').disabled = true;
  element('strategy-detail').textContent = 'Reading version…';
  try {
    const result = await strategyRequest({ operation: 'detail', version_id });
    if (generation !== detailGeneration) return;
    if (result.kind !== 'detail') throw new Error('Unexpected response');
    inspected = result.version;
    element('strategy-detail').textContent = `Version ${inspected.summary.id} · ${inspected.summary.status}\n${inspected.notice}\n${inspected.meaning_id ?? 'Historical meaning not recorded'}\n\n${inspected.explanation ?? 'No stored explanation.'}\n\n${inspected.json ?? 'Full content unavailable for display.'}`;
    element<HTMLButtonElement>('strategy-copy').disabled = inspected.json === null;
  } catch (failure) { if (generation === detailGeneration) element('strategy-detail').textContent = strategyFailure(failure); }
}
element('strategy-copy').addEventListener('click', () => {
  if (inspected?.json === null || !inspected) return;
  editor.value = inspected.json; changedText(); editor.focus();
});
element('strategy-reload').addEventListener('click', () => void loadStrategies());
moreButton.addEventListener('click', () => { if (nextCursor) void loadStrategies(nextCursor); });
void loadStrategies();

const sourceNames: SourceName[] = ['kalshi', 'brti', 'coinbase', 'kraken'];
const sourcePanel = element<HTMLDetailsElement>('sources-panel');
let sourceGeneration = 0;
let sourceInFlight = false;
let sourceWanted: SourceRequest | null = null;
let sourceTimer: number | undefined;
let sourceOwner = '';
let sourceRevision = '0';
let sourceActionPending = false;
let sourceActionSequence = 0;
function sourcesActive() { return sourcePanel.open && !document.hidden; }
function renderSources(snapshot: SourceSnapshot) {
  if (snapshot.owner === sourceOwner && BigInt(snapshot.revision) < BigInt(sourceRevision)) return;
  sourceOwner = snapshot.owner; sourceRevision = snapshot.revision;
  element('source-environment').textContent = `${snapshot.environment === 'production' ? 'Production market data' : 'Demo'} · Read-only`;
  element('provider-environment').textContent = `${snapshot.environment === 'production' ? 'Production' : 'Demo'} API key. Shared by Kalshi and BRTI. Saving a key does not connect either source.`;
  element('source-markets').textContent = `Current market: ${snapshot.current_market ?? 'not verified'}\n${snapshot.markets.map(m => `${m.market_id} · ${m.preparation}${m.verified ? '' : ' · metadata unverified'}`).join('\n') || 'Next market not ready — checking its information.'}`;
  for (const name of sourceNames) element(`source-${name}-status`).textContent = sourcePresentation(snapshot, name);
}
function scheduleSourceRead() {
  if (sourceTimer !== undefined) window.clearTimeout?.(sourceTimer);
  if (sourcesActive()) sourceTimer = window.setTimeout(() => { sourceTimer = undefined; queueSourceRead({ operation: 'snapshot' }); }, 1000);
}
function queueSourceRead(request: SourceRequest) {
  if (sourceTimer !== undefined) { window.clearTimeout?.(sourceTimer); sourceTimer = undefined; }
  sourceWanted = request;
  sourceGeneration++;
  void drainSourceRead();
}
async function drainSourceRead() {
  if (sourceInFlight || !sourcesActive() || sourceActionPending || !sourceWanted) return;
  sourceInFlight = true;
  const request = sourceWanted; sourceWanted = null;
  const generation = sourceGeneration;
  try {
    const result = await invoke<SourceResponse>('sources', request);
    if (generation !== sourceGeneration || !sourcesActive()) return;
    if (result.kind === 'snapshot') renderSources(result);
    else if (result.kind === 'details') element('source-details').textContent = `${result.source}\n${JSON.stringify({ values: result.values, bindings: result.bindings }, null, 2)}\n${result.notice}${result.counters ? '\nLocal provider counters: ' + JSON.stringify(result.counters) : ''}`;
    else if (result.kind === 'requirements') element('source-requirements').textContent = `Version ${result.version_id}\n${result.requirements.map(r => `${r.input_id}: ${r.source}.${r.instrument}.${r.field} · ${r.unit} · ${r.side ?? 'underlying'} · startup ${r.startup_ms} ms${r.implicit ? ' · implicit' : ''}`).join('\n')}`;
  } catch (failure) {
    if (generation === sourceGeneration && sourcesActive()) element('source-status').textContent = sourceFailure(failure);
  } finally {
    sourceInFlight = false;
    if (sourceWanted) void drainSourceRead(); else scheduleSourceRead();
  }
}
sourcePanel.addEventListener('toggle', () => {
  sourceGeneration++;
  if (sourcesActive()) queueSourceRead({ operation: 'snapshot' });
  else { sourceWanted = null; if (sourceTimer !== undefined) window.clearTimeout?.(sourceTimer); }
});
document.addEventListener?.('visibilitychange', () => {
  sourceGeneration++;
  ++paperVisibilityGeneration;
  if (!document.hidden) requestPaperRead('summary'); else if (paperTimer !== undefined) window.clearTimeout(paperTimer);
  if (sourcesActive()) queueSourceRead({ operation: 'snapshot' });
  else if (sourceTimer !== undefined) window.clearTimeout?.(sourceTimer);
});
for (const name of sourceNames) {
  for (const operation of ['connect', 'disconnect'] as const) element(`source-${name}-${operation}`).addEventListener('click', () => {
    sourceGeneration++;
    if (sourceActionPending && operation === 'connect') { element('source-status').textContent = 'A source action is in progress. Wait and retry.'; return; }
    const actionSequence = ++sourceActionSequence;
    const ownsPending = !sourceActionPending;
    sourceActionPending = true;
    element('source-status').textContent = `${operation === 'connect' ? 'Connecting' : 'Disconnecting'} ${name}…`;
    void invoke<SourceResponse>('sources', { operation, source: name }).then(() => {
      if (actionSequence !== sourceActionSequence) return;
      element('source-status').textContent = operation === 'connect' ? `Connection requested for ${name}. Waiting for verified source data.` : `${name} disconnected.`;
    }).catch(failure => { if (actionSequence === sourceActionSequence) element('source-status').textContent = sourceFailure(failure); }).finally(() => {
      if (ownsPending) sourceActionPending = false;
      queueSourceRead({ operation: 'snapshot' });
    });
  });
  element(`source-${name}-details`).addEventListener('click', () => queueSourceRead({ operation: 'details', source: name }));
}
element('source-inspect').addEventListener('click', () => queueSourceRead({ operation: 'inspect', version_id: element<HTMLInputElement>('source-version').value }));
let providerPending = false;
for (const operation of ['status', 'save', 'replace', 'remove'] as const) element(`provider-${operation}`).addEventListener('click', () => {
  const secret = element<HTMLTextAreaElement>('provider-private-key');
  const private_key = secret.value; secret.value = '';
  if (providerPending) return;
  providerPending = true; sourceGeneration++;
  const buttons = ['status', 'save', 'replace', 'remove'].map(name => element<HTMLButtonElement>(`provider-${name}`));
  buttons.forEach(button => { button.disabled = true; });
  const status = element('provider-status-text'); status.textContent = 'Checking credential action…';
  const material = operation === 'save' || operation === 'replace' ? { key_id: element<HTMLInputElement>('provider-key-id').value, private_key } : {};
  void invoke<ProviderCredentialResponse>('provider_credential', { operation, provider: 'kalshi', ...material }).then(result => {
    status.textContent = result.kind === 'saved' ? `Saved: ${result.key_id} · ${result.key_type}` : result.kind === 'missing' ? 'Missing credential.' : `Credential ${result.operation} confirmed. Connect explicitly when ready.`;
  }).catch(failure => { status.textContent = sourceFailure(failure); }).finally(() => {
    providerPending = false; buttons.forEach(button => { button.disabled = false; }); queueSourceRead({ operation: 'snapshot' });
  });
});
if (sourcesActive()) queueSourceRead({ operation: 'snapshot' });

const paperPanel = element<HTMLDetailsElement>('paper-panel');
const paperVersion = element<HTMLSelectElement>('paper-version');
const paperSettings = element<HTMLFieldSetElement>('paper-settings');
let paperSnapshot: PaperSnapshot | null = null;
let paperHistory: PaperReceipt[] = [];
let paperCursor: string | null = null;
let paperSelection: { run_id: string; paper_market_id: string } | null = null;
let paperReadBusy = false;
let paperWanted: 'summary' | 'detail' | 'more' | 'evidence_more' | null = null;
let paperActionPending = false;
let paperActionGeneration = 0;
let paperVisibilityGeneration = 0;
let paperSelectionGeneration = 0;
let paperResponseEpoch = 0;
let paperEvidenceCursor: string | null = null;
let paperDetailBase = '';
let paperJournalKey = '';
let paperJournalNodes = new Map<string, { item: HTMLLIElement; button: HTMLButtonElement; receipt: PaperReceipt }>();
let paperUnavailable = true;
let paperHistoryKey = '';
let paperRequestError = '';
let paperTimer: ReturnType<typeof window.setTimeout> | undefined;
let paperStartRequest: Extract<PaperRequest, { operation: 'start' }> | null = null;
function paperControls() {
  const run = paperSnapshot?.run;
  const active = !!run && run.control_state !== 'stopped';
  const holding = run?.pending_holdings ?? false;
  const checking = !!run && paperChecking(run.recovery);
  paperSettings.disabled = active || holding || paperStartRequest !== null || paperActionPending || (paperUnavailable && !!run);
  element<HTMLButtonElement>('paper-go').disabled = paperUnavailable || active || holding || paperActionPending || (!paperStartRequest && !paperVersion.value);
  element<HTMLButtonElement>('paper-pause').disabled = !run;
  element<HTMLButtonElement>('paper-resume').disabled = paperUnavailable || !active || !paperSnapshot?.effective_paused || run?.control_state === 'unresolved' || checking || paperActionPending;
  element<HTMLButtonElement>('paper-stop').disabled = paperUnavailable || !active || paperActionPending;
  // A stopped run with an unconfirmed purchase retries its check here before any settlement.
  element<HTMLButtonElement>('paper-settle').disabled = paperUnavailable || !run || !(run.control_state === 'unresolved' || (!active && holding)) || paperActionPending;
  element<HTMLButtonElement>('paper-more').disabled = paperReadBusy || paperCursor === null;
  if (run && paperSettings.disabled && !paperStartRequest) {
    paperVersion.value = run.strategy_version_id;
    if (paperVersion.value !== run.strategy_version_id) { const option = document.createElement('option'); option.value = run.strategy_version_id; option.textContent = `Version ${run.strategy_version_id} · current run`; paperVersion.append(option); paperVersion.value = run.strategy_version_id; }
    element<HTMLInputElement>('paper-quantity').value = run.quantity;
    element<HTMLInputElement>('paper-minimum').value = run.minimum_price;
    element<HTMLInputElement>('paper-maximum').value = run.maximum_price;
    element<HTMLInputElement>('paper-balance').value = paperMoney(run.starting_balance).slice(1);
    const daily = run.daily_risk;
    if (daily?.maximum_loss) element<HTMLInputElement>('paper-loss-limit').value = paperMoney(daily.maximum_loss).slice(1);
    element<HTMLInputElement>('paper-profit-target').value = daily?.profit_target ? paperMoney(daily.profit_target).slice(1) : '';
    element<HTMLInputElement>('paper-next-day').checked = daily?.resume_next_day ?? false;
  }
  element<HTMLButtonElement>('paper-evidence-more').disabled = paperReadBusy || paperEvidenceCursor === null;
  element('paper-go').textContent = paperStartRequest ? 'Retry the same Go request' : 'Go';
}
// Rewriting identical text would make a screen reader announce it again.
function announce(id: string, text: string) { const region = element(id); if (region.textContent !== text) region.textContent = text; }
function renderPaper(snapshot: PaperSnapshot): boolean {
  const prior = paperSnapshot;
  if (prior?.run && !snapshot.run) return false;
  if (prior?.run && snapshot.run && (BigInt(snapshot.run.run_id) < BigInt(prior.run.run_id) || snapshot.run.run_id === prior.run.run_id && BigInt(snapshot.run.revision) < BigInt(prior.run.revision))) return false;
  if (prior && BigInt(snapshot.effective_generation) < BigInt(prior.effective_generation)) return false;
  paperSnapshot = snapshot; paperUnavailable = false;
  const run = snapshot.run;
  if (run && run.run_id !== prior?.run?.run_id) element<HTMLDetailsElement>('paper-settings-details').open = false;
  if (run && paperStartRequest?.start_uuid === run.start_uuid) paperStartRequest = null;
  const states: Record<string, string> = { collecting: 'Collecting required evidence', waiting: 'Waiting for the next full market boundary', active: 'Monitoring the current paper market', paused: 'Purchases paused · monitoring continues', stopped: 'Stopped · purchases remain stopped', unresolved: 'Stopped · an operation remains unconfirmed' };
  element('paper-status').textContent = run && paperChecking(run.recovery) ? 'Trading paused — checking a purchase' : run ? (snapshot.effective_paused && run.control_state !== 'stopped' && run.control_state !== 'unresolved' ? 'Purchases paused · monitoring continues' : states[run.control_state]) : 'Ready for a controlled paper example';
  element('paper-fixed').textContent = run ? `Run ${run.run_id} · Version ${run.strategy_version_id} · ${run.quantity} contracts per market · ${run.minimum_price}–${run.maximum_price}¢ · starting balance ${paperMoney(run.starting_balance)} · cash ${paperMoney(run.cash)} · reserved ${paperMoney(run.reserved)}\nSynthetic clock ${run.virtual_frontier_ms} ms · ${snapshot.stop_after_market ? 'Stop after market requested · ' : ''}${run.pending_ordinal === null ? 'Displayed accounting is committed.' : 'An input remains unconfirmed; inspect details.'}` : 'Choose a saved version and fixed settings. Go waits for an eligible full boundary.';
  element('paper-recovery').textContent = run?.recovery ? paperRecovery(run.recovery) : '';
  const daily = run?.daily_risk;
  let risk = run ? 'Daily limits were not configured for this run.' : '';
  if (daily) {
    const day = new Date(Number(daily.utc_day) * 86_400_000).toISOString().slice(0, 10);
    const target = daily.profit_target ? ` · profit target ${paperMoney(daily.profit_target)}` : '';
    const loss = daily.maximum_loss ? ` · loss limit ${paperMoney(daily.maximum_loss)}` : '';
    risk = `UTC ${day} · settled ${paperMoney(daily.settled_net)} · pending holdings ${paperMoney(daily.pending_holdings)} · reserved attempts ${paperMoney(daily.pending_attempts)}${loss}${target}`;
    if (daily.stop_reason) {
      const threshold = daily.stop_reason === 'daily_loss' ? 'Daily loss limit reached' : 'Daily profit target reached';
      const action = run?.control_state === 'active' ? 'finishing the current market under existing safeguards' : 'new market entry blocked';
      risk += ` · ${threshold}, ${action}`;
    }
    risk += '. Final losses can exceed the threshold.';
  }
  if (run?.current_risk_block) risk += ` ${paperReason(run.current_risk_block)}`;
  announce('paper-risk', risk);
  const errorCode = snapshot.playback_error ?? run?.error_code;
  announce('paper-error', errorCode ? paperFailure({ code: errorCode }) : paperRequestError);
  paperControls();
  return true;
}
function showPaperReceipt(receipt: PaperReceipt) {
  const view = paperReceipt(receipt);
  element('paper-result-title').textContent = view.title;
  element('paper-reason').textContent = view.reason;
  const facts = view.facts.flatMap(([name, value]) => {
    const label = document.createElement('dt'); label.textContent = name;
    const content = document.createElement('dd'); content.textContent = value;
    return [label, content];
  });
  element('paper-receipt').replaceChildren(...facts);
}
function renderPaperHistory() {
  const focused = document.activeElement?.getAttribute('data-paper-market');
  const nodes = new Map<string, { item: HTMLLIElement; button: HTMLButtonElement; receipt: PaperReceipt }>();
  for (const receipt of paperHistory) {
    const key = `${receipt.run_id}:${receipt.paper_market_id}`;
    let node = paperJournalNodes.get(key);
    if (!node) {
      node = { item: document.createElement('li'), button: document.createElement('button'), receipt };
      node.button.setAttribute('data-paper-market', key);
      const entry = node;
      node.button.addEventListener('click', () => {
        paperSelection = { run_id: entry.receipt.run_id, paper_market_id: entry.receipt.paper_market_id }; ++paperSelectionGeneration;
        paperEvidenceCursor = null; paperDetailBase = ''; element('paper-detail').textContent = 'Reading the selected market…'; showPaperReceipt(entry.receipt); renderPaperHistory(); requestPaperRead('detail');
      });
      node.item.append(node.button);
    }
    node.receipt = receipt;
    const view = paperReceipt(receipt);
    node.button.textContent = `Run ${receipt.run_id} · ${receipt.source_market_id}\n${view.title}${receipt.settled_net === null ? '' : '\nNet ' + paperMoney(receipt.settled_net)}`;
    node.button.ariaPressed = String(paperSelection?.run_id === receipt.run_id && paperSelection.paper_market_id === receipt.paper_market_id);
    nodes.set(key, node);
  }
  const key = [...nodes.keys()].join(',');
  if (key !== paperJournalKey || !nodes.size && paperJournalNodes.size) {
    element('paper-history').replaceChildren(...[...nodes.values()].map(n => n.item));
    if (focused) nodes.get(focused)?.button.focus({ preventScroll: true });
  }
  if (!nodes.size && paperJournalKey !== 'empty') {
    const empty = document.createElement('li'); empty.textContent = 'No participating markets yet. Prepared markets are not counted.'; element('paper-history').replaceChildren(empty);
  }
  paperJournalKey = nodes.size ? key : 'empty'; paperJournalNodes = nodes;
}
function requestPaperRead(kind: 'summary' | 'detail' | 'more' | 'evidence_more') {
  if (!paperPanel.open || document.hidden) return;
  if (paperReadBusy) { if (kind !== 'summary' || paperWanted === null) paperWanted = kind; return; }
  void readPaper(kind);
}
async function readPaper(kind: 'summary' | 'detail' | 'more' | 'evidence_more') {
  if (paperReadBusy || !paperPanel.open || document.hidden) return;
  paperReadBusy = true;
  const action = paperActionGeneration, visibility = paperVisibilityGeneration, selectedGeneration = paperSelectionGeneration, responseEpoch = paperResponseEpoch;
  const current = () => responseEpoch === paperResponseEpoch && action === paperActionGeneration && visibility === paperVisibilityGeneration && paperPanel.open && !document.hidden;
  try {
    if (kind === 'summary') {
      const response = await invoke<PaperResponse>('paper', { operation: 'snapshot' });
      if (!current()) return;
      if (response.kind !== 'snapshot') throw new Error('Unexpected paper response');
      if (!renderPaper(response)) return;
      const run = response.run;
      const key = run ? `${run.run_id}:${run.history_revision}:${run.control_state}:${run.cash}` : 'empty';
      if (key !== paperHistoryKey) {
        try {
          const history = await invoke<PaperResponse>('paper', { operation: 'history' });
          if (!current()) return;
          if (history.kind !== 'history') throw new Error('Unexpected paper history');
          paperHistory = history.page.markets; paperCursor = history.page.next_cursor; paperHistoryKey = key; announce('paper-journal-error', '');
          const latestCurrent = paperHistory.find(m => m.run_id === run?.run_id);
          if (!paperSelection && latestCurrent) paperSelection = { run_id: latestCurrent.run_id, paper_market_id: latestCurrent.paper_market_id };
          renderPaperHistory();
          const selected = paperHistory.find(r => r.run_id === paperSelection?.run_id && r.paper_market_id === paperSelection.paper_market_id);
          if (selected) {
            showPaperReceipt(selected);
            ++paperSelectionGeneration; paperEvidenceCursor = null;
            element('paper-detail').textContent = 'Receipt updated. Open details to read the current retained evidence.';
            if (element<HTMLDetailsElement>('paper-details').open) paperWanted = 'detail';
          }
        } catch {
          // The accepted status stays usable; the unchanged key retries history on the next summary.
          if (!current()) return;
          announce('paper-journal-error', 'Paper history could not be read. Last confirmed history remains visible and is read again on the next refresh.');
        }
      }
    }
    if (kind === 'more' && paperCursor !== null) {
      // Only an explicit click reads more, so clear first: a repeated failure is announced again.
      announce('paper-journal-error', '');
      const response = await invoke<PaperResponse>('paper', { operation: 'history', before_paper_market_id: paperCursor });
      if (!current()) return;
      if (response.kind !== 'history') throw new Error('Unexpected paper history');
      paperHistory = response.page.markets; paperCursor = response.page.next_cursor; announce('paper-journal-error', ''); renderPaperHistory();
    }
    if (kind === 'detail' && paperSelection) {
      const selection = { ...paperSelection }, generation = paperSelectionGeneration;
      element('paper-detail').textContent = 'Reading confirmed receipt and retained evidence…';
      const response = await invoke<PaperResponse>('paper', { operation: 'market', ...selection });
      if (!current() || generation !== paperSelectionGeneration) return;
      if (response.kind !== 'market') throw new Error('Unexpected paper receipt');
      showPaperReceipt(response.receipt);
      paperDetailBase = JSON.stringify({ run: response.run, receipt: response.receipt, assumptions: response.assumptions }, null, 2);
      element('paper-detail').textContent = paperDetailBase;
      const evidence = await invoke<PaperResponse>('paper', { operation: 'evidence', ...selection });
      if (!current() || generation !== paperSelectionGeneration) return;
      if (evidence.kind !== 'evidence') throw new Error('Unexpected paper evidence');
      paperEvidenceCursor = evidence.page.next_cursor;
      element('paper-detail').textContent = paperDetailBase + (evidence.page.evidence.length ? '\n\nRetained evidence (up to 20 records)\n' + JSON.stringify(evidence.page.evidence, null, 2) : '\n\nNo additional rule witnesses were retained.');
    }
    if (kind === 'evidence_more' && paperSelection && paperEvidenceCursor !== null) {
      const response = await invoke<PaperResponse>('paper', { operation: 'evidence', ...paperSelection, before_evidence_id: paperEvidenceCursor });
      if (!current() || selectedGeneration !== paperSelectionGeneration) return;
      if (response.kind !== 'evidence') throw new Error('Unexpected paper evidence');
      paperEvidenceCursor = response.page.next_cursor;
      element('paper-detail').textContent = paperDetailBase + '\n\nOlder retained evidence (up to 20 records)\n' + JSON.stringify(response.page.evidence, null, 2);
    }
  } catch {
    if (!current() || (kind === 'detail' || kind === 'evidence_more') && selectedGeneration !== paperSelectionGeneration) return;
    if (kind === 'more') { announce('paper-journal-error', 'More markets could not be read. This page remains visible; choose More markets to try again.'); return; }
    if (kind === 'summary') { paperUnavailable = true; element('paper-status').textContent = 'Paper status unavailable · Pause remains available'; }
    announce('paper-error', 'Paper state could not be read. Last confirmed history remains visible. Pause purchases and refresh status.');
  } finally {
    paperReadBusy = false; paperControls();
    const desired = paperWanted; paperWanted = null;
    if (desired && paperPanel.open && !document.hidden) requestPaperRead(desired); else schedulePaperRead();
  }
}
function schedulePaperRead() {
  if (paperTimer !== undefined) window.clearTimeout(paperTimer);
  if (paperPanel.open && !document.hidden) paperTimer = window.setTimeout(() => requestPaperRead('summary'), 200);
}
async function paperAction(request: PaperRequest) {
  if (paperActionPending && request.operation !== 'pause') return;
  const generation = ++paperActionGeneration;
  const visibility = paperVisibilityGeneration;
  paperActionPending = true; paperRequestError = ''; announce('paper-error', '');
  element('paper-status').textContent = request.operation === 'start' ? 'Go requested · waiting for confirmation. A request is not a fill.' : `Requesting ${request.operation.replaceAll('_', ' ')}…`;
  paperControls();
  try {
    const response = await invoke<PaperResponse>('paper', request);
    if (generation !== paperActionGeneration) return;
    if (response.kind !== 'snapshot') throw new Error('Unexpected paper action response');
    ++paperResponseEpoch;
    if (visibility === paperVisibilityGeneration && paperPanel.open && !document.hidden && request.operation === 'start' && response.run?.run_id !== paperSnapshot?.run?.run_id) {
      paperSelection = null; ++paperSelectionGeneration; paperEvidenceCursor = null;
      element('paper-result-title').textContent = 'No market entered'; element('paper-reason').textContent = 'The new run waits for its eligible full market boundary.'; element('paper-receipt').replaceChildren();
      element('paper-detail').textContent = 'Select a participating market to inspect its confirmed facts.';
    }
    if (visibility === paperVisibilityGeneration && paperPanel.open && !document.hidden) renderPaper(response);
  } catch (failure) {
    if (generation !== paperActionGeneration) return;
    ++paperResponseEpoch;
    if (request.operation === 'start' && failure && typeof failure === 'object' && 'code' in failure && ['range', 'invalid_request', 'unsupported_input', 'run_active'].includes(String(failure.code))) paperStartRequest = null;
    if (visibility === paperVisibilityGeneration && paperPanel.open && !document.hidden) { paperRequestError = paperFailure(failure); announce('paper-error', paperRequestError); }
  } finally {
    if (generation === paperActionGeneration) { paperActionPending = false; paperHistoryKey = ''; paperControls(); requestPaperRead('summary'); }
  }
}
element('paper-go').addEventListener('click', () => {
  if (element<HTMLButtonElement>('paper-go').disabled) return;
  paperStartRequest ??= { operation: 'start', start_uuid: crypto.randomUUID(), version_id: paperVersion.value, settings: { quantity: element<HTMLInputElement>('paper-quantity').value, minimum_price: element<HTMLInputElement>('paper-minimum').value, maximum_price: element<HTMLInputElement>('paper-maximum').value, starting_balance: element<HTMLInputElement>('paper-balance').value,
    maximum_loss: element<HTMLInputElement>('paper-loss-limit').value,
    profit_target: element<HTMLInputElement>('paper-profit-target').value || null,
    resume_next_day: element<HTMLInputElement>('paper-next-day').checked,
  } };
  void paperAction(paperStartRequest);
});
for (const [id, operation] of [['pause', 'pause'], ['resume', 'resume'], ['stop', 'stop_after_market'], ['settle', 'check_settlement']] as const) {
  element(`paper-${id}`).addEventListener('click', () => { const run = paperSnapshot?.run; if (run) void paperAction({ operation, run_id: run.run_id }); });
}
element('paper-refresh').addEventListener('click', () => { paperHistoryKey = ''; requestPaperRead('summary'); });
element('paper-evidence-more').addEventListener('click', () => requestPaperRead('evidence_more'));
element('paper-more').addEventListener('click', () => requestPaperRead('more'));
paperVersion.addEventListener('change', paperControls);
element('paper-example').addEventListener('click', () => { editor.value = learningExample; changedText(); strategyStatus.textContent = 'Learning example loaded. Preview and save it, then choose its saved version for paper.'; editor.focus(); });
paperPanel.addEventListener('toggle', () => { ++paperVisibilityGeneration; if (paperPanel.open) requestPaperRead('summary'); else if (paperTimer !== undefined) window.clearTimeout(paperTimer); });
element('paper-details').addEventListener('toggle', () => { if (element<HTMLDetailsElement>('paper-details').open) requestPaperRead('detail'); });
paperControls(); requestPaperRead('summary');
