import type { Snapshot, SourceSnapshot, SourceName, PaperReceipt, PaperWitness, PaperRecovery } from './contract';

export function presentation(snapshot: Snapshot) {
  const failed = snapshot.recording_error !== null;
  const busy = snapshot.pending > 0;
  return {
    status: failed ? 'Stopped — recording needs attention' : snapshot.running ? 'Running · controlled non-trading session' : busy ? 'Stopped · recording control change…' : snapshot.history[0].status === 'paused' ? 'Paused · no test events running' : 'Stopped · ready when you are',
    recording: failed ? 'Requested change not confirmed saved. History below shows the last confirmed records.' : busy ? 'Saving… This change is not yet confirmed saved.' : 'All displayed records are saved.',
    startDisabled: failed || busy || snapshot.running,
    advanceDisabled: failed || busy || !snapshot.running,
    pauseDisabled: failed || (!busy && !snapshot.running),
  };
}

export function strategyFailure(failure: unknown, saving = false) {
  const code = failure && typeof failure === 'object' && 'code' in failure ? failure.code : '';
  if (code === 'busy') return 'Strategy action already in progress. Wait and retry.';
  if (code === 'not_reviewed') return 'Preview the current text before saving.';
  if (code === 'quitting') return 'Quitting — wait for the stopped state to be saved.';
  if (code === 'resource_limit') return 'Strategy content is too large to process or show completely. Stored history is preserved.';
  return saving ? 'Save not confirmed — inspect saved versions before retrying.' : 'Local strategy action failed. Wait and retry.';
}

export function sourceFailure(failure: unknown) {
  const code = failure && typeof failure === 'object' && 'code' in failure && typeof failure.code === 'string' ? failure.code : '';
  const messages: { [code: string]: string } = { busy: 'A source action is in progress. Wait and retry.', missing: 'Kalshi credentials are missing. Save your configured key before connecting.', already_saved: 'A credential is already saved. Use Replace to change it.', denied: 'Access was explicitly refused. Check source permissions.', invalid_credential: 'The key ID or private key is invalid or unsupported.', still_stopping: 'Data intake is stopping. Wait before reconnecting.', quitting: 'The application is stopping.', ambiguous: 'Credential change could not be confirmed. Check status before reconnecting.', capacity: 'Source data exceeds its bounded allowance.', invalid_request: 'Invalid source request.', not_found: 'Saved version unavailable. Refresh saved versions.' };
  return Object.hasOwn(messages, code) ? messages[code] : 'Source action unavailable. Inspect its status and retry explicitly.';
}
export function sourcePresentation(snapshot: SourceSnapshot, name: SourceName) {
  const state = snapshot.sources.find(s => s.source === name);
  const labels: { [connection: string]: string } = { disconnected: 'Disconnected', missing_credentials: 'Credentials missing', connecting: 'Connecting…', open: 'Connection open', access_refused: 'Access explicitly refused' };
  const lines = [state && Object.hasOwn(labels, state.connection) ? labels[state.connection] : 'Source state unavailable'];
  const bindings = snapshot.bindings.filter(b => b.source === name);
  const primary = bindings.filter(b => name !== 'kalshi' || (b.field === 'buyable_ask' && (!snapshot.current_market || b.market_id === snapshot.current_market)));
  const received = primary.filter(b => b.value !== null);
  for (const b of received) {
    const price = b.unit === 'contract_cents' ? `${b.value}¢` : `${b.value} USD/BTC`;
    lines.push(`${b.side ? b.side.toUpperCase() + ' ask · ' : ''}${price}`);
    lines.push(`${b.market_id ?? b.identity ?? name} · receipt age ${b.age_ms === null ? 'unknown' : b.age_ms + ' ms'}`);
  }
  const reasons: { [reason: string]: string } = { market_closed: 'Market closed — last value retained.', stale_data: 'Stale data — waiting for a verified update.', unknown_freshness: 'Freshness has not been verified.', gap: 'Data gap — rebuilding.', disconnected: received.length ? 'Last value retained; source disconnected.' : 'Source disconnected.', invalid_value: 'Invalid source data.', inconsistent_evidence: 'Conflicting source data.', over_age: 'Intake exceeded its age limit.', overflow: 'Intake capacity exceeded.', access_refused: 'Access explicitly refused.', awaiting_data: 'Awaiting verified data.' };
  const unavailable = [...new Set(primary.filter(b => !b.usable).map(b => b.reason ?? b.capabilities.current ?? 'awaiting_data'))];
  if (!received.length && state?.enabled && !unavailable.length) unavailable.push('awaiting_data');
  for (const reason of unavailable) lines.push(Object.hasOwn(reasons, reason) ? reasons[reason] : 'This input is unavailable for strategy use.');
  if (received.some(b => !b.history_ready)) lines.push('Complete history is unverified.');
  return lines.join('\n');
}


