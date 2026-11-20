export type Action = 'start' | 'pause' | 'advance';
export type Record = { id: number; status: 'stopped' | 'running' | 'paused'; tick: number; reason: string; recorded_at: string };
export type Snapshot = {
  running: boolean; pending: number; recording_error: string | null; history: Record[];
  writer_settings: { journal_mode: string; synchronous: number; fullfsync: number; checkpoint_fullfsync: number; sqlite_version: string };
};
export type StrategyRequest =
  | { operation: 'preview'; json: string }
  | { operation: 'save'; json: string; reviewed_json: string }
  | { operation: 'list'; before_id?: string }
  | { operation: 'detail'; version_id: string };
export type StrategySummary = { id: string; recorded_at: string; display_name: string | null; format_revision: number | null; status: 'supported' | 'legacy_read_only' | 'unsupported' | 'invalid_record' };
export type StrategyDetail = { summary: StrategySummary; json: string | null; explanation: string | null; meaning_id: string | null; notice: string };
export type RuleError = { path: string; code: string; message: string; accepted_form: string };
export type Requirements = { support_profile: string; planned_bytes: number; startup_ms: { [id: string]: number }; per_node_bytes: { [id: string]: number }; shared_underlying_bytes: number; per_market_bytes: number; implicit_buyable_asks: string[] };
export type StrategyResponse =
  | { kind: 'preview'; format_revision: number; meaning_id: string; explanation: string; requirements: Requirements; notice: string }
  | { kind: 'rule_errors'; errors: RuleError[]; truncated: boolean }
  | { kind: 'saved'; version: StrategySummary }
  | { kind: 'versions'; versions: StrategySummary[]; next_cursor: string | null }
  | { kind: 'detail'; version: StrategyDetail };

export type SourceName = 'kalshi' | 'brti' | 'coinbase' | 'kraken';
export type SourceRequest = { operation: 'snapshot' } | { operation: 'details' | 'connect' | 'disconnect'; source: SourceName } | { operation: 'inspect'; version_id: string };
export interface SourceBinding { id: string; source: SourceName; field: string; unit: string; market_id: string | null; side: string | null; value: string | null; identity: string | null; provider_time_utc: string | null; age_ms: string | null; usable: boolean; history_ready: boolean; reason: string | null; capabilities: { current: string | null; observations: string | null; history: string | null } }
export interface SourceCounters { provider: string; rest_attempts: string; handshake_attempts: string; control_writes: string; pong_flush_attempts: string; data_messages: string; slow_retry: boolean }
export interface SourceSnapshot { kind: 'snapshot'; owner: string; revision: string; observed_at_utc_ms: string; environment: 'demo' | 'production'; current_market: string | null; markets: { market_id: string; open_ms: string; close_ms: string; verified: boolean; preparation: string }[]; sources: { source: SourceName; connection: string; enabled: boolean }[]; bindings: SourceBinding[]; counters?: SourceCounters[] }
export type SourceResponse = SourceSnapshot | { kind: 'details'; source: SourceName; values: unknown[]; bindings?: SourceBinding[]; notice: string; counters?: SourceCounters[] } | { kind: 'requirements'; version_id: string; requirements: { input_id: string; source: string; instrument: string; field: string; unit: string; side: string | null; startup_ms: string; implicit: boolean }[] } | { kind: 'action'; source: SourceName; accepted: boolean };
export type ProviderCredentialResponse = { kind: 'missing' } | { kind: 'saved'; key_id: string; key_type: string } | { kind: 'credential_changed'; operation: string };

export interface PaperRun {
  run_id: string; start_uuid: string; strategy_version_id: string; revision: string; history_revision: string;
  control_state: 'collecting' | 'waiting' | 'active' | 'paused' | 'stopped' | 'unresolved';
  pending_holdings: boolean; cash: string; reserved: string; virtual_frontier_ms: string; last_evaluated_through_ms: string;
  completed_ordinal: string; pending_ordinal: string | null; error_code: string | null;
  quantity: string; minimum_price: string; maximum_price: string; starting_balance: string;
  fee_model: string; simulation_model: string; scenario_revision: string; go_utc_ms: string;
}
export interface PaperWitness {
  path: string; root_path: string; root_kind: string; root_status: string; option_id: string | null; checkpoint_id: string | null;
  side: string | null; kind: string; status: string; interval_state: string; interval_start_seconds: string; interval_end_seconds: string;
  blocked_by_checkpoint: string | null; left_reference: string | null; right_reference: string | null;
  observed: string | null; required: string | null; comparison: string | null; unit: string | null; evaluated_at_ms: string; transaction: string;
}
export interface PaperReceipt {
  run_id: string; strategy_version_id: string; paper_market_id: string; source_market_id: string; source_contract_id: string; series_id: string;
  open_utc_ms: string; close_utc_ms: string; close_day: string; reason: string; primary_witness: PaperWitness | null; purchase_check: { evaluated_at_ms: string; offered_price: string | null; minimum_price: string; maximum_price: string; available_funds: string; required_debit: string | null } | null; more_evidence: boolean; requested_quantity: string;
  attempt_state: 'intent' | 'dispatched' | 'full' | 'zero' | 'canceled' | 'unknown' | null;
  side: string | null; price: string | null; available_millionths: string | null; attempt_virtual_ms: string | null;
  filled_quantity: string | null; cost: string | null; fee: string | null; debit: string | null; payout: string | null;
  settled_net: string | null; winning_side: string | null; settlement_virtual_ms: string | null; settlement_recorded_at: string | null;
}
export interface PaperSettings { quantity: string; minimum_price: string; maximum_price: string; starting_balance: string }
export type PaperRequest =
  | { operation: 'start'; start_uuid: string; version_id: string; settings: PaperSettings }
  | { operation: 'pause' | 'resume' | 'stop_after_market' | 'check_settlement'; run_id: string }
  | { operation: 'snapshot' }
  | { operation: 'history'; run_id?: string; before_paper_market_id?: string }
  | { operation: 'market'; run_id: string; paper_market_id: string }
  | { operation: 'evidence'; run_id: string; paper_market_id: string; before_evidence_id?: string };
export interface PaperSnapshot {
  kind: 'snapshot'; run: PaperRun | null; effective_paused: boolean; stop_after_market?: boolean;
  effective_generation: string; playback_error?: string | null; completion_revision?: string | null;
  origin: 'synthetic_controlled'; clock_multiplier: '20';
}
export type PaperResponse = PaperSnapshot
  | { kind: 'history'; page: { markets: PaperReceipt[]; next_cursor: string | null } }
  | { kind: 'market'; run: PaperRun; receipt: PaperReceipt; assumptions: { [key: string]: string } }
  | { kind: 'evidence'; page: { evidence: { evidence_id: string; first_virtual_ms: string; last_virtual_ms: string; repetitions: string; witness: unknown }[]; next_cursor: string | null } };
