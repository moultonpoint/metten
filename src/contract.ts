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