export function paperMoney(amount: string): string {
  if (!/^-?(0|[1-9][0-9]*)$/.test(amount)) throw new Error('Invalid exact amount');
  const value = BigInt(amount);
  const magnitude = value < 0n ? -value : value;
  const fraction = (magnitude % 100000000n).toString().padStart(8, '0').replace(/0+$/, '').padEnd(2, '0');
  return `${value < 0n ? '−' : ''}$${magnitude / 100000000n}.${fraction}`;
}
export function paperFailure(failure: unknown): string {
  const code = failure && typeof failure === 'object' && 'code' in failure && typeof failure.code === 'string' ? failure.code : '';
  const messages: Record<string, string> = {
    busy: 'Paper work is busy. Inspect status, then retry explicitly.', range: 'Check the quantity, price range, starting balance and daily limits.', invalid_request: 'The paper request is invalid. Check the fixed settings.',
    unsupported_input: 'This controlled example supports Kalshi asks with no historical lookback. Choose a compatible saved version.',
    run_active: 'Finish the current run and resolve its holdings before starting another example.', stale_run: 'This control belongs to an older run. Refresh paper status.',
    stopped_context: 'Purchases are stopped. Reopening never restarts playback.', unresolved: 'A purchase outcome is unresolved. New purchases remain blocked; confirmed history is retained.',
    checking_purchase: 'A purchase is still being checked. Purchases stay paused until its outcome is confirmed.',
    report_conflict: 'The venue report contradicts recorded purchase or settlement facts. Confirmed accounting is kept and purchases remain blocked.',
    storage: 'Recording could not be confirmed. Purchases are fenced. Inspect retained history and retry Quit after fixing storage.',
    unconfirmed: 'This action is not confirmed. Inspect paper status before retrying.', resource_limit: 'The controlled input or response exceeds its supported bound. Purchases remain blocked.',
    input_conflict: 'Conflicting paper input was refused. Purchases are stopped.', input_gap: 'A required paper input is missing. Purchases are stopped.', quitting: 'The application is stopping.',
  };
  return Object.hasOwn(messages, code) ? messages[code] : 'Paper state is unavailable. Pause purchases and refresh status.';
}
export function paperReason(reason: string): string {
  const messages: Record<string, string> = {
    prepared: 'Prepared for a future boundary; this market has not participated.',
    confirmed_full_fill_pending_settlement: 'Simulated fills for the whole requested quantity are confirmed. The holding awaits its identified synthetic outcome.',
    confirmed_partial_fill_pending_settlement: 'Part of the requested quantity is confirmed and locked to its side. Later attempts may only complete the unfilled remainder; the holding awaits its identified synthetic outcome.',
    insufficient_quantity: 'No contracts filled from the finite observed quantity. A zero fill is confirmed; no fee was charged.',
    settled: 'The identified synthetic outcome is recorded and the account reconciles after fees.',
    canceled_before_dispatch: 'The proposal was canceled before dispatch. No purchase was confirmed.',
    checking_purchase: 'A purchase may have been sent and its outcome is being checked. Its reserved funds stay held; confirmed contracts are kept.',
    daily_budget: 'The whole purchase would exceed the remaining daily loss allowance.',
    daily_entry_block: 'A daily threshold blocked entry into this market.',
    funds_unavailable: 'Current funds could not be verified.',
    insufficient_funds: 'The whole objective, including its fee, could not be funded. Quantity was not reduced.',
    evidence_mismatch: 'Waiting for a finite execution book matching the exact evaluated ask identity and price.',
    purchases_paused: 'The rule qualified while purchases were paused. Monitoring continued; no purchase was confirmed.',
    price: 'The evaluated purchase price is outside the fixed permitted range.',
    unavailable: 'Required rule evidence was unavailable at the recorded check.', unavailable_evidence: 'Required source or execution evidence could not be established.',
    condition_false: 'The purchase option did not qualify at the recorded check. View details for retained rule evidence.',
    objective_blocked: 'The rule qualified, but the objective safeguards did not permit another purchase.',
    eligible: 'The rule qualified. A confirmed purchase still requires coherent execution evidence, funded intent and a recorded outcome.',
    lower_priority: 'An earlier eligible purchase option takes priority.', ambiguous_side: 'The rule did not establish one unique purchase side.',
  };
  return Object.hasOwn(messages, reason) ? messages[reason] : `Recorded reason: ${reason}`;
}
export function paperWitness(witness: PaperWitness): string {
  const name = `${witness.option_id ? 'Option ' + witness.option_id : 'Checkpoint ' + (witness.checkpoint_id ?? 'check')} · ${witness.path}`;
  const when = `At ${witness.evaluated_at_ms} ms on the synthetic clock`;
  if (witness.blocked_by_checkpoint) return `${when}, ${name} could not select a side: checkpoint ${witness.blocked_by_checkpoint} was not latched.`;
  if (witness.interval_state !== 'within') return `${when}, ${name} was outside its ${witness.interval_start_seconds}–${witness.interval_end_seconds} second interval (${witness.interval_state}).`;
  if (witness.status === 'unavailable') return `${when}, could not evaluate ${name}: ${witness.left_reference ?? witness.right_reference ?? witness.kind.replaceAll('_', ' ')} was unavailable.`;
  if (witness.kind === 'checkpoint_reached' || witness.kind === 'after_checkpoint_update') return `${when}, ${name}: ${witness.kind === 'checkpoint_reached' ? 'checkpoint ' + witness.checkpoint_id + ' was ' + (witness.status === 'true' ? 'latched' : 'not latched') : 'a genuine update after checkpoint ' + witness.checkpoint_id + ' was ' + (witness.status === 'true' ? 'observed' : 'not established')}.`;
  const comparisons: Record<string, string> = { less_than: 'below', at_most: 'at most', equal_to: 'equal to', at_least: 'at least', greater_than: 'above' };
  const units: Record<string, string> = { contract_cents: '¢', usd_per_btc: ' USD/BTC', percent: '%' };
  const unit = witness.unit && Object.hasOwn(units, witness.unit) ? units[witness.unit] : witness.kind === 'held_for' ? ' seconds' : witness.kind === 'confirmed_updates' ? ' updates' : '';
  const check = witness.observed !== null && witness.required !== null
    ? `${witness.left_reference ?? witness.kind.replaceAll('_', ' ')} was ${witness.observed}${unit}; required ${witness.comparison && Object.hasOwn(comparisons, witness.comparison) ? comparisons[witness.comparison] : witness.kind === 'held_for' || witness.kind === 'confirmed_updates' ? 'at least' : ''} ${witness.required}${unit}`
    : `${witness.kind.replaceAll('_', ' ')} was ${witness.status}`;
  const root = witness.root_path !== witness.path ? ` The enclosing ${witness.root_kind === 'any' ? 'any-of' : 'all-of'} check was ${witness.root_status}.` : '';
  return `${when}, ${name}: ${check} (${witness.status}).${root}`;
}
export function paperReceipt(receipt: PaperReceipt) {
  const state = receipt.attempt_state;
  const confirmed = state === 'full' || state === 'partial';
  const terminal = confirmed || state === 'zero' || state === 'canceled';
  // Totals include only confirmed fills, so they stay true while a later attempt is unresolved.
  const held = terminal || (receipt.filled_quantity ?? '0') !== '0';
  const several = Number(receipt.attempt_count) > 1;
  const title = receipt.participated === false ? 'Market entry blocked' : confirmed ? `Bought ${receipt.filled_quantity ?? '0'}${state === 'partial' ? ' of ' + receipt.requested_quantity : ''} ${(receipt.side ?? '').toUpperCase()} · ${receipt.settled_net === null ? 'pending settlement' : 'settled'}`
    : state === 'zero' ? 'Attempted · zero contracts filled'
    : state === 'intent' || state === 'dispatched' || state === 'unknown' ? 'Purchase outcome unconfirmed'
    : state === 'canceled' ? 'Canceled before dispatch' : 'No confirmed purchase';
  const facts: [string, string][] = [['Paper run', receipt.run_id], ['Saved strategy version', receipt.strategy_version_id], ['Market', receipt.source_market_id], ['Requested quantity', receipt.requested_quantity], ['Confirmed quantity', held ? receipt.filled_quantity ?? '0' : state === null ? 'No purchase' : 'Unconfirmed']];
  if (several) facts.push(['Attempts', receipt.attempt_count]);
  if (receipt.risk_block) facts.push(['Risk decision', paperReason(receipt.risk_block)]);
  if (receipt.recovery) facts.push(['Purchase check', paperRecovery(receipt.recovery)]);
  if (receipt.side !== null) facts.push(['Side', receipt.side.toUpperCase()]);
  if (receipt.price !== null) facts.push([several ? 'Latest attempt price per contract' : 'Requested price per contract', paperMoney(receipt.price)]);
  if (receipt.attempt_virtual_ms !== null) facts.push([several ? 'Latest attempt time' : 'Attempt time', `${receipt.attempt_virtual_ms} ms on the synthetic clock`]);
  if (receipt.available_millionths !== null) facts.push([several ? 'Latest attempt observed available quantity' : 'Observed available quantity', paperQuantity(receipt.available_millionths)]);
  if (held && receipt.debit !== null) facts.push(['Confirmed total debit', paperMoney(receipt.debit)]);
  if (held && receipt.cost !== null) facts.push(['Confirmed cost', paperMoney(receipt.cost)]);
  if (held && receipt.fee !== null) facts.push(['Confirmed fee', paperMoney(receipt.fee)]);
  if (confirmed) facts.push(['Payout', receipt.payout === null ? 'Pending settlement' : paperMoney(receipt.payout)], ['Settled net', receipt.settled_net === null ? 'Pending settlement' : paperMoney(receipt.settled_net)]);
  const check = receipt.purchase_check;
  const block = check && ['price', 'insufficient_funds', 'purchases_paused', 'evidence_mismatch'].includes(receipt.reason)
    ? ` At ${check.evaluated_at_ms} ms, ${check.offered_price === null ? 'the offered price was unavailable' : 'the offered price was ' + check.offered_price + '¢'}; permitted range ${check.minimum_price}–${check.maximum_price}¢.${receipt.reason === 'insufficient_funds' && check.required_debit !== null ? ' Full debit required ' + paperMoney(check.required_debit) + '; available ' + paperMoney(check.available_funds) + '.' : ''}` : '';
  return { title, reason: (['condition_false', 'unavailable', 'unavailable_evidence', 'eligible'].includes(receipt.reason) && receipt.primary_witness ? paperWitness(receipt.primary_witness) : paperReason(receipt.participated === false ? receipt.risk_block ?? receipt.reason : receipt.reason) + (['ambiguous_side', 'objective_blocked'].includes(receipt.reason) && receipt.primary_witness ? ' ' + paperWitness(receipt.primary_witness) : '')) + block, facts };
}

