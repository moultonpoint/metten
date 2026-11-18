import type { Snapshot, SourceSnapshot, SourceName } from './contract';

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
