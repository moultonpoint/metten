//! Synchronous trusted source evidence. Provider protocols and evaluator history live elsewhere.
use crate::strategy::{self, *};
use std::sync::atomic::{AtomicU64, Ordering};

const BINDINGS: usize = 128;
const VIEW_BYTES: usize = 256 * 1024;
static NEXT_OWNER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provider {
    Kalshi,
    Coinbase,
    Kraken,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stream {
    Book,
    Trades,
    Brti,
    Metadata,
    Coinbase,
    Kraken,
}
impl Stream {
    fn index(self) -> usize {
        self as usize
    }
    fn source(self) -> Source {
        match self {
            Self::Book | Self::Trades | Self::Metadata => Source::Kalshi,
            Self::Brti => Source::Brti,
            Self::Coinbase => Source::Coinbase,
            Self::Kraken => Source::Kraken,
        }
    }
    fn provider(self) -> Provider {
        match self {
            Self::Coinbase => Provider::Coinbase,
            Self::Kraken => Provider::Kraken,
            _ => Provider::Kalshi,
        }
    }
}
const STREAMS: [Stream; 6] = [
    Stream::Book,
    Stream::Trades,
    Stream::Brti,
    Stream::Metadata,
    Stream::Coinbase,
    Stream::Kraken,
];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unavailable {
    Awaiting,
    Freshness,
    Observations,
    History,
    Precision,
    Reference,
    Access,
    Rights,
    Gap,
    Overflow,
    OverAge,
    InvalidValue,
    Inconsistent,
    MetadataChanged,
    Disconnected,
    Stopped,
}
impl Unavailable {
    pub fn code(self) -> &'static str {
        match self {
            Self::Awaiting => "awaiting_data",
            Self::Freshness => "unknown_freshness",
            Self::Observations => "unsupported_observations",
            Self::History => "unsupported_history",
            Self::Precision => "unsupported_precision",
            Self::Reference => "unavailable_reference",
            Self::Access => "access_refused",
            Self::Rights => "unresolved_rights",
            Self::Gap => "gap",
            Self::Overflow => "overflow",
            Self::OverAge => "over_age",
            Self::InvalidValue => "invalid_value",
            Self::Inconsistent => "inconsistent_evidence",
            Self::MetadataChanged => "metadata_changed",
            Self::Disconnected => "disconnected",
            Self::Stopped => "stopped",
        }
    }
}
/// None means substantiated by the trusted caller, not by a received price or this module.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Capabilities {
    pub current: Option<Unavailable>,
    pub observations: Option<Unavailable>,
    pub history: Option<Unavailable>,
}
impl Default for Capabilities {
    fn default() -> Self {
        Self {
            current: Some(Unavailable::Freshness),
            observations: Some(Unavailable::Observations),
            history: Some(Unavailable::History),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Connection {
    Disconnected,
    MissingConfiguration,
    Connecting,
    Open,
    AccessRefused,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Subscription {
    Requested,
    Acknowledged,
    Initialized,
    Rebuilding,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StreamToken {
    stream: Stream,
    owner: u64,
    generation: u64,
}
#[derive(Clone, Copy, Debug)]
pub struct Clock {
    pub at_ms: u64,
    pub receipt_ms: u64,
    pub received_through_ms: u64,
    pub completed_frontier_ms: u64,
}
#[derive(Clone, Copy, Debug)]
pub struct GapInterval {
    pub start_ms: u64,
    pub end_ms: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    Stream(Stream),
    Source(Source),
    Provider(Provider),
    Market,
    All,
}
#[derive(Clone, Copy, Debug)]
pub enum ValueUpdate {
    Initialize(Decimal),
    Observation { number: u64, value: Decimal },
}
#[derive(Clone, Debug)]
pub struct BindingUpdate {
    pub binding: BindingDescriptor,
    pub unit: RuleUnit,
    pub value: Option<ValueUpdate>,
    pub valid_until_ms: Option<u64>,
}
#[derive(Clone, Debug)]
pub enum SourceEvent {
    Connection {
        provider: Provider,
        token: StreamToken,
        state: Connection,
    },
    Subscription {
        stream: Stream,
        token: StreamToken,
        state: Subscription,
    },
    Update {
        stream: Stream,
        token: StreamToken,
        updates: Vec<BindingUpdate>,
        coverage: Vec<CoverageFact>,
    },
    RegisterMarket {
        token: StreamToken,
        descriptor: MarketDescriptor,
        bindings: Vec<BindingDescriptor>,
        capabilities: Vec<(u32, Capabilities)>,
    },
    ReplaceMarket {
        token: StreamToken,
        expected: MarketDescriptor,
        replacement: MarketDescriptor,
    },
    Metadata {
        token: StreamToken,
        market_id: String,
        verified: bool,
    },
    Boundary {
        market_id: String,
    },
    Advance,
}
#[derive(Clone, Debug)]
pub struct Envelope {
    pub clock: Clock,
    pub provider_identity: Option<String>,
    pub provider_time_utc: Option<String>,
    pub event: SourceEvent,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceFailure {
    pub code: &'static str,
    pub message: &'static str,
}
fn failure(code: &'static str) -> SourceFailure {
    SourceFailure {
        code,
        message: match code {
            "stopped" => "Source intake is stopped.",
            "resource_limit" => "Source evidence exceeds its bounded allowance.",
            "obsolete_generation" => "Source evidence belongs to a revoked connection.",
            "identity_mismatch" => "Source evidence does not match its registered input.",
            "invalid_order" => "Source evidence has an invalid accepted time or order.",
            "invalid_value" => "Source evidence has an incompatible price.",
            "inconsistent_evidence" => "Source observations contradict an accepted identity.",
            _ => "Invalid source request.",
        },
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct StampEntry {
    id: u32,
    incarnation: u64,
    generation: u64,
    stream_generation: u64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceStamp {
    owner: u64,
    market_revision: u64,
    entries: Vec<StampEntry>,
}
#[derive(Clone, Debug)]
pub struct SourceEffect {
    pub batch: AdvanceBatch,
    pub stamp: EvidenceStamp,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Preparation {
    Unknown,
    Identified,
    Collecting,
    Ready,
}
#[derive(Clone, Debug)]
pub struct MarketView {
    pub descriptor: MarketDescriptor,
    pub verified: bool,
    pub preparation: Preparation,
}
#[derive(Clone, Debug)]
pub struct BindingView {
    pub binding: BindingDescriptor,
    pub requirement: SourceRequirement,
    pub capabilities: Capabilities,
    pub value: Option<Decimal>,
    pub provider_identity: Option<String>,
    pub provider_time_utc: Option<String>,
    pub age_ms: Option<u64>,
    pub valid_until_ms: u64,
    pub usable: bool,
    pub history_ready: bool,
    pub covered_through_ms: Option<u64>,
    pub reason: Option<Unavailable>,
    pub initialized: bool,
    pub generation: u64,
}
#[derive(Clone, Debug)]
pub struct SourceSnapshot {
    pub owner: u64,
    pub revision: u64,
    pub at_ms: u64,
    pub stopped: bool,
    pub current_market: Option<String>,
    pub sources: Vec<(Source, Connection)>,
    pub streams: Vec<(Stream, Subscription, StreamToken)>,
    pub markets: Vec<MarketView>,
    pub bindings: Vec<BindingView>,
}
struct Bound {
    descriptor: BindingDescriptor,
    requirement: usize,
    caps: Capabilities,
    stream: Stream,
    incarnation: u64,
    generation: u64,
    value: Option<Decimal>,
    number: u64,
    observed_value: Option<Decimal>,
    initialized: bool,
    valid_until: u64,
    receipt: Option<u64>,
    provider_identity: Option<String>,
    provider_time: Option<String>,
    coverage: Option<(u64, u64)>,
    recovery_at: u64,
    reason: Option<Unavailable>,
}
struct Market {
    descriptor: MarketDescriptor,
    verified: bool,
    entered: bool,
    ready_since: Option<u64>,
}
/// Owns only received source facts. A successful apply is not completed evaluation.
pub struct SourceState {
    owner: u64,
    requirements: Vec<SourceRequirement>,
    bindings: Vec<Bound>,
    markets: Vec<Market>,
    connections: [Connection; 3],
    streams: [(u64, Subscription); 6],
    revision: u64,
    incarnation: u64,
    transaction: u64,
    last_at: u64,
    market_revision: u64,
    current: Option<String>,
    initial_boundary_emitted: bool,
    series_id: Option<String>,
    // Error returns carry no batch; require explicit revocation delivery before more intake.
    revocation_required: u8,
    stopped: bool,
}
impl SourceState {
    pub fn new(
        strategy: &ValidatedStrategy,
        markets: &[MarketDescriptor],
        bindings: &[BindingDescriptor],
        capabilities: &[(u32, Capabilities)],
    ) -> Result<Self, SourceFailure> {
        if markets.len() > 2 {
            return Err(failure("resource_limit"));
        }
        Self::with_requirements(
            strategy.source_requirements(),
            markets,
            bindings,
            capabilities,
        )
    }
    /// Fixed read-only catalog; no strategy, evaluator or capability authorization is created.
    pub fn display() -> Result<Self, SourceFailure> {
        let mut requirements = Vec::new();
        for field in ["bid", "buyable_ask", "last_trade"] {
            requirements.push(SourceRequirement {
                input_id: field.into(),
                source: Source::Kalshi,
                instrument: "current_contract".into(),
                field: field.into(),
                unit: RuleUnit::ContractCents,
                side: Some(SourceSide::Selected),
                startup_ms: 0,
                implicit: false,
            });
        }
        for field in ["opening_reference", "settlement_reference"] {
            requirements.push(SourceRequirement {
                input_id: field.into(),
                source: Source::Kalshi,
                instrument: "current_market".into(),
                field: field.into(),
                unit: RuleUnit::UsdPerBtc,
                side: None,
                startup_ms: 0,
                implicit: false,
            });
        }
        for (id, source, field) in [
            ("brti", Source::Brti, "index_value"),
            ("coinbase", Source::Coinbase, "last_trade"),
            ("kraken", Source::Kraken, "last_trade"),
        ] {
            requirements.push(SourceRequirement {
                input_id: id.into(),
                source,
                instrument: "btc_usd".into(),
                field: field.into(),
                unit: RuleUnit::UsdPerBtc,
                side: None,
                startup_ms: 0,
                implicit: false,
            });
        }
        let bindings: Vec<_> = requirements
            .iter()
            .filter(|r| r.instrument == "btc_usd")
            .enumerate()
            .map(|(i, r)| BindingDescriptor {
                binding_id: i as u32 + 1,
                input_id: r.input_id.clone(),
                scope: BindingScope::Underlying,
            })
            .collect();
        Self::with_requirements(requirements, &[], &bindings, &[])
    }
    pub fn requirements(&self) -> &[SourceRequirement] {
        &self.requirements
    }
    fn with_requirements(
        requirements: Vec<SourceRequirement>,
        markets: &[MarketDescriptor],
        bindings: &[BindingDescriptor],
        capabilities: &[(u32, Capabilities)],
    ) -> Result<Self, SourceFailure> {
        if markets.len() > 2 {
            return Err(failure("resource_limit"));
        }
        let mut state = Self {
            owner: NEXT_OWNER
                .try_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
                .map_err(|_| failure("resource_limit"))?,
            requirements,
            bindings: Vec::new(),
            markets: Vec::new(),
            connections: [Connection::Disconnected; 3],
            streams: [(1, Subscription::Requested); 6],
            revision: 0,
            incarnation: 0,
            transaction: 0,
            last_at: 0,
            market_revision: 0,
            current: None,
            initial_boundary_emitted: false,
            series_id: None,
            revocation_required: 0,
            stopped: false,
        };
        for m in markets {
            state.validate_market(m)?;
            if state.series_id.is_none() {
                state.series_id = Some(m.series_id.clone());
            }
            state.markets.push(Market {
                descriptor: m.clone(),
                verified: false,
                entered: false,
                ready_since: None,
            });
        }
        state.validate_bindings(bindings, capabilities, None)?;
        state.add_bindings(bindings, capabilities);
        Ok(state)
    }
    pub fn token(&self, stream: Stream) -> StreamToken {
        StreamToken {
            stream,
            owner: self.owner,
            generation: self.streams[stream.index()].0,
        }
    }
    fn check_token(&self, stream: Stream, token: StreamToken) -> Result<(), SourceFailure> {
        if token != self.token(stream) {
            Err(failure("obsolete_generation"))
        } else {
            Ok(())
        }
    }
    fn clock(&self, c: Clock) -> Result<(), SourceFailure> {
        if c.receipt_ms > c.at_ms
            || c.at_ms < self.last_at
            || c.at_ms < c.completed_frontier_ms
            || c.at_ms > c.received_through_ms
        {
            Err(failure("invalid_order"))
        } else {
            Ok(())
        }
    }
    fn validate_market(&self, m: &MarketDescriptor) -> Result<(), SourceFailure> {
        let series = self.series_id.as_deref().unwrap_or(&m.series_id);
        strategy::validate_market_descriptor(m, series)
            .map_err(|_| failure("identity_mismatch"))?;
        if self.markets.iter().any(|old| {
            old.descriptor.market_id == m.market_id
                || (old.descriptor.open_ms < m.close_ms && m.open_ms < old.descriptor.close_ms)
        }) {
            return Err(failure("identity_mismatch"));
        }
        Ok(())
    }
    fn validate_bindings(
        &self,
        bindings: &[BindingDescriptor],
        caps: &[(u32, Capabilities)],
        extra: Option<&MarketDescriptor>,
    ) -> Result<(), SourceFailure> {
        if bindings.len() + self.bindings.len() > BINDINGS
            || std::mem::size_of_val(bindings) > 128 * 512
            || caps.len() > BINDINGS
        {
            return Err(failure("resource_limit"));
        }
        let mut markets: Vec<_> = self.markets.iter().map(|m| m.descriptor.clone()).collect();
        if let Some(m) = extra {
            markets.push(m.clone());
        }
        for (n, b) in bindings.iter().enumerate() {
            let r = self
                .requirements
                .iter()
                .find(|r| r.input_id == b.input_id)
                .ok_or_else(|| failure("identity_mismatch"))?;
            let side = match r.side {
                None => None,
                Some(SourceSide::Selected) => Some("selected_side"),
                Some(SourceSide::Fixed(Side::Yes)) => Some("yes"),
                Some(SourceSide::Fixed(Side::No)) => Some("no"),
            };
            strategy::validate_binding_scope(&r.instrument, side, &markets, b)
                .map_err(|_| failure("identity_mismatch"))?;
            if self.bindings.iter().any(|old| {
                old.descriptor.binding_id == b.binding_id
                    || (old.descriptor.input_id == b.input_id && old.descriptor.scope == b.scope)
            }) || bindings[..n].iter().any(|old| {
                old.binding_id == b.binding_id
                    || (old.input_id == b.input_id && old.scope == b.scope)
            }) {
                return Err(failure("identity_mismatch"));
            }
        }
        for (n, (id, _)) in caps.iter().enumerate() {
            if !bindings.iter().any(|b| b.binding_id == *id)
                || caps[..n].iter().any(|(old, _)| old == id)
            {
                return Err(failure("invalid_request"));
            }
        }
        self.incarnation
            .checked_add(bindings.len() as u64)
            .ok_or_else(|| failure("resource_limit"))?;
        Ok(())
    }
    fn add_bindings(&mut self, bindings: &[BindingDescriptor], caps: &[(u32, Capabilities)]) {
        for b in bindings {
            self.incarnation += 1;
            let requirement = self
                .requirements
                .iter()
                .position(|r| r.input_id == b.input_id)
                .unwrap();
            let r = &self.requirements[requirement];
            let stream = match r.source {
                Source::Kalshi if r.instrument == "current_market" => Stream::Metadata,
                Source::Kalshi if r.field == "last_trade" => Stream::Trades,
                Source::Kalshi => Stream::Book,
                Source::Brti => Stream::Brti,
                Source::Coinbase => Stream::Coinbase,
                Source::Kraken => Stream::Kraken,
            };
            self.bindings.push(Bound {
                descriptor: b.clone(),
                requirement,
                caps: caps
                    .iter()
                    .find(|(id, _)| *id == b.binding_id)
                    .map_or_else(Capabilities::default, |(_, c)| *c),
                stream,
                incarnation: self.incarnation,
                generation: 1,
                value: None,
                number: 0,
                observed_value: None,
                initialized: false,
                valid_until: 0,
                receipt: None,
                provider_identity: None,
                provider_time: None,
                coverage: None,
                recovery_at: self.last_at,
                reason: None,
            });
        }
    }
    fn next_revision(&mut self) -> Result<(), SourceFailure> {
        match self.revision.checked_add(1) {
            Some(n) => {
                self.revision = n;
                Ok(())
            }
            None => {
                self.stopped = true;
                Err(failure("resource_limit"))
            }
        }
    }
    fn effect(
        &self,
        clock: Clock,
        changes: Vec<Vec<EvidenceChange>>,
        coverage: Vec<CoverageFact>,
        ids: &[u32],
    ) -> Result<SourceEffect, SourceFailure> {
        let mut number = self.transaction;
        let mut transactions = Vec::new();
        for change in changes.into_iter().filter(|c| !c.is_empty()) {
            number = number
                .checked_add(1)
                .ok_or_else(|| failure("resource_limit"))?;
            transactions.push(NormalizedTransaction {
                number,
                at_ms: clock.at_ms,
                changes: change,
            });
        }
        let batch = AdvanceBatch {
            received_through_ms: clock.received_through_ms,
            target_ms: clock.at_ms,
            transactions,
            coverage,
            current_admission: self.admission(clock.at_ms),
        };
        if batch.transactions.len() > 4096
            || batch
                .ingress_bytes()
                .map_err(|_| failure("resource_limit"))?
                > 8 * 1024 * 1024
        {
            return Err(failure("resource_limit"));
        }
        Ok(SourceEffect {
            batch,
            stamp: self.stamp(ids)?,
        })
    }
    fn readiness(&self, at_ms: u64, coverage: &[CoverageFact]) -> [bool; 2] {
        std::array::from_fn(|i| {
            i < self.markets.len() && self.preparation(i, at_ms, coverage) == Preparation::Ready
        })
    }
    fn admission(&self, at_ms: u64) -> Admission {
        if self.stopped {
            Admission::Blocked("stopped")
        } else if self.markets.iter().any(|m| {
            !m.verified
                && i128::from(m.descriptor.open_ms) <= i128::from(at_ms)
                && i128::from(at_ms) < i128::from(m.descriptor.close_ms)
        }) {
            Admission::Blocked("unverified_metadata")
        } else {
            Admission::Allowed
        }
    }
    fn accepted(
        &mut self,
        effect: &mut SourceEffect,
        at: u64,
        ids: &[u32],
        previous_ready: [bool; 2],
    ) -> Result<(), SourceFailure> {
        self.next_revision()?;
        self.last_at = at;
        if let Some(t) = effect.batch.transactions.last() {
            self.transaction = t.number;
        }
        let ready = self.readiness(at, &[]);
        for (i, market) in self.markets.iter_mut().enumerate() {
            if !ready[i] {
                market.ready_since = None;
            } else if !previous_ready[i] || market.ready_since.is_none() {
                market.ready_since = Some(at);
            }
        }
        effect.batch.current_admission = self.admission(at);
        effect.stamp = self.stamp(ids)?;
        Ok(())
    }
    pub fn apply(&mut self, envelope: Envelope) -> Result<SourceEffect, SourceFailure> {
        if self.stopped {
            return Err(failure("stopped"));
        }
        if self.revocation_required != 0 {
            return Err(failure("invalid_request"));
        }
        let c = envelope.clock;
        self.clock(c)?;
        for text in [&envelope.provider_identity, &envelope.provider_time_utc]
            .into_iter()
            .flatten()
        {
            if text.len() > 128 || text.capacity() > 128 || text.chars().any(char::is_control) {
                return Err(failure("resource_limit"));
            }
        }
        // Reserve counter headroom before ordinary state mutation.
        if self.revision == u64::MAX
            || self.transaction > u64::MAX - 3
            || self.incarnation == u64::MAX
            || self.market_revision > u64::MAX - 2
        {
            self.stopped = true;
            return Err(failure("resource_limit"));
        }
        // Observe expiry before applying a renewal at the same accepted time.
        let mut previous_ready = self.readiness(c.at_ms, &[]);
        match envelope.event {
            SourceEvent::Update {
                stream,
                token,
                updates,
                coverage,
            } => {
                self.check_token(stream, token)?;
                if self.connections[stream.provider() as usize] != Connection::Open {
                    return Err(failure("invalid_request"));
                }
                if updates.len() > BINDINGS
                    || coverage.len() > BINDINGS
                    || updates.capacity() > BINDINGS
                    || coverage.capacity() > BINDINGS
                {
                    return Err(failure("resource_limit"));
                }
                let mut changes = Vec::new();
                let mut ids = Vec::new();
                for u in &updates {
                    if u.binding.input_id.capacity() > 64
                        || match &u.binding.scope {
                            BindingScope::Underlying => false,
                            BindingScope::Market { market_id } => market_id.capacity() > 128,
                            BindingScope::Contract {
                                market_id,
                                contract_id,
                                ..
                            } => market_id.capacity() > 128 || contract_id.capacity() > 128,
                        }
                    {
                        return Err(failure("resource_limit"));
                    }
                    let b = self
                        .bindings
                        .iter()
                        .find(|b| b.descriptor.binding_id == u.binding.binding_id)
                        .ok_or_else(|| failure("identity_mismatch"))?;
                    if b.stream != stream
                        || b.descriptor != u.binding
                        || self.requirements[b.requirement].unit != u.unit
                        || ids.contains(&u.binding.binding_id)
                    {
                        return Err(failure("identity_mismatch"));
                    }
                    ids.push(u.binding.binding_id);
                    if u.value.is_none() && (u.valid_until_ms.is_none() || !b.initialized) {
                        return Err(failure("invalid_request"));
                    }
                    if !b.initialized && matches!(u.value, Some(ValueUpdate::Observation { .. })) {
                        return Err(failure("invalid_request"));
                    }
                    if let Some(value) = u.value {
                        let v = match value {
                            ValueUpdate::Initialize(v)
                            | ValueUpdate::Observation { value: v, .. } => v,
                        };
                        let n = v.millionths();
                        if (u.unit == RuleUnit::ContractCents && !(0..=100_000_000).contains(&n))
                            || (u.unit == RuleUnit::UsdPerBtc && n <= 0)
                        {
                            let revoked = self.invalidate(
                                Scope::Stream(stream),
                                Unavailable::InvalidValue,
                                c.at_ms,
                                None,
                            );
                            self.revocation_required |= 1 << stream.index();
                            revoked?;
                            return Err(failure("invalid_value"));
                        }
                        match value {
                            ValueUpdate::Initialize(v) => {
                                changes.push(EvidenceChange::Resynchronize {
                                    binding_id: u.binding.binding_id,
                                    value: v,
                                })
                            }
                            ValueUpdate::Observation { number, value } => {
                                if number < b.number
                                    || (number == b.number && b.observed_value != Some(value))
                                {
                                    let revoked = self.invalidate(
                                        Scope::Stream(stream),
                                        Unavailable::Inconsistent,
                                        c.at_ms,
                                        None,
                                    );
                                    self.revocation_required |= 1 << stream.index();
                                    revoked?;
                                    return Err(failure("inconsistent_evidence"));
                                }
                                if number > b.number {
                                    changes.push(if b.caps.observations.is_none() {
                                        EvidenceChange::Observation {
                                            binding_id: u.binding.binding_id,
                                            observation_number: number,
                                            value,
                                        }
                                    } else {
                                        EvidenceChange::Resynchronize {
                                            binding_id: u.binding.binding_id,
                                            value,
                                        }
                                    });
                                }
                            }
                        }
                    }
                    if let Some(until) = u.valid_until_ms
                        && b.caps.current.is_none()
                    {
                        changes.push(EvidenceChange::Validity {
                            binding_id: u.binding.binding_id,
                            valid_until_ms: until,
                            reason: None,
                        });
                    }
                }
                let mut accepted_coverage = Vec::new();
                for fact in &coverage {
                    let b = self
                        .bindings
                        .iter()
                        .find(|b| b.descriptor.binding_id == fact.binding_id)
                        .ok_or_else(|| failure("identity_mismatch"))?;
                    let initializes = updates
                        .iter()
                        .any(|u| u.binding.binding_id == fact.binding_id && u.value.is_some());
                    if b.stream != stream
                        || fact.start_ms >= fact.end_ms
                        || fact.end_ms > c.received_through_ms
                        || fact.start_ms < b.recovery_at
                        || (!b.initialized && !initializes)
                    {
                        return Err(failure("invalid_request"));
                    }
                    if b.caps.history.is_none() {
                        accepted_coverage.push(fact.clone());
                    }
                    if !ids.contains(&fact.binding_id) {
                        ids.push(fact.binding_id);
                    }
                }
                if !self.initial_boundary_emitted
                    && updates.is_empty()
                    && !accepted_coverage.is_empty()
                {
                    // Continuous quiet proof preserves readiness; expiry and disjoint history do not.
                    let refreshed = self.readiness(c.at_ms, &accepted_coverage);
                    for (i, was_ready) in previous_ready.iter_mut().enumerate() {
                        *was_ready |= refreshed[i]
                            && self.markets.get(i).is_some_and(|m| m.ready_since.is_some());
                    }
                }
                let mut effect = self.effect(c, vec![changes], accepted_coverage, &ids)?;
                for u in updates {
                    let b = self
                        .bindings
                        .iter_mut()
                        .find(|b| b.descriptor.binding_id == u.binding.binding_id)
                        .unwrap();
                    if let Some(value) = u.value {
                        match value {
                            ValueUpdate::Initialize(v) => {
                                b.value = Some(v);
                                b.initialized = true;
                            }
                            ValueUpdate::Observation { number, value } if number > b.number => {
                                b.number = number;
                                b.observed_value = Some(value);
                                b.value = Some(value);
                            }
                            _ => (),
                        }
                        b.receipt = Some(c.receipt_ms);
                        b.provider_identity = envelope.provider_identity.clone();
                        b.provider_time = envelope.provider_time_utc.clone();
                    }
                    if let Some(until) = u.valid_until_ms {
                        b.valid_until = if b.caps.current.is_none() { until } else { 0 };
                        if until <= c.at_ms {
                            b.initialized = false;
                            b.value = None;
                            b.coverage = None;
                        }
                    }
                    if b.initialized {
                        b.reason = None;
                    }
                }
                for fact in &effect.batch.coverage {
                    let b = self
                        .bindings
                        .iter_mut()
                        .find(|b| b.descriptor.binding_id == fact.binding_id)
                        .unwrap();
                    b.coverage = Some(match b.coverage {
                        Some((start, end)) if fact.start_ms <= end && fact.end_ms >= start => {
                            (start.min(fact.start_ms), end.max(fact.end_ms))
                        }
                        Some((start, end)) if fact.end_ms < start => (start, end),
                        _ => (fact.start_ms, fact.end_ms),
                    });
                }
                if self
                    .bindings
                    .iter()
                    .any(|b| b.stream == stream && b.initialized)
                {
                    self.streams[stream.index()].1 = Subscription::Initialized;
                }
                self.accepted(&mut effect, c.at_ms, &ids, previous_ready)?;
                Ok(effect)
            }
            SourceEvent::Connection {
                provider,
                token,
                state,
            } => {
                self.check_token(
                    match provider {
                        Provider::Kalshi => Stream::Book,
                        Provider::Coinbase => Stream::Coinbase,
                        Provider::Kraken => Stream::Kraken,
                    },
                    token,
                )?;
                if state != Connection::Open {
                    let effect = self.invalidate(
                        Scope::Provider(provider),
                        if state == Connection::AccessRefused {
                            Unavailable::Access
                        } else {
                            Unavailable::Disconnected
                        },
                        c.at_ms,
                        None,
                    )?;
                    self.connections[provider as usize] = state;
                    Ok(effect)
                } else {
                    self.connections[provider as usize] = state;
                    let mut effect = self.effect(c, vec![], vec![], &[])?;
                    self.accepted(&mut effect, c.at_ms, &[], previous_ready)?;
                    Ok(effect)
                }
            }
            SourceEvent::Subscription {
                stream,
                token,
                state,
            } => {
                self.check_token(stream, token)?;
                if !matches!(state, Subscription::Requested | Subscription::Acknowledged) {
                    return Err(failure("invalid_request"));
                }
                self.streams[stream.index()].1 = state;
                let mut effect = self.effect(c, vec![], vec![], &[])?;
                self.accepted(&mut effect, c.at_ms, &[], previous_ready)?;
                Ok(effect)
            }
            SourceEvent::RegisterMarket {
                token,
                descriptor,
                bindings,
                capabilities,
            } => {
                self.check_token(Stream::Metadata, token)?;
                if self.markets.len() == 2
                    || bindings.capacity() > BINDINGS
                    || capabilities.capacity() > BINDINGS
                {
                    return Err(failure("resource_limit"));
                }
                if [
                    &descriptor.series_id,
                    &descriptor.market_id,
                    &descriptor.contract_id,
                ]
                .iter()
                .any(|text| text.capacity() > 128)
                    || bindings.iter().any(|b| {
                        b.input_id.capacity() > 64
                            || match &b.scope {
                                BindingScope::Underlying => false,
                                BindingScope::Market { market_id } => market_id.capacity() > 128,
                                BindingScope::Contract {
                                    market_id,
                                    contract_id,
                                    ..
                                } => market_id.capacity() > 128 || contract_id.capacity() > 128,
                            }
                    })
                {
                    return Err(failure("resource_limit"));
                }
                self.validate_market(&descriptor)?;
                self.validate_bindings(&bindings, &capabilities, Some(&descriptor))?;
                let mut effect = self.effect(
                    c,
                    vec![vec![EvidenceChange::RegisterMarket {
                        descriptor: descriptor.clone(),
                        bindings: bindings.clone(),
                    }]],
                    vec![],
                    &[],
                )?;
                if self.series_id.is_none() {
                    self.series_id = Some(descriptor.series_id.clone());
                }
                self.markets.push(Market {
                    descriptor,
                    verified: false,
                    entered: false,
                    ready_since: None,
                });
                self.add_bindings(&bindings, &capabilities);
                let ids: Vec<_> = bindings.iter().map(|b| b.binding_id).collect();
                self.accepted(&mut effect, c.at_ms, &ids, previous_ready)?;
                Ok(effect)
            }
            SourceEvent::ReplaceMarket {
                token,
                expected,
                replacement,
            } => {
                self.check_token(Stream::Metadata, token)?;
                for text in [
                    &expected.series_id,
                    &expected.market_id,
                    &expected.contract_id,
                ] {
                    if text.len() > 128 || text.capacity() > 128 {
                        return Err(failure("resource_limit"));
                    }
                }
                let index = self
                    .markets
                    .iter()
                    .position(|m| m.descriptor == expected)
                    .ok_or_else(|| failure("identity_mismatch"))?;
                if [
                    &replacement.series_id,
                    &replacement.market_id,
                    &replacement.contract_id,
                ]
                .iter()
                .any(|s| s.len() > 128 || s.capacity() > 128)
                {
                    self.invalidate(Scope::Market, Unavailable::MetadataChanged, c.at_ms, None)?;
                    self.revocation_required |= (1 << Stream::Book.index())
                        | (1 << Stream::Trades.index())
                        | (1 << Stream::Metadata.index());
                    return Err(failure("resource_limit"));
                }
                if expected == replacement {
                    let mut effect = self.effect(c, vec![], vec![], &[])?;
                    self.accepted(&mut effect, c.at_ms, &[], previous_ready)?;
                    return Ok(effect);
                }
                if self.markets[index].entered {
                    return self.stop(c.at_ms);
                }
                let mut effect =
                    self.invalidate(Scope::Market, Unavailable::MetadataChanged, c.at_ms, None)?;
                let valid = expected.series_id == replacement.series_id
                    && expected.market_id == replacement.market_id
                    && expected.contract_id == replacement.contract_id
                    && strategy::validate_market_descriptor(&replacement, &expected.series_id)
                        .is_ok()
                    && !self.markets.iter().enumerate().any(|(i, m)| {
                        i != index
                            && m.descriptor.open_ms < replacement.close_ms
                            && replacement.open_ms < m.descriptor.close_ms
                    });
                if !valid {
                    // Error has no effect: the consumer must deliver the revocation explicitly.
                    self.revocation_required |= (1 << Stream::Book.index())
                        | (1 << Stream::Trades.index())
                        | (1 << Stream::Metadata.index());
                    return Err(failure("identity_mismatch"));
                }
                self.markets[index].descriptor = replacement;
                self.current = self
                    .markets
                    .iter()
                    .find(|m| {
                        m.verified
                            && i128::from(m.descriptor.open_ms) <= i128::from(c.at_ms)
                            && i128::from(c.at_ms) < i128::from(m.descriptor.close_ms)
                    })
                    .map(|m| m.descriptor.market_id.clone());
                effect.batch.current_admission = self.admission(c.at_ms);
                Ok(effect)
            }
            SourceEvent::Metadata {
                token,
                market_id,
                verified,
            } => {
                self.check_token(Stream::Metadata, token)?;
                if market_id.capacity() > 128 {
                    return Err(failure("resource_limit"));
                }
                let index = self
                    .markets
                    .iter()
                    .position(|m| m.descriptor.market_id == market_id)
                    .ok_or_else(|| failure("identity_mismatch"))?;
                if !verified {
                    self.markets[index].verified = false;
                    return self.invalidate(
                        Scope::Market,
                        Unavailable::MetadataChanged,
                        c.at_ms,
                        None,
                    );
                }
                let next = self
                    .market_revision
                    .checked_add(1)
                    .ok_or_else(|| failure("resource_limit"))?;
                self.markets[index].verified = true;
                self.market_revision = next;
                let market = &self.markets[index].descriptor;
                if i128::from(market.open_ms) <= i128::from(c.at_ms)
                    && i128::from(c.at_ms) < i128::from(market.close_ms)
                {
                    self.current = Some(market_id);
                }
                let mut effect = self.effect(c, vec![], vec![], &[])?;
                self.accepted(&mut effect, c.at_ms, &[], previous_ready)?;
                Ok(effect)
            }
            SourceEvent::Boundary { market_id } => {
                if market_id.capacity() > 128 {
                    return Err(failure("resource_limit"));
                }
                let index = self
                    .markets
                    .iter()
                    .position(|m| m.descriptor.market_id == market_id)
                    .ok_or_else(|| failure("identity_mismatch"))?;
                let market = &self.markets[index];
                if i128::from(c.at_ms) < i128::from(market.descriptor.open_ms) {
                    return Err(failure("invalid_order"));
                }
                let ready = self.preparation(index, c.at_ms, &[]) == Preparation::Ready;
                let exact =
                    i128::from(c.at_ms) == i128::from(self.markets[index].descriptor.open_ms);
                let enter = ready
                    && exact
                    && !self.markets[index].entered
                    && (self.initial_boundary_emitted
                        || self.markets[index]
                            .ready_since
                            .is_some_and(|at| at < c.at_ms));
                let changes = if enter {
                    vec![EvidenceChange::MarketBoundary {
                        market_id: market_id.clone(),
                    }]
                } else {
                    vec![]
                };
                let mut effect = self.effect(c, vec![changes], vec![], &[])?;
                self.initial_boundary_emitted |= enter;
                self.markets[index].entered |= enter;
                if ready && self.current.as_deref() != Some(&market_id) {
                    self.market_revision = self
                        .market_revision
                        .checked_add(1)
                        .ok_or_else(|| failure("resource_limit"))?;
                    self.current = Some(market_id.clone());
                }
                self.accepted(&mut effect, c.at_ms, &[], previous_ready)?;
                self.retire_expired(c.at_ms)?;
                Ok(effect)
            }
            SourceEvent::Advance => {
                let mut effect = self.effect(c, vec![], vec![], &[])?;
                self.accepted(&mut effect, c.at_ms, &[], previous_ready)?;
                self.retire_expired(c.at_ms)?;
                Ok(effect)
            }
        }
    }
    fn retire_expired(&mut self, at_ms: u64) -> Result<(), SourceFailure> {
        let old_len = self.markets.len();
        self.markets
            .retain(|m| i128::from(m.descriptor.close_ms) > i128::from(at_ms));
        if old_len != self.markets.len() {
            self.market_revision = self
                .market_revision
                .checked_add(1)
                .ok_or_else(|| failure("resource_limit"))?;
            self.bindings.retain(|b| {
                market_id_of(&b.descriptor.scope)
                    .is_none_or(|id| self.markets.iter().any(|m| m.descriptor.market_id == id))
            });
            if self
                .current
                .as_ref()
                .is_some_and(|id| !self.markets.iter().any(|m| m.descriptor.market_id == *id))
            {
                self.current = None;
            }
        }
        Ok(())
    }
    pub fn invalidate(
        &mut self,
        scope: Scope,
        reason: Unavailable,
        at_ms: u64,
        known_gap: Option<GapInterval>,
    ) -> Result<SourceEffect, SourceFailure> {
        if self.stopped {
            return Err(failure("stopped"));
        }
        if at_ms < self.last_at {
            return Err(failure("invalid_order"));
        }
        if let Some(gap) = known_gap
            && (gap.start_ms >= gap.end_ms || gap.end_ms > at_ms)
        {
            return Err(failure("invalid_request"));
        }
        let previous_ready = self.readiness(at_ms, &[]);
        let scope = if scope == Scope::Stream(Stream::Metadata) {
            Scope::Market
        } else {
            scope
        };
        let selected = |stream: Stream| match scope {
            Scope::Stream(s) => s == stream,
            Scope::Source(s) => s == stream.source(),
            Scope::Provider(p) => p == stream.provider(),
            Scope::Market => matches!(stream, Stream::Book | Stream::Trades | Stream::Metadata),
            Scope::All => true,
        };
        if selected(Stream::Metadata) {
            for market in &mut self.markets {
                market.verified = false;
                market.ready_since = None;
            }
            let Some(next) = self.market_revision.checked_add(1) else {
                self.stopped = true;
                return Err(failure("resource_limit"));
            };
            self.market_revision = next;
        }
        for stream in STREAMS {
            if selected(stream) {
                let Some(next) = self.streams[stream.index()].0.checked_add(1) else {
                    self.stopped = true;
                    return Err(failure("resource_limit"));
                };
                self.streams[stream.index()] = (next, Subscription::Rebuilding);
            }
        }
        let mut ids = Vec::new();
        let mut validity = Vec::new();
        let mut gaps = Vec::new();
        for b in &mut self.bindings {
            if !selected(b.stream) {
                continue;
            }
            let Some(next) = b.generation.checked_add(1) else {
                self.stopped = true;
                return Err(failure("resource_limit"));
            };
            b.generation = next;
            b.initialized = false;
            b.valid_until = 0;
            b.coverage = None;
            b.recovery_at = at_ms;
            b.reason = Some(reason);
            ids.push(b.descriptor.binding_id);
            validity.push(EvidenceChange::Validity {
                binding_id: b.descriptor.binding_id,
                valid_until_ms: 0,
                reason: Some(reason.code().into()),
            });
            if let Some(gap) = known_gap {
                gaps.push(EvidenceChange::Gap {
                    binding_id: b.descriptor.binding_id,
                    start_ms: gap.start_ms,
                    end_ms: gap.end_ms,
                    reason: reason.code().into(),
                });
            }
        }
        let c = Clock {
            at_ms,
            receipt_ms: at_ms,
            received_through_ms: at_ms,
            completed_frontier_ms: at_ms,
        };
        let mut effect = self.effect(c, vec![validity, gaps], vec![], &ids)?;
        self.accepted(&mut effect, at_ms, &ids, previous_ready)?;
        for stream in STREAMS {
            if selected(stream) {
                self.revocation_required &= !(1 << stream.index());
            }
        }
        Ok(effect)
    }
    pub fn stop(&mut self, at_ms: u64) -> Result<SourceEffect, SourceFailure> {
        if self.stopped {
            return self.effect(
                Clock {
                    at_ms: self.last_at,
                    receipt_ms: self.last_at,
                    received_through_ms: self.last_at,
                    completed_frontier_ms: self.last_at,
                },
                vec![],
                vec![],
                &[],
            );
        }
        let result = self.invalidate(Scope::All, Unavailable::Stopped, at_ms, None);
        self.stopped = true;
        result.map(|mut effect| {
            effect.batch.current_admission = Admission::Blocked("stopped");
            effect
        })
    }
    pub fn stamp(&self, ids: &[u32]) -> Result<EvidenceStamp, SourceFailure> {
        if ids.len() > BINDINGS {
            return Err(failure("resource_limit"));
        }
        let mut entries = Vec::new();
        for id in ids {
            let b = self
                .bindings
                .iter()
                .find(|b| b.descriptor.binding_id == *id)
                .ok_or_else(|| failure("identity_mismatch"))?;
            if entries.iter().any(|e: &StampEntry| e.id == *id) {
                return Err(failure("invalid_request"));
            }
            entries.push(StampEntry {
                id: *id,
                incarnation: b.incarnation,
                generation: b.generation,
                stream_generation: self.streams[b.stream.index()].0,
            });
        }
        Ok(EvidenceStamp {
            owner: self.owner,
            market_revision: if ids.iter().any(|id| {
                self.bindings.iter().any(|b| {
                    b.descriptor.binding_id == *id && market_id_of(&b.descriptor.scope).is_some()
                })
            }) {
                self.market_revision
            } else {
                0
            },
            entries,
        })
    }
    pub fn is_current(&self, stamp: &EvidenceStamp) -> bool {
        if self.stopped || stamp.owner != self.owner {
            return false;
        }
        let ids: Vec<_> = stamp.entries.iter().map(|e| e.id).collect();
        self.stamp(&ids).is_ok_and(|live| live == *stamp)
    }
    fn usable(&self, b: &Bound, now: u64) -> bool {
        !self.stopped
            && b.initialized
            && b.value.is_some()
            && b.caps.current.is_none()
            && b.valid_until > now
            && self.connections[b.stream.provider() as usize] == Connection::Open
            && market_id_of(&b.descriptor.scope).is_none_or(|id| {
                self.markets.iter().any(|m| {
                    m.descriptor.market_id == id
                        && i128::from(m.descriptor.close_ms) > i128::from(now)
                })
            })
    }
    fn history_ready(&self, b: &Bound, now: u64, coverage: &[CoverageFact]) -> bool {
        let required = self.requirements[b.requirement].startup_ms;
        required == 0
            || (b.caps.history.is_none()
                && b.coverage.is_some_and(|(mut start, mut end)| {
                    for fact in coverage
                        .iter()
                        .filter(|f| f.binding_id == b.descriptor.binding_id)
                    {
                        if fact.start_ms > end || fact.end_ms < start {
                            return false;
                        }
                        start = start.min(fact.start_ms);
                        end = end.max(fact.end_ms);
                    }
                    end >= now && now.saturating_sub(start) >= required
                }))
    }
    fn preparation(&self, index: usize, now: u64, coverage: &[CoverageFact]) -> Preparation {
        let m = &self.markets[index];
        if !m.verified {
            return Preparation::Identified;
        }
        for (r, requirement) in self.requirements.iter().enumerate() {
            let matching: Vec<_> = self
                .bindings
                .iter()
                .filter(|b| {
                    b.requirement == r
                        && market_id_of(&b.descriptor.scope)
                            .is_none_or(|id| id == m.descriptor.market_id)
                })
                .collect();
            if matching.is_empty()
                || (requirement.side == Some(SourceSide::Selected) && matching.len() != 2)
                || matching
                    .iter()
                    .any(|b| !self.usable(b, now) || !self.history_ready(b, now, coverage))
            {
                return Preparation::Collecting;
            }
        }
        Preparation::Ready
    }
    /// Retained market identities in snapshot order, without constructing a view.
    pub fn market_ids(&self) -> impl Iterator<Item = &str> {
        self.markets.iter().map(|m| m.descriptor.market_id.as_str())
    }

    pub fn snapshot(&self, now: u64) -> Result<SourceSnapshot, SourceFailure> {
        if now < self.last_at {
            return Err(failure("invalid_request"));
        }
        // Bound all strings by registration/identity caps; this also charges cloned records.
        let estimate = 2048
            + self
                .markets
                .iter()
                .map(|m| {
                    512 + 2
                        * (m.descriptor.series_id.len()
                            + m.descriptor.market_id.len()
                            + m.descriptor.contract_id.len())
                })
                .sum::<usize>()
            + self
                .bindings
                .iter()
                .map(|b| {
                    let r = &self.requirements[b.requirement];
                    let scope = match &b.descriptor.scope {
                        BindingScope::Underlying => 0,
                        BindingScope::Market { market_id } => market_id.len(),
                        BindingScope::Contract {
                            market_id,
                            contract_id,
                            ..
                        } => market_id.len() + contract_id.len(),
                    };
                    1024 + 2
                        * (b.descriptor.input_id.len()
                            + scope
                            + r.input_id.len()
                            + r.instrument.len()
                            + r.field.len()
                            + b.provider_identity.as_ref().map_or(0, String::len)
                            + b.provider_time.as_ref().map_or(0, String::len))
                })
                .sum::<usize>();
        if estimate > VIEW_BYTES {
            return Err(failure("resource_limit"));
        }
        let sources = [
            Source::Kalshi,
            Source::Brti,
            Source::Coinbase,
            Source::Kraken,
        ]
        .into_iter()
        .map(|s| {
            (
                s,
                self.connections[match s {
                    Source::Kalshi | Source::Brti => 0,
                    Source::Coinbase => 1,
                    Source::Kraken => 2,
                }],
            )
        })
        .collect();
        Ok(SourceSnapshot {
            owner: self.owner,
            revision: self.revision,
            at_ms: now,
            stopped: self.stopped,
            current_market: self.current.clone(),
            sources,
            streams: STREAMS
                .into_iter()
                .map(|s| (s, self.streams[s.index()].1, self.token(s)))
                .collect(),
            markets: self
                .markets
                .iter()
                .enumerate()
                .map(|(i, m)| MarketView {
                    descriptor: m.descriptor.clone(),
                    verified: m.verified,
                    preparation: self.preparation(i, now, &[]),
                })
                .collect(),
            bindings: self
                .bindings
                .iter()
                .map(|b| BindingView {
                    binding: b.descriptor.clone(),
                    requirement: self.requirements[b.requirement].clone(),
                    capabilities: b.caps,
                    value: b.value,
                    provider_identity: b.provider_identity.clone(),
                    provider_time_utc: b.provider_time.clone(),
                    age_ms: b.receipt.map(|at| now - at),
                    valid_until_ms: b.valid_until,
                    usable: !self.stopped && self.usable(b, now),
                    history_ready: self.history_ready(b, now, &[]),
                    covered_through_ms: b.coverage.map(|(_, end)| end),
                    reason: b.reason.or(if b.receipt.is_none() {
                        Some(Unavailable::Awaiting)
                    } else {
                        b.caps.current.or(if b.valid_until <= now {
                            Some(Unavailable::Freshness)
                        } else {
                            None
                        })
                    }),
                    initialized: b.initialized,
                    generation: b.generation,
                })
                .collect(),
        })
    }
}
fn market_id_of(scope: &BindingScope) -> Option<&str> {
    match scope {
        BindingScope::Underlying => None,
        BindingScope::Market { market_id } | BindingScope::Contract { market_id, .. } => {
            Some(market_id)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn checked_source_counters_fail_closed_and_fixed_records_have_bounded_sizes() {
        let strategy=ValidatedStrategy::parse(r#"{"format":2,"name":"Counters","inputs":[{"id":"p","source":"coinbase","instrument":"btc_usd","field":"last_trade","unit":"usd_per_btc"}],"purchase_options":[{"id":"buy","side":{"type":"fixed","side":"yes"},"when":{"type":"compare","left":{"reference":"p"},"comparison":"greater_than","right":{"value":"1","unit":"usd_per_btc"}}}]}"#).unwrap();
        for counter in 0..3 {
            let mut state = SourceState::new(&strategy, &[], &[], &[]).unwrap();
            match counter {
                0 => state.revision = u64::MAX,
                1 => state.transaction = u64::MAX,
                _ => state.incarnation = u64::MAX,
            };
            let clock = Clock {
                at_ms: 0,
                receipt_ms: 0,
                received_through_ms: 0,
                completed_frontier_ms: 0,
            };
            assert_eq!(
                state
                    .apply(Envelope {
                        clock,
                        provider_identity: None,
                        provider_time_utc: None,
                        event: SourceEvent::Advance
                    })
                    .unwrap_err()
                    .code,
                "resource_limit"
            );
            assert!(state.stopped);
        }
        let mut state = SourceState::new(&strategy, &[], &[], &[]).unwrap();
        state.streams[Stream::Book.index()].0 = u64::MAX;
        assert_eq!(
            state
                .invalidate(Scope::Stream(Stream::Book), Unavailable::Gap, 0, None)
                .unwrap_err()
                .code,
            "resource_limit"
        );
        assert!(state.stopped);
        let mut state = SourceState::new(&strategy, &[], &[], &[]).unwrap();
        let stamp = state.stamp(&[]).unwrap();
        state.market_revision = u64::MAX;
        assert_eq!(
            state
                .invalidate(
                    Scope::Stream(Stream::Metadata),
                    Unavailable::MetadataChanged,
                    0,
                    None
                )
                .unwrap_err()
                .code,
            "resource_limit"
        );
        assert!(state.stopped && !state.is_current(&stamp));
        eprintln!(
            "source_record_bytes: owner={} binding={} view={} requirement={} stamp_entry={} market={}",
            std::mem::size_of::<SourceState>(),
            std::mem::size_of::<Bound>(),
            std::mem::size_of::<BindingView>(),
            std::mem::size_of::<SourceRequirement>(),
            std::mem::size_of::<StampEntry>(),
            std::mem::size_of::<Market>()
        );
    }
}