export function paperChecking(recovery: PaperRecovery | null | undefined): boolean {
  return !!recovery && ['intent', 'dispatched', 'unknown'].includes(recovery.state);
}
export function paperRecovery(recovery: PaperRecovery): string {
  const causes: Record<string, string> = { no_outcome: 'Its reply was not recorded.', restart: 'Metten restarted before its outcome was recorded.', legacy: 'An earlier Metten recorded it without a dispatch marker, so sending cannot be ruled out.' };
  const cause = recovery.interrupted && Object.hasOwn(causes, recovery.interrupted) ? ' ' + causes[recovery.interrupted] : '';
  const attempt = `${recovery.requested_quantity} ${recovery.side.toUpperCase()} in ${recovery.source_market_id}`;
  const held = ` Confirmed in this market: ${recovery.confirmed_quantity}.`;
  if (paperChecking(recovery)) return `Checking a purchase of ${attempt}.${cause} No other purchase is sent, and the funds reserved for this purchase stay held until its outcome is confirmed.${held}`;
  if (recovery.state === 'canceled') return `Purchase check complete: the attempt for ${attempt} was never sent and was canceled.${cause}${held}`;
  return `Purchase check complete: ${recovery.filled_quantity} of ${attempt} filled.${cause}${held}`;
}

export function paperQuantity(amount: string): string {
  if (!/^(0|[1-9][0-9]*)$/.test(amount)) throw new Error('Invalid exact quantity');
  const value = BigInt(amount);
  const fraction = (value % 1000000n).toString().padStart(6, '0').replace(/0+$/, '');
  return `${value / 1000000n}${fraction ? '.' + fraction : ''}`;
}
