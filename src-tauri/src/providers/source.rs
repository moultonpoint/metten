//! The sole native source writer and its owned provider service; no financial run.
use super::*;
use std::collections::BTreeMap;

pub(super) const SOURCE_STREAMS: [metten_engine::sources::Stream; 6] = [
    metten_engine::sources::Stream::Book,
    metten_engine::sources::Stream::Trades,
    metten_engine::sources::Stream::Brti,
    metten_engine::sources::Stream::Metadata,
    metten_engine::sources::Stream::Coinbase,
    metten_engine::sources::Stream::Kraken,
];
const MARKET_STREAMS: [metten_engine::sources::Stream; 3] = [
    metten_engine::sources::Stream::Book,
    metten_engine::sources::Stream::Trades,
    metten_engine::sources::Stream::Metadata,
];
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct ScheduleLoss {
    pub epoch: Option<i64>,
    pub reason: metten_engine::sources::Unavailable,
    pub terminal: bool,
}
/// Runs only on the existing owned native worker. It owns no financial run.
#[derive(Clone, Copy)]
struct DesiredSources {
    enabled: [bool; 4],
    revocations: [u64; 4],
}
pub struct SourceWorker {
    pub(super) state: metten_engine::sources::SourceState,
    kalshi: kalshi::Kalshi,
    coinbase: coinbase::Coinbase,
    kraken: kraken::Kraken,
    pub(super) memory: memory::Memory,
    tickers: Vec<String>,
    requirements: Vec<metten_engine::strategy::SourceRequirement>,
    details: [Option<wire::Details>; 6],
    market_details: BTreeMap<String, [Option<wire::Details>; 2]>,
    schedule: watch::Sender<Option<ScheduleLoss>>,
    worker_signal: Option<watch::Sender<bool>>,
    number: u64,
    at: u64,
    epoch: Instant,
    epoch_utc_ms: i64,
    links: [Option<(Fence, u64)>; 3],
    pub(super) controls: [Option<mpsc::Sender<Control>>; 3],
    command_id: u32,
    gates: [Arc<AtomicU64>; 6],
    book_sid: Option<u32>,
    trade_sid: Option<u32>,
    progress: Option<watch::Sender<transport::Progress>>,
    blocked: [bool; 6],
    index_requested: bool,
    enabled: [bool; 4],
    failed_links: [Option<Fence>; 3],
    seen_revocations: [u64; 4],
    sids: [Option<u32>; 6],
    #[cfg(test)]
    pub(super) lease: Option<u64>,
    _base_charge: Arc<tokio::sync::OwnedSemaphorePermit>,
}
impl SourceWorker {
    pub fn new(
        state: metten_engine::sources::SourceState,
        memory: memory::Memory,
    ) -> Result<Self, Failure> {
        let base_charge = memory.reserve(64 * 1024)?;
        let _initial_view_charge = memory.reserve(256 * 1024)?;
        let view = state.snapshot(0).map_err(|_| Failure::InvalidRequest)?;
        let requirements = state.requirements().to_vec();
        let tickers = view
            .markets
            .iter()
            .map(|m| m.descriptor.market_id.clone())
            .collect();
        Ok(Self {
            state,
            details: std::array::from_fn(|_| None),
            market_details: BTreeMap::new(),
            schedule: watch::channel(None).0,
            _base_charge: Arc::new(base_charge),
            worker_signal: None,
            kalshi: kalshi::Kalshi::new(memory.clone()),
            coinbase: coinbase::Coinbase::default(),
            kraken: kraken::Kraken::default(),
            memory,
            tickers,
            requirements,
            number: 0,
            at: 0,
            epoch: Instant::now(),
            epoch_utc_ms: utc_now_ms()?,
            links: std::array::from_fn(|_| None),
            failed_links: std::array::from_fn(|_| None),
            controls: std::array::from_fn(|_| None),
            command_id: 100,
            gates: std::array::from_fn(|_| Arc::new(AtomicU64::new(1))),
            book_sid: None,
            trade_sid: None,
            progress: None,
            blocked: [false; 6],
            index_requested: false,
            enabled: [true; 4],
            seen_revocations: [0; 4],
            sids: [None; 6],
            #[cfg(test)]
            lease: None,
        })
    }
    pub fn tokens(&self) -> [metten_engine::sources::StreamToken; 6] {
        SOURCE_STREAMS.map(|s| self.state.token(s))
    }
    pub fn snapshot(&self, at: u64) -> Result<SourceView, Failure> {
        if self.schedule.borrow().is_some() {
            return Err(Failure::Stopped);
        }
        if self
            .gates
            .iter()
            .any(|g| g.load(Ordering::SeqCst) == u64::MAX)
        {
            return Err(Failure::Capacity);
        }
        let charge = self.memory.reserve(256 * 1024)?;
        Ok(SourceView {
            requirements: self.requirements.clone(),
            blocked: self.blocked,
            enabled: self.enabled,
            failures: std::array::from_fn(|i| {
                self.links[i]
                    .as_ref()
                    .map(|(f, _)| f)
                    .or(self.failed_links[i].as_ref())
                    .and_then(|f| f.cause.lock().ok().and_then(|cause| *cause))
            }),
            revocations: self.seen_revocations,
            details: self.details.clone(),
            market_details: self.market_details.clone(),
            schedule: self.schedule.clone(),
            epoch_utc_ms: self.epoch_utc_ms,
            worker_signal: self.worker_signal.clone(),
            snapshot: self
                .state
                .snapshot(at)
                .map_err(|_| Failure::InvalidRequest)?,
            fences: self.links.clone(),
            epochs: self.gates.each_ref().map(|g| g.load(Ordering::SeqCst)),
            gates: self.gates.clone(),
            _charge: charge,
        })
    }
    pub fn connection(
        &mut self,
        provider: Provider,
        token: metten_engine::sources::StreamToken,
        state: metten_engine::sources::Connection,
        at: u64,
    ) -> Result<metten_engine::sources::SourceEffect, Failure> {
        let stream = match provider {
            Provider::Kalshi => metten_engine::sources::Stream::Book,
            Provider::Coinbase => metten_engine::sources::Stream::Coinbase,
            Provider::Kraken => metten_engine::sources::Stream::Kraken,
        };
        if token != self.state.token(stream) {
            return Err(Failure::Stopped);
        }
        if state == metten_engine::sources::Connection::Open {
            self.failed_links[provider.index()] = None;
        } else {
            // Keep the existing fence's cause after active-link cleanup, not a second mutable reason.
            self.failed_links[provider.index()] = self.links[provider.index()]
                .as_ref()
                .filter(|(f, _)| f.cause.lock().is_ok_and(|cause| cause.is_some()))
                .map(|(f, _)| f.clone());
            for stream in provider_streams(provider) {
                self.sids[*stream as usize] = None;
            }
            revoke_gates(&self.gates, provider_streams(provider))?;
            if let Some((fence, _)) = &self.links[provider.index()] {
                fence.revoke();
            }
            self.links[provider.index()] = None;
            match provider {
                Provider::Kalshi => {
                    self.book_sid = None;
                    self.trade_sid = None;
                    self.progress = None;
                    self.index_requested = false;
                    self.kalshi.reconnect();
                }
                Provider::Coinbase => self.coinbase.reconnect(),
                Provider::Kraken => self.kraken.reconnect(),
            }
        }
        let provider = engine_provider(provider);
        let effect = self
            .state
            .apply(source_envelope(
                at,
                metten_engine::sources::SourceEvent::Connection {
                    provider,
                    token,
                    state,
                },
            ))
            .map_err(|_| Failure::Protocol)?;
        Ok(effect)
    }
    pub fn process(
        &mut self,
        item: Intake,
        at: u64,
        completed_frontier_ms: u64,
        mut consume: impl FnMut(metten_engine::sources::SourceEffect) -> Result<(), Failure>,
    ) -> Result<(), Failure> {
        use metten_engine::sources::{Scope, Unavailable};
        if !self.memory.owns(&item.bytes) {
            return Err(Failure::InvalidRequest);
        }
        let tokens = item.source_tokens.ok_or(Failure::InvalidRequest)?;
        if !item.fence.current(item.generation) {
            item.fence.reject();
            return Err(Failure::Stopped);
        }
        if at < self.at {
            return Err(Failure::InvalidRequest);
        }
        self.at = at;
        let expired = Instant::now().saturating_duration_since(item.received_at) > MAX_INTAKE_AGE;
        if item.provider == Provider::Kalshi
            && !expired
            && self.kalshi.retired_frame(item.bytes.as_slice())
        {
            item.fence.reject();
            return Err(Failure::Stopped);
        }
        let wire_stream = if item.provider == Provider::Kalshi && !expired {
            let disabled = std::array::from_fn(|i| {
                !self.enabled[if i == metten_engine::sources::Stream::Brti as usize {
                    1
                } else {
                    0
                }]
            });
            match self.kalshi.stream(item.bytes.as_slice(), &disabled) {
                Ok(stream) => stream,
                Err(error) => {
                    item.fence.fail(error);
                    revoke_gates(&self.gates, provider_streams(item.provider))?;
                    consume(
                        self.state
                            .invalidate(
                                Scope::Provider(engine_provider(item.provider)),
                                Unavailable::InvalidValue,
                                at,
                                None,
                            )
                            .map_err(|_| Failure::Protocol)?,
                    )?;
                    return Err(error);
                }
            }
        } else {
            None
        };
        if let Some(stream) = wire_stream
            && (self.blocked[stream as usize]
                || tokens[stream as usize] != self.state.token(stream))
        {
            self.kalshi.skip_obsolete(item.bytes.as_slice())?;
            if stream == metten_engine::sources::Stream::Book
                && !self.blocked[stream as usize]
                && self.enabled[0]
                && let (Some(progress), Some(sid)) = (&item.progress, self.book_sid)
            {
                let token = self.state.token(stream);
                let needs_retry = {
                    let p = progress.borrow();
                    p.tokens[stream as usize] == Some(token) && p.recovery[0].is_none()
                };
                if needs_retry && !self.tickers.is_empty() {
                    // A receive begun before recovery cannot certify its first returned frame; retry under the new receive identity.
                    self.command_id = self.command_id.checked_add(1).ok_or(Failure::Capacity)?;
                    progress.send_modify(|p| {
                        p.recover(Control::KalshiBookUpdate {
                            id: self.command_id,
                            sid,
                            action: BookAction::Snapshot,
                            markets: self.tickers.clone(),
                            token: Some(token),
                        })
                    });
                }
            }
            item.fence.reject();
            return Err(Failure::Stopped);
        }
        self.links[item.provider.index()] = Some((item.fence.clone(), item.generation));
        if item.provider == Provider::Kalshi {
            self.progress = item.progress.clone();
            if let Some(progress) = &self.progress {
                progress.send_modify(|p| {
                    for stream in SOURCE_STREAMS {
                        if self.blocked[stream as usize] {
                            p.disable(stream);
                        }
                    }
                });
            }
        }
        let _scratch = if expired {
            None
        } else {
            Some(match self.memory.reserve(4 * MAX_BYTES) {
                Ok(charge) => charge,
                Err(error) => {
                    item.fence.revoke();
                    consume(
                        self.state
                            .invalidate(
                                Scope::Provider(engine_provider(item.provider)),
                                Unavailable::Overflow,
                                at,
                                None,
                            )
                            .map_err(|_| Failure::Unavailable)?,
                    )?;
                    return Err(error);
                }
            })
        };
        let receipt = u64::try_from(
            item.received_at
                .saturating_duration_since(self.epoch)
                .as_millis(),
        )
        .map_err(|_| Failure::InvalidRequest)?;
        #[cfg(test)]
        let receipt = item.receipt_ms.unwrap_or(receipt);
        let clock = metten_engine::sources::Clock {
            at_ms: at,
            receipt_ms: receipt,
            received_through_ms: at,
            completed_frontier_ms,
        };
        let state = &mut self.state;
        let number = &mut self.number;
        #[cfg(test)]
        let lease = self.lease;
        #[cfg(not(test))]
        let lease = None;
        let outbound = self.controls[item.provider.index()].clone();
        let book_sid = &mut self.book_sid;
        let trade_sid = &mut self.trade_sid;
        let command_id = &mut self.command_id;
        let tickers = &self.tickers;
        let gates = &self.gates;
        let details = &mut self.details;
        let market_details = &mut self.market_details;
        let index_requested = &mut self.index_requested;
        let sids = &mut self.sids;
        let enabled = self.enabled;
        let blocked = &mut self.blocked;
        let schedule = self.schedule.clone();
        let mut emit = |event| {
            if !item.fence.current(item.generation) {
                return Err(Failure::Stopped);
            }
            if let wire::Event::Prices(p) = &event
                && MARKET_STREAMS.contains(&p.stream)
                && schedule.borrow().is_some()
            {
                return Ok(());
            }
            if let wire::Event::Gap(stream, sid) = event {
                if blocked[stream as usize] {
                    return Ok(());
                }
                let scopes = if stream == metten_engine::sources::Stream::Metadata {
                    &MARKET_STREAMS[..]
                } else {
                    std::slice::from_ref(&stream)
                };
                revoke_gates(gates, scopes)?;
                consume(
                    state
                        .invalidate(
                            metten_engine::sources::Scope::Stream(stream),
                            metten_engine::sources::Unavailable::Gap,
                            at,
                            None,
                        )
                        .map_err(|_| Failure::Protocol)?,
                )?;
                if stream == metten_engine::sources::Stream::Book {
                    let token = state.token(stream);
                    if let Some(progress) = &item.progress {
                        progress.send_modify(|p| p.desired_book = Some(token));
                    }
                    if let Some(outbound) = &outbound {
                        *command_id = command_id.checked_add(1).ok_or(Failure::Capacity)?;
                        let control = Control::KalshiBookUpdate {
                            id: *command_id,
                            sid,
                            action: BookAction::Snapshot,
                            markets: tickers.clone(),
                            token: Some(state.token(stream)),
                        };
                        if let Some(progress) = &item.progress {
                            progress.send_modify(|p| p.recover(control.clone()));
                        } else {
                            outbound.try_send(control).map_err(|_| Failure::Capacity)?;
                        }
                    }
                }
                if let Some(outbound) = &outbound {
                    *command_id = command_id.checked_add(1).ok_or(Failure::Capacity)?;
                    let token = state.token(stream);
                    let control = match stream {
                        metten_engine::sources::Stream::Brti => Some(Control::KalshiIndexes {
                            id: *command_id,
                            sid,
                            token: Some(token),
                        }),
                        metten_engine::sources::Stream::Trades if !tickers.is_empty() => {
                            Some(Control::KalshiTradeUpdate {
                                id: *command_id,
                                sid,
                                action: BookAction::Add,
                                markets: tickers.clone(),
                                token,
                            })
                        }
                        _ => None,
                    };
                    if let Some(control) = control {
                        if let Some(progress) = &item.progress {
                            progress.send_modify(|p| p.recover(control.clone()));
                        } else {
                            outbound.try_send(control).map_err(|_| Failure::Capacity)?;
                        }
                    }
                }
                return Ok(());
            }
            if let wire::Event::SubscriptionEnded(stream, sid)
            | wire::Event::Unsubscribed(stream, sid) = event
            {
                if matches!(event, wire::Event::Unsubscribed(..))
                    && let Some(progress) = &item.progress
                {
                    progress.send_modify(|p| p.unsubscribed(sid));
                }
                let ended = matches!(event, wire::Event::SubscriptionEnded(..))
                    || sids[stream as usize] == Some(sid);
                if sids[stream as usize] == Some(sid) {
                    sids[stream as usize] = None;
                }
                if stream == metten_engine::sources::Stream::Book && *book_sid == Some(sid) {
                    *book_sid = None;
                }
                if stream == metten_engine::sources::Stream::Trades && *trade_sid == Some(sid) {
                    *trade_sid = None;
                }
                if ended && !blocked[stream as usize] {
                    let scopes = if stream == metten_engine::sources::Stream::Metadata {
                        &MARKET_STREAMS[..]
                    } else {
                        std::slice::from_ref(&stream)
                    };
                    revoke_gates(gates, scopes)?;
                    for scope in scopes {
                        blocked[*scope as usize] = true;
                    }
                    consume(
                        state
                            .invalidate(
                                metten_engine::sources::Scope::Stream(stream),
                                Unavailable::Disconnected,
                                at,
                                None,
                            )
                            .map_err(|_| Failure::Protocol)?,
                    )?;
                    if let Some(progress) = &item.progress {
                        progress.send_modify(|p| {
                            for scope in scopes {
                                p.disable(*scope);
                            }
                        });
                    }
                } else if enabled[if stream == metten_engine::sources::Stream::Brti {
                    1
                } else {
                    0
                }] && !blocked[stream as usize]
                {
                    *command_id = command_id.checked_add(1).ok_or(Failure::Capacity)?;
                    if let Some(control) = subscription(stream, *command_id, tickers)
                        && let Some(outbound) = &outbound
                    {
                        outbound.try_send(control).map_err(|_| Failure::Capacity)?;
                    }
                }
                return Ok(());
            }
            if let wire::Event::ControlRejected(id, code) = event {
                let progress = item.progress.as_ref().ok_or(Failure::Protocol)?;
                if code == 7 {
                    let mut cleanup = false;
                    progress.send_modify(|p| cleanup = p.complete_unsubscribe(id));
                    if cleanup {
                        return Ok(());
                    }
                }
                let stream = { progress.borrow().requested_stream(id) }.ok_or(Failure::Protocol)?;
                if blocked[stream as usize] {
                    return Ok(());
                }
                let scopes = if stream == metten_engine::sources::Stream::Metadata {
                    &MARKET_STREAMS[..]
                } else {
                    std::slice::from_ref(&stream)
                };
                revoke_gates(gates, scopes)?;
                for scope in scopes {
                    blocked[*scope as usize] = true;
                }
                consume(
                    state
                        .invalidate(
                            metten_engine::sources::Scope::Stream(stream),
                            if code == 9 {
                                Unavailable::Access
                            } else {
                                Unavailable::InvalidValue
                            },
                            at,
                            None,
                        )
                        .map_err(|_| Failure::Protocol)?,
                )?;
                progress.send_modify(|p| {
                    for scope in scopes {
                        p.disable(*scope);
                    }
                });
                return Ok(());
            }
            if let wire::Event::ControlAcknowledged(id) = event {
                if let Some(progress) = &item.progress {
                    progress.send_modify(|p| p.acknowledge(id));
                }
                return Ok(());
            }
            if let wire::Event::Acknowledged(stream, sid) = &event {
                sids[*stream as usize] = Some(*sid);
                if blocked[*stream as usize] {
                    if !enabled[if *stream == metten_engine::sources::Stream::Brti {
                        1
                    } else {
                        0
                    }] && let Some(outbound) = &outbound
                    {
                        *command_id = command_id.checked_add(1).ok_or(Failure::Capacity)?;
                        outbound
                            .try_send(Control::KalshiUnsubscribe {
                                id: *command_id,
                                subscriptions: vec![*sid],
                            })
                            .map_err(|_| Failure::Capacity)?;
                    }
                    sids[*stream as usize] = None;
                    return Ok(());
                }
                match stream {
                    metten_engine::sources::Stream::Book => *book_sid = Some(*sid),
                    metten_engine::sources::Stream::Trades => *trade_sid = Some(*sid),
                    metten_engine::sources::Stream::Brti if !*index_requested => {
                        if let Some(outbound) = &outbound {
                            *command_id = command_id.checked_add(1).ok_or(Failure::Capacity)?;
                            outbound
                                .try_send(Control::KalshiIndexes {
                                    id: *command_id,
                                    sid: *sid,
                                    token: None,
                                })
                                .map_err(|_| Failure::Capacity)?;
                            *index_requested = true;
                        }
                    }
                    _ => (),
                }
            }
            match &event {
                wire::Event::Absent(stream) => {
                    revoke_gates(gates, std::slice::from_ref(stream))?;
                    if let Some(progress) = &item.progress {
                        progress.send_modify(|p| p.received(*stream, None));
                    }
                }
                wire::Event::MetadataChanged => revoke_gates(gates, &MARKET_STREAMS)?,
                wire::Event::Prices(p) if p.readings.iter().any(|r| r.value.is_none()) => {
                    revoke_gates(gates, std::slice::from_ref(&p.stream))?
                }
                _ => (),
            }
            let acknowledged = if let wire::Event::Acknowledged(stream, _) = &event {
                Some(*stream)
            } else {
                None
            };
            if let wire::Event::Prices(prices) = &event {
                if let Some(market) = &prices.market {
                    if !market_details.contains_key(market) && market_details.len() == 2 {
                        return Err(Failure::Capacity);
                    }
                    market_details
                        .entry(market.clone())
                        .or_insert_with(|| [None, None])
                        [usize::from(prices.stream == metten_engine::sources::Stream::Trades)] =
                        prices.details.clone();
                } else {
                    details[prices.stream as usize] = prices.details.clone();
                }
            }
            let received = if let wire::Event::Prices(prices) = &event {
                Some((prices.stream, prices.market.clone()))
            } else {
                None
            };
            apply_source_event(state, number, event, &tokens, clock, lease, &mut consume)?;
            if let Some(progress) = &item.progress {
                progress.send_modify(|p| {
                    if let Some(stream) = acknowledged {
                        p.acknowledged[stream as usize] = true;
                    }
                    if let Some((stream, market)) = &received {
                        p.received(*stream, market.as_deref());
                    }
                });
            }
            Ok(())
        };
        let result = if expired {
            Err(Failure::Deadline)
        } else {
            match item.provider {
                Provider::Kalshi => {
                    self.kalshi
                        .decode(item.bytes.as_slice(), &self.tickers, &mut emit)
                }
                Provider::Coinbase => self.coinbase.decode(item.bytes.as_slice(), &mut emit),
                Provider::Kraken => self.kraken.decode(item.bytes.as_slice(), &mut emit),
            }
        };
        for index in 0..2 {
            if !self.enabled[index] {
                for &stream in logical_streams(index) {
                    self.kalshi.retire(stream)?;
                }
            }
        }
        if self.blocked[metten_engine::sources::Stream::Book as usize] {
            self.kalshi.discard_book();
        }
        if let Err(error) = result {
            if let Some(stream) = wire_stream
                .filter(|_| !expired && matches!(error, Failure::Protocol | Failure::Capacity))
            {
                let streams = if stream == metten_engine::sources::Stream::Metadata {
                    &MARKET_STREAMS[..]
                } else {
                    std::slice::from_ref(&stream)
                };
                for affected in streams {
                    self.blocked[*affected as usize] = true;
                }
                revoke_gates(&self.gates, streams)?;
                consume(
                    self.state
                        .invalidate(
                            Scope::Stream(stream),
                            if error == Failure::Capacity {
                                Unavailable::Overflow
                            } else {
                                Unavailable::InvalidValue
                            },
                            at,
                            None,
                        )
                        .map_err(|_| Failure::Protocol)?,
                )?;
                if streams.contains(&metten_engine::sources::Stream::Book) {
                    self.kalshi.discard_book();
                }
                if let Some(progress) = &item.progress {
                    progress.send_modify(|p| {
                        for affected in streams {
                            p.disable(*affected);
                        }
                    });
                }
                if let (Some(sid), Some(outbound)) = (
                    self.kalshi.sid(stream),
                    &self.controls[item.provider.index()],
                ) {
                    self.command_id = self.command_id.checked_add(1).ok_or(Failure::Capacity)?;
                    outbound
                        .try_send(Control::KalshiUnsubscribe {
                            id: self.command_id,
                            subscriptions: vec![sid],
                        })
                        .map_err(|_| Failure::Capacity)?;
                }
                return Err(error);
            }
            item.fence.fail(error);
            revoke_gates(&self.gates, provider_streams(item.provider))?;
            let reason = match error {
                Failure::Capacity => Unavailable::Overflow,
                Failure::Deadline => Unavailable::OverAge,
                Failure::Transport => Unavailable::Gap,
                Failure::Denied => Unavailable::Access,
                Failure::Stopped => Unavailable::Stopped,
                _ => Unavailable::InvalidValue,
            };
            let effect = self
                .state
                .invalidate(
                    Scope::Provider(engine_provider(item.provider)),
                    reason,
                    at,
                    None,
                )
                .map_err(|_| Failure::Unavailable)?;
            consume(effect)?;
            return Err(error);
        }
        if !item.fence.current(item.generation) {
            consume(
                self.state
                    .invalidate(
                        Scope::Provider(engine_provider(item.provider)),
                        Unavailable::Disconnected,
                        at,
                        None,
                    )
                    .map_err(|_| Failure::Unavailable)?,
            )?;
            return Err(Failure::Stopped);
        }
        Ok(())
    }
}
pub struct SourceView {
    epoch_utc_ms: i64,
    blocked: [bool; 6],
    enabled: [bool; 4],
    failures: [Option<Failure>; 3],
    revocations: [u64; 4],
    snapshot: metten_engine::sources::SourceSnapshot,
    details: [Option<wire::Details>; 6],
    market_details: BTreeMap<String, [Option<wire::Details>; 2]>,
    schedule: watch::Sender<Option<ScheduleLoss>>,
    worker_signal: Option<watch::Sender<bool>>,
    requirements: Vec<metten_engine::strategy::SourceRequirement>,
    fences: [Option<(Fence, u64)>; 3],
    epochs: [u64; 6],
    gates: [Arc<AtomicU64>; 6],
    _charge: tokio::sync::OwnedSemaphorePermit,
}
pub fn source_name(source: metten_engine::strategy::Source) -> &'static str {
    use metten_engine::strategy::Source::*;
    match source {
        Kalshi => "kalshi",
        Brti => "brti",
        Coinbase => "coinbase",
        Kraken => "kraken",
    }
}
fn logical_streams(index: usize) -> &'static [metten_engine::sources::Stream] {
    use metten_engine::sources::Stream::*;
    match index {
        0 => &[Book, Trades, Metadata],
        1 => &[Brti],
        2 => &[Coinbase],
        _ => &[Kraken],
    }
}
impl SourceHandle {
    pub fn disable(&self, index: usize) -> Result<(), Failure> {
        if index >= 4 {
            return Err(Failure::InvalidRequest);
        }
        revoke_gates(&self.gates, logical_streams(index))?;
        let mut result = Ok(());
        self.desired.send_modify(|set| {
            set.enabled[index] = false;
            if let Some(next) = set.revocations[index].checked_add(1) {
                set.revocations[index] = next;
            } else {
                result = Err(Failure::Capacity);
            }
        });
        result
    }
    pub fn intent_revision(&self, index: usize) -> u64 {
        self.desired.borrow().revocations[index]
    }
    pub fn enable_if_current(&self, index: usize, expected: u64) -> Result<(), Failure> {
        if index >= 4 {
            return Err(Failure::InvalidRequest);
        }
        let mut current = false;
        self.desired.send_modify(|set| {
            if set.revocations[index] == expected {
                set.enabled[index] = true;
                current = true;
            }
        });
        if !current {
            return Err(Failure::Stopped);
        }
        self.wait_membership(index, true, expected)
    }
    pub fn enable(&self, index: usize, enabled: bool) -> Result<(), Failure> {
        if index >= 4 {
            return Err(Failure::InvalidRequest);
        }
        if enabled {
            self.enable_if_current(index, self.intent_revision(index))
        } else {
            self.disable(index)?;
            self.wait_membership(index, false, self.intent_revision(index))
        }
    }
    fn wait_membership(&self, index: usize, enabled: bool, expected: u64) -> Result<(), Failure> {
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        loop {
            if self.intent_revision(index) != expected {
                return Err(Failure::Stopped);
            }
            if self.latest.borrow().as_ref().is_some_and(|view| {
                view.enabled[index] == enabled && view.revocations[index] == expected
            }) {
                return Ok(());
            }
            if std::time::Instant::now() >= deadline {
                return Err(Failure::Deadline);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    pub fn show_connection(
        &self,
        provider: Provider,
        state: metten_engine::sources::Connection,
    ) -> Result<(), Failure> {
        let (reply, received) = tokio::sync::oneshot::channel();
        self.command
            .try_send(SourceCommand::Connection {
                provider,
                state,
                reply,
            })
            .map_err(|_| Failure::Capacity)?;
        tauri::async_runtime::block_on(async {
            tokio::time::timeout(Duration::from_secs(2), received)
                .await
                .map_err(|_| Failure::Deadline)?
                .map_err(|_| Failure::Stopped)?
        })
    }
    pub fn prepare_fixture_markets(&self) -> Result<(), Failure> {
        let (reply, received) = tokio::sync::oneshot::channel();
        self.command
            .try_send(SourceCommand::FixtureMarkets { reply })
            .map_err(|_| Failure::Capacity)?;
        tauri::async_runtime::block_on(async {
            tokio::time::timeout(Duration::from_secs(2), received)
                .await
                .map_err(|_| Failure::Deadline)?
                .map_err(|_| Failure::Stopped)?
        })
    }
    pub fn dispatch_fixture(&self, provider: Provider, payload: &[u8]) -> Result<(), Failure> {
        let fence = self.fixtures[provider.index()]
            .borrow()
            .clone()
            .ok_or(Failure::Stopped)?;
        if !fence.current(fence.token()) {
            return Err(Failure::Stopped);
        }
        let view = self.latest.borrow().clone().ok_or(Failure::Stopped)?;
        let snapshot = view.snapshot()?;
        let tokens = SOURCE_STREAMS.map(|stream| {
            snapshot
                .streams
                .iter()
                .find(|(s, _, _)| *s == stream)
                .unwrap()
                .2
        });
        let (reply, received) = tokio::sync::oneshot::channel();
        let mut item = Intake::new(provider, &fence, self.memory.copy(payload)?, Instant::now());
        item.source_tokens = Some(tokens);
        item.completion = Some(reply);
        admit(&self.intake, &fence, item)?;
        tauri::async_runtime::block_on(async {
            tokio::time::timeout(Duration::from_secs(2), received)
                .await
                .map_err(|_| Failure::Deadline)?
                .map_err(|_| Failure::Stopped)?
        })
    }
    pub fn enabled(&self) -> [bool; 4] {
        self.desired.borrow().enabled
    }
    pub fn memory(&self) -> &memory::Memory {
        &self.memory
    }
    pub fn elapsed_ms(&self) -> u64 {
        u64::try_from(self.epoch.elapsed().as_millis()).unwrap_or(u64::MAX)
    }
}
impl SourceWorker {
    fn observe_membership(&mut self, desired: DesiredSources, at: u64) -> Result<(), Failure> {
        for i in 0..4 {
            if self.seen_revocations[i] != desired.revocations[i] {
                let mut stopped = self.enabled;
                stopped[i] = false;
                self.membership(stopped, at)?;
                self.seen_revocations[i] = desired.revocations[i];
            }
        }
        self.membership(desired.enabled, at)
    }
    pub(super) fn membership(&mut self, desired: [bool; 4], at: u64) -> Result<(), Failure> {
        use metten_engine::sources::{Scope, Stream, Unavailable};
        use metten_engine::strategy::Source;
        for (index, enabled) in desired.into_iter().enumerate() {
            if enabled == self.enabled[index] {
                continue;
            }
            let streams = logical_streams(index);
            let source = [
                Source::Kalshi,
                Source::Brti,
                Source::Coinbase,
                Source::Kraken,
            ][index];
            revoke_gates(&self.gates, streams)?;
            let effect = self
                .state
                .invalidate(Scope::Source(source), Unavailable::Disconnected, at, None)
                .map_err(|_| Failure::Protocol)?;
            let _ = effect;
            self.enabled[index] = enabled;
            let provider = match index {
                0 | 1 => Provider::Kalshi,
                2 => Provider::Coinbase,
                _ => Provider::Kraken,
            };
            for &stream in streams {
                self.blocked[stream as usize] = !enabled;
                if let Some(progress) = &self.progress {
                    progress.send_modify(|p| {
                        p.tokens[stream as usize] = Some(self.state.token(stream));
                        if !enabled {
                            p.disable(stream);
                        }
                    });
                }
                if !self.links[provider.index()]
                    .as_ref()
                    .is_some_and(|(f, g)| f.current(*g))
                {
                    continue;
                }
                self.command_id = self.command_id.checked_add(1).ok_or(Failure::Capacity)?;
                let command = if !enabled {
                    self.sids[stream as usize]
                        .take()
                        .map(|sid| Control::KalshiUnsubscribe {
                            id: self.command_id,
                            subscriptions: vec![sid],
                        })
                } else if self.kalshi.retiring(stream) {
                    None
                } else {
                    subscription(stream, self.command_id, &self.tickers)
                };
                if let Some(command) = command {
                    self.controls[provider.index()]
                        .as_ref()
                        .ok_or(Failure::Stopped)?
                        .try_send(command)
                        .map_err(|_| Failure::Capacity)?;
                }
            }
            if !enabled && index < 2 {
                for &stream in streams {
                    self.kalshi.retire(stream)?;
                    if stream == Stream::Book {
                        self.book_sid = None;
                    }
                    if stream == Stream::Trades {
                        self.trade_sid = None;
                    }
                }
            }
            if index == 1 {
                self.index_requested = false;
            }
        }
        Ok(())
    }
}
fn subscription(
    stream: metten_engine::sources::Stream,
    id: u32,
    tickers: &[String],
) -> Option<Control> {
    use metten_engine::sources::Stream;
    let channel = match stream {
        Stream::Book => Channel::Book,
        Stream::Trades => Channel::Trade,
        Stream::Brti => Channel::Brti,
        Stream::Metadata => Channel::Lifecycle,
        _ => return None,
    };
    let scoped = matches!(stream, Stream::Book | Stream::Trades);
    if scoped && tickers.is_empty() {
        return None;
    }
    Some(Control::KalshiSubscribe {
        id,
        channel,
        markets: if scoped { tickers.to_vec() } else { vec![] },
    })
}
fn provider_streams(provider: Provider) -> &'static [metten_engine::sources::Stream] {
    use metten_engine::sources::Stream;
    match provider {
        Provider::Kalshi => &[Stream::Book, Stream::Trades, Stream::Brti, Stream::Metadata],
        Provider::Coinbase => &[Stream::Coinbase],
        Provider::Kraken => &[Stream::Kraken],
    }
}
fn revoke_gates(
    gates: &[Arc<AtomicU64>; 6],
    streams: &[metten_engine::sources::Stream],
) -> Result<(), Failure> {
    for stream in streams {
        gates[*stream as usize]
            .try_update(Ordering::SeqCst, Ordering::SeqCst, |g| g.checked_add(1))
            .map_err(|_| Failure::Capacity)?;
    }
    Ok(())
}
impl SourceView {
    fn schedule_loss(&self, mut loss: ScheduleLoss) {
        self.schedule.send_modify(|pending| {
            if let Some(old) = *pending {
                if old.terminal {
                    loss.terminal = true;
                    loss.reason = old.reason;
                }
                if loss.epoch.is_none() {
                    loss.epoch = old.epoch;
                }
            }
            *pending = Some(loss);
        });
    }
    fn stream_epoch_current(&self, stream: metten_engine::sources::Stream) -> bool {
        if MARKET_STREAMS.contains(&stream) && self.schedule.borrow().is_some() {
            return false;
        }
        if self.worker_signal.as_ref().is_some_and(|s| *s.borrow()) {
            return false;
        }
        let i = stream as usize;
        let live = self.gates[i].load(Ordering::SeqCst);
        live != u64::MAX && live == self.epochs[i]
    }
    pub fn stream_current(&self, stream: metten_engine::sources::Stream) -> bool {
        let provider = match stream {
            metten_engine::sources::Stream::Coinbase => Provider::Coinbase,
            metten_engine::sources::Stream::Kraken => Provider::Kraken,
            _ => Provider::Kalshi,
        };
        self.current(provider) && self.stream_epoch_current(stream)
    }
    /// Recheck permissions at use, including a stop after this view was constructed.
    pub fn wire_snapshot(&self, now_ms: u64) -> serde_json::Value {
        use metten_engine::{
            sources::*,
            strategy::{BindingScope, RuleUnit},
        };
        use serde_json::json;
        let sources: Vec<_> = self.snapshot.sources.iter().map(|(source, state)| {
            let i = *source as usize;
            json!({"source": source_name(*source), "enabled": self.enabled[i],
                "connection": if !self.enabled[i] || (*state==Connection::Open && !self.current(match i {0|1=>super::Provider::Kalshi,2=>super::Provider::Coinbase,_=>super::Provider::Kraken})) { "disconnected" } else { match state {
                    Connection::Disconnected => "disconnected", Connection::MissingConfiguration => "missing_credentials",
                    Connection::Connecting => "connecting", Connection::Open => "open", Connection::AccessRefused => "access_refused" } }})
        }).collect();
        let bindings: Vec<_> = self.snapshot.bindings.iter().map(|b| {
            let stream = match b.requirement.source { metten_engine::strategy::Source::Kalshi if b.requirement.instrument == "current_market" => Stream::Metadata,
                metten_engine::strategy::Source::Kalshi if b.requirement.field == "last_trade" => Stream::Trades,
                metten_engine::strategy::Source::Kalshi => Stream::Book, metten_engine::strategy::Source::Brti => Stream::Brti,
                metten_engine::strategy::Source::Coinbase => Stream::Coinbase, _ => Stream::Kraken };
            let enabled = self.enabled[b.requirement.source as usize];
            let permitted = enabled && self.stream_current(stream);
            let provider = match b.requirement.source as usize {0|1=>super::Provider::Kalshi,2=>super::Provider::Coinbase,_=>super::Provider::Kraken};
            let failure_reason = self.failures[provider.index()].map(|failure| match failure {
                Failure::Protocol => Unavailable::InvalidValue,
                Failure::Capacity => Unavailable::Overflow,
                Failure::Deadline => Unavailable::OverAge,
                Failure::Transport => Unavailable::Gap,
                Failure::Denied => Unavailable::Access,
                _ => Unavailable::Disconnected,
            });
            let (market, side) = match &b.binding.scope { BindingScope::Underlying => (None, None), BindingScope::Market { market_id } => (Some(market_id), None),
                BindingScope::Contract { market_id, side, .. } => (Some(market_id), Some(match side { metten_engine::strategy::Side::Yes => "yes", _ => "no" })) };
            let market_unexpired = market.is_none_or(|id| self.snapshot.markets.iter().any(|m|
                &m.descriptor.market_id == id && i128::from(now_ms) < i128::from(m.descriptor.close_ms)));
            let reason = if b.requirement.instrument == "current_market" {
                Some("unavailable_reference")
            } else if enabled && self.stream_epoch_current(stream) && failure_reason.is_some() {
                failure_reason.map(Unavailable::code)
            } else if !permitted {
                Some("disconnected")
            } else if b.reason.is_some_and(|reason| reason != Unavailable::Freshness) {
                b.reason.map(Unavailable::code)
            } else if !market_unexpired {
                Some("market_closed")
            } else if b.value.is_some()
                && b.capabilities.current.is_none()
                && now_ms >= b.valid_until_ms
            {
                Some("stale_data")
            } else {
                b.reason.map(Unavailable::code)
            };
            json!({"id":b.binding.binding_id.to_string(), "source":source_name(b.requirement.source),
                "field":b.requirement.field,"unit":match b.requirement.unit {RuleUnit::ContractCents=>"contract_cents",RuleUnit::UsdPerBtc=>"usd_per_btc",RuleUnit::Percent=>"percent"},
                "market_id":market,"side":side,"value":b.value.map(|v|v.to_string()),"identity":b.provider_identity,
                "provider_time_utc":b.provider_time_utc,"age_ms":b.age_ms.map(|age|age.saturating_add(now_ms.saturating_sub(self.snapshot.at_ms)).to_string()),
                "generation":b.generation.to_string(),"usable":permitted && market_unexpired && b.usable && now_ms < b.valid_until_ms,
                "history_ready":permitted && market_unexpired && b.history_ready && b.capabilities.history.is_none(),
                "reason":reason,
                "capabilities":{"current":b.capabilities.current.map(Unavailable::code),"observations":b.capabilities.observations.map(Unavailable::code),"history":b.capabilities.history.map(Unavailable::code)}})
        }).collect();
        let markets:Vec<_> = self.snapshot.markets.iter().map(|m|json!({"market_id":m.descriptor.market_id,"contract_id":m.descriptor.contract_id,
            "series_id":m.descriptor.series_id,"open_ms":m.descriptor.open_ms.to_string(),"close_ms":m.descriptor.close_ms.to_string(),
            "verified":m.verified && self.stream_current(Stream::Metadata),"preparation":if i128::from(now_ms) >= i128::from(m.descriptor.close_ms) {"expired"} else if !self.stream_current(Stream::Book) {"identified"} else {match m.preparation {Preparation::Unknown=>"unknown",Preparation::Identified=>"identified",Preparation::Collecting=>"collecting",Preparation::Ready=>"ready"}}})).collect();
        let current_market = self.snapshot.current_market.as_ref().filter(|id| {
            self.snapshot.markets.iter().any(|m| {
                &m.descriptor.market_id == *id
                    && i128::from(now_ms) < i128::from(m.descriptor.close_ms)
            })
        });
        json!({"kind":"snapshot","owner":self.snapshot.owner.to_string(),"revision":self.snapshot.revision.to_string(),
            "observed_at_utc_ms":(i128::from(self.epoch_utc_ms)+i128::from(self.snapshot.at_ms)).to_string(),
            "current_market":current_market,"markets":markets,"sources":sources,"bindings":bindings})
    }
    pub fn wire_details(&self, source: usize, now_ms: u64) -> Result<serde_json::Value, Failure> {
        if source >= 4 {
            return Err(Failure::InvalidRequest);
        }
        use metten_engine::sources::Stream;
        use serde_json::json;
        let streams = logical_streams(source);
        let mut values = Vec::new();
        for &stream in streams {
            if !self.enabled[source] || self.blocked[stream as usize] {
                continue;
            }
            let markets: Vec<_> = if matches!(stream, Stream::Book | Stream::Trades) {
                self.snapshot
                    .markets
                    .iter()
                    .map(|m| Some(m.descriptor.market_id.as_str()))
                    .collect()
            } else {
                vec![None]
            };
            for market in markets {
                if let Ok(Some(detail)) = self.details(stream, market) {
                    let detail = match detail {
                        wire::Details::Book {
                            sending_ts_ms,
                            clock,
                            sid,
                            sequence,
                            quantities,
                        } => {
                            json!({"kind":"book","sending_ts_ms":sending_ts_ms.map(|v|v.to_string()),"clock":clock.map(|v|v.to_string()),"sid":sid.to_string(),"sequence":sequence.to_string(),"quantity_unit":"contracts","quantities":{"yes_bid":quantities[0].map(|q|q.to_string()),"no_bid":quantities[1].map(|q|q.to_string()),"yes_ask":quantities[2].map(|q|q.to_string()),"no_ask":quantities[3].map(|q|q.to_string())}})
                        }
                        wire::Details::Trade {
                            sid,
                            sending_ts_ms,
                            quantity,
                            direction,
                            role,
                            sequence,
                            block,
                        } => {
                            json!({"kind":"trade","sid":sid.map(|v|v.to_string()),"sending_ts_ms":sending_ts_ms.map(|v|v.to_string()),"quantity_unit":if source==0 {"contracts"}else{"btc"},"quantity":quantity,"direction":direction,"role":role,"sequence":sequence.map(|v|v.to_string()),"block":block})
                        }
                        wire::Details::Brti {
                            sid,
                            sequence,
                            received_at,
                            sending_ts_ms,
                            trailing,
                            final_minute,
                        } => {
                            let average = |a: &Option<wire::Average>| {
                                a.as_ref().map(|a|json!({"value":a.value,"unit":"usd_per_btc","count":a.count.to_string(),"start":a.start.map(|v|v.to_string()),"end_exclusive":a.end_exclusive.map(|v|v.to_string()),"reference_usable":false}))
                            };
                            json!({"kind":"brti","sid":sid.to_string(),"sequence":sequence.to_string(),"received_at":received_at.map(|v|v.to_string()),"sending_ts_ms":sending_ts_ms.map(|v|v.to_string()),"trailing":average(trailing),"provisional_final_minute":average(final_minute)})
                        }
                    };
                    let binding = self.snapshot.bindings.iter().find(|b| {
                        b.requirement.source as usize == source
                            && match &b.binding.scope {
                                metten_engine::strategy::BindingScope::Underlying => {
                                    market.is_none()
                                }
                                metten_engine::strategy::BindingScope::Contract {
                                    market_id,
                                    ..
                                } => {
                                    market == Some(market_id.as_str())
                                        && (b.requirement.field == "last_trade")
                                            == (stream == Stream::Trades)
                                }
                                _ => false,
                            }
                    });
                    values.push(json!({"market_id":market,"detail":detail,"identity":binding.and_then(|b|b.provider_identity.as_ref()),"provider_time_utc":binding.and_then(|b|b.provider_time_utc.as_ref()),"receipt_age_ms":binding.and_then(|b|b.age_ms).map(|age|age.saturating_add(now_ms.saturating_sub(self.snapshot.at_ms)).to_string())}));
                }
            }
        }
        let projected = self.wire_snapshot(now_ms);
        let name = source_name(
            [
                metten_engine::strategy::Source::Kalshi,
                metten_engine::strategy::Source::Brti,
                metten_engine::strategy::Source::Coinbase,
                metten_engine::strategy::Source::Kraken,
            ][source],
        );
        let bindings: Vec<_> = projected["bindings"]
            .as_array()
            .ok_or(Failure::Protocol)?
            .iter()
            .filter(|b| b["source"] == name)
            .collect();
        Ok(
            json!({"kind":"details","source":name,"owner":projected["owner"],"revision":projected["revision"],"bindings":bindings,"values":values,"notice":"Received details only; capability, history and reference limitations remain."}),
        )
    }
    pub fn current(&self, provider: Provider) -> bool {
        if self.worker_signal.as_ref().is_some_and(|s| *s.borrow()) {
            return false;
        }
        self.fences[provider.index()]
            .as_ref()
            .is_none_or(|(f, g)| f.current(*g))
    }
    pub fn details(
        &self,
        stream: metten_engine::sources::Stream,
        market: Option<&str>,
    ) -> Result<Option<&wire::Details>, Failure> {
        if !self.stream_current(stream) {
            return Err(Failure::Stopped);
        }
        if matches!(
            stream,
            metten_engine::sources::Stream::Book | metten_engine::sources::Stream::Trades
        ) {
            let market = market.ok_or(Failure::InvalidRequest)?;
            Ok(self.market_details.get(market).and_then(|d| {
                d[usize::from(stream == metten_engine::sources::Stream::Trades)].as_ref()
            }))
        } else {
            if market.is_some() {
                return Err(Failure::InvalidRequest);
            }
            Ok(self.details[stream as usize].as_ref())
        }
    }
    pub fn snapshot(&self) -> Result<&metten_engine::sources::SourceSnapshot, Failure> {
        if self.schedule.borrow().is_some() {
            return Err(Failure::Stopped);
        }
        if self.worker_signal.as_ref().is_some_and(|s| *s.borrow()) {
            return Err(Failure::Stopped);
        }
        if self.gates.iter().enumerate().any(|(i, g)| {
            let live = g.load(Ordering::SeqCst);
            live == u64::MAX || live != self.epochs[i]
        }) {
            return Err(Failure::Stopped);
        }
        if self.fences.iter().flatten().any(|(f, g)| !f.current(*g)) {
            return Err(Failure::Stopped);
        }
        Ok(&self.snapshot)
    }
}
impl SourceWorker {
    fn observe_schedule_loss(&mut self, at: u64) -> Result<(), Failure> {
        let pending = { *self.schedule.borrow() };
        let Some(loss) = pending else { return Ok(()) };
        revoke_gates(&self.gates, &MARKET_STREAMS)?;
        self.kalshi.discard_book();
        let effect = self
            .state
            .invalidate(metten_engine::sources::Scope::Market, loss.reason, at, None)
            .map_err(|_| Failure::Protocol)?;
        let _ = effect;
        if let Some(epoch) = loss.epoch {
            self.epoch_utc_ms = epoch;
        }
        if loss.terminal {
            for stream in MARKET_STREAMS {
                self.blocked[stream as usize] = true;
            }
            if let Some(progress) = &self.progress {
                progress.send_modify(|p| {
                    for stream in MARKET_STREAMS {
                        p.disable(stream);
                    }
                });
            }
        }
        self.schedule.send_if_modified(|pending| {
            if *pending == Some(loss) {
                *pending = None;
                true
            } else {
                false
            }
        });
        Ok(())
    }
    pub fn observe_stops(&mut self, at: u64) -> Result<(), Failure> {
        for provider in [Provider::Kalshi, Provider::Coinbase, Provider::Kraken] {
            if self.links[provider.index()]
                .as_ref()
                .is_some_and(|(f, g)| !f.current(*g))
            {
                let stream = provider_streams(provider)[0];
                self.connection(
                    provider,
                    self.state.token(stream),
                    metten_engine::sources::Connection::Disconnected,
                    at,
                )?;
            }
        }
        Ok(())
    }
}
pub(super) fn source_envelope(
    at: u64,
    event: metten_engine::sources::SourceEvent,
) -> metten_engine::sources::Envelope {
    metten_engine::sources::Envelope {
        clock: metten_engine::sources::Clock {
            at_ms: at,
            receipt_ms: at,
            received_through_ms: at,
            completed_frontier_ms: 0,
        },
        provider_identity: None,
        provider_time_utc: None,
        event,
    }
}
fn engine_provider(provider: Provider) -> metten_engine::sources::Provider {
    match provider {
        Provider::Kalshi => metten_engine::sources::Provider::Kalshi,
        Provider::Coinbase => metten_engine::sources::Provider::Coinbase,
        Provider::Kraken => metten_engine::sources::Provider::Kraken,
    }
}
fn apply_source_event(
    state: &mut metten_engine::sources::SourceState,
    number: &mut u64,
    event: wire::Event,
    tokens: &[metten_engine::sources::StreamToken; 6],
    clock: metten_engine::sources::Clock,
    lease: Option<u64>,
    consume: &mut impl FnMut(metten_engine::sources::SourceEffect) -> Result<(), Failure>,
) -> Result<(), Failure> {
    use metten_engine::{
        sources::*,
        strategy::{BindingScope, Source},
    };
    let at = clock.at_ms;
    match event {
        wire::Event::Heartbeat | wire::Event::ControlAcknowledged(_) => Ok(()),
        wire::Event::Absent(stream) => consume(
            state
                .invalidate(Scope::Stream(stream), Unavailable::Awaiting, at, None)
                .map_err(|_| Failure::Protocol)?,
        ),
        wire::Event::Gap(..)
        | wire::Event::ControlRejected(..)
        | wire::Event::SubscriptionEnded(..)
        | wire::Event::Unsubscribed(..) => Err(Failure::InvalidRequest),
        wire::Event::MetadataChanged => consume(
            state
                .invalidate(Scope::Market, Unavailable::MetadataChanged, at, None)
                .map_err(|_| Failure::Protocol)?,
        ),
        wire::Event::Acknowledged(stream, _) => {
            if tokens[stream as usize] != state.token(stream) {
                return Ok(());
            }
            consume(
                state
                    .apply(source_envelope(
                        at,
                        SourceEvent::Subscription {
                            stream,
                            token: tokens[stream as usize],
                            state: Subscription::Acknowledged,
                        },
                    ))
                    .map_err(|_| Failure::Protocol)?,
            )
        }
        wire::Event::Prices(prices) => {
            if tokens[prices.stream as usize] != state.token(prices.stream) {
                return Err(Failure::Stopped);
            }
            let view = state.snapshot(at).map_err(|_| Failure::Protocol)?;
            let source = match prices.stream {
                Stream::Book | Stream::Trades | Stream::Metadata => Source::Kalshi,
                Stream::Brti => Source::Brti,
                Stream::Coinbase => Source::Coinbase,
                Stream::Kraken => Source::Kraken,
            };
            let mut updates = Vec::new();
            for binding in view.bindings {
                if binding.requirement.source != source {
                    continue;
                }
                let (market, side) = match &binding.binding.scope {
                    BindingScope::Underlying => (None, None),
                    BindingScope::Market { market_id } => (Some(market_id.as_str()), None),
                    BindingScope::Contract {
                        market_id, side, ..
                    } => (Some(market_id.as_str()), Some(*side)),
                };
                if market != prices.market.as_deref() {
                    continue;
                }
                if let Some(reading) = prices
                    .readings
                    .iter()
                    .find(|r| r.field == binding.requirement.field && r.side == side)
                {
                    if let Some(value) = reading.value {
                        let value = if prices.initialize || !binding.initialized {
                            ValueUpdate::Initialize(value)
                        } else {
                            ValueUpdate::Observation {
                                number: number.checked_add(1).ok_or(Failure::Capacity)?,
                                value,
                            }
                        };
                        updates.push(BindingUpdate {
                            binding: binding.binding,
                            unit: binding.requirement.unit,
                            value: Some(value),
                            valid_until_ms: lease,
                        });
                    } else if binding.initialized {
                        // Expire this absent side without changing the live subscription identity.
                        updates.push(BindingUpdate {
                            binding: binding.binding,
                            unit: binding.requirement.unit,
                            value: None,
                            valid_until_ms: Some(0),
                        });
                    }
                }
            }
            if updates.is_empty() {
                return Ok(());
            }
            *number = number.checked_add(1).ok_or(Failure::Capacity)?;
            let mut envelope = source_envelope(
                at,
                SourceEvent::Update {
                    stream: prices.stream,
                    token: tokens[prices.stream as usize],
                    updates,
                    coverage: vec![],
                },
            );
            envelope.clock = clock;
            envelope.provider_identity = Some(prices.identity);
            envelope.provider_time_utc = prices.time;
            consume(state.apply(envelope).map_err(|_| Failure::Protocol)?)
        }
    }
}

/// Native-only handle. Step 5 may expose a finite projection, never this sender or fixtures.
pub(super) enum SourceCommand {
    Connection {
        provider: Provider,
        state: metten_engine::sources::Connection,
        reply: tokio::sync::oneshot::Sender<Result<(), Failure>>,
    },
    FixtureMarkets {
        reply: tokio::sync::oneshot::Sender<Result<(), Failure>>,
    },
    Metadata {
        route: Route,
        bytes: memory::Bytes,
        epoch: i64,
        now: i64,
        token: metten_engine::sources::StreamToken,
        reply: tokio::sync::oneshot::Sender<Result<discovery::Parsed, Failure>>,
    },
    Markets {
        discovered: discovery::Discovered,
        token: metten_engine::sources::StreamToken,
        reply: tokio::sync::oneshot::Sender<Result<(), Failure>>,
    },
}
type Notice = (
    metten_engine::sources::StreamToken,
    metten_engine::sources::Connection,
    Option<Fence>,
);
type ControlSlot = Arc<std::sync::Mutex<Option<mpsc::Receiver<Control>>>>;
struct ControlOwner {
    slot: ControlSlot,
    receiver: Option<mpsc::Receiver<Control>>,
}
impl Drop for ControlOwner {
    fn drop(&mut self) {
        if let Ok(mut slot) = self.slot.lock() {
            *slot = self.receiver.take();
        }
    }
}
#[derive(Clone)]
pub struct SourceHandle {
    pub(super) command: mpsc::Sender<SourceCommand>,
    desired: watch::Sender<DesiredSources>,
    gates: [Arc<AtomicU64>; 6],
    fixtures: [watch::Sender<Option<Fence>>; 3],
    epoch: Instant,
    memory: memory::Memory,
    pub(super) schedule: watch::Sender<Option<ScheduleLoss>>,
    controls: [ControlSlot; 3],
    outbound: [mpsc::Sender<Control>; 3],
    notices: [watch::Sender<Option<Notice>>; 3],
    pub intake: mpsc::Sender<Intake>,
    pub latest: watch::Receiver<Option<Arc<SourceView>>>,
    _base_charge: Arc<tokio::sync::OwnedSemaphorePermit>,
}
impl Coordinator {
    pub fn start_sources(&mut self, mut source: SourceWorker) -> Result<SourceHandle, Failure> {
        if self.stopping || self.worker.is_some() {
            return Err(Failure::Stopped);
        }
        source.worker_signal = Some(self.stop.clone());
        let base_charge = source._base_charge.clone();
        let memory = source.memory.clone();
        let epoch = source.epoch;
        let schedule = source.schedule.clone();
        let channels: [_; 3] = std::array::from_fn(|_| mpsc::channel(7));
        let mut receivers: [ControlSlot; 3] =
            std::array::from_fn(|_| Arc::new(std::sync::Mutex::new(None)));
        let mut senders = Vec::new();
        for (i, (tx, rx)) in channels.into_iter().enumerate() {
            source.controls[i] = Some(tx.clone());
            senders.push(tx);
            receivers[i] = Arc::new(std::sync::Mutex::new(Some(rx)));
        }
        let outbound: [mpsc::Sender<Control>; 3] =
            senders.try_into().map_err(|_| Failure::Unavailable)?;
        let notices: [watch::Sender<Option<Notice>>; 3] =
            std::array::from_fn(|_| watch::channel(None).0);
        let notice_readers = notices.clone();
        let (desired, membership) = watch::channel(DesiredSources {
            enabled: source.enabled,
            revocations: source.seen_revocations,
        });
        let gates = source.gates.clone();
        let fixtures = std::array::from_fn(|_| watch::channel(None).0);
        let (command, mut command_rx) = mpsc::channel(1);
        let (tx, mut rx) = intake();
        let (publish, latest) = watch::channel(None);
        let stop = self.stop.subscribe();
        let lifetime = WorkerLifetime(self.stop.clone());
        self.worker = Some(std::thread::spawn(move || {
            let _lifetime = lifetime;
            let idle = Arc::new(IntakeWake::default());
            let waker = std::task::Waker::from(idle.clone());
            while !*stop.borrow() {
                // Claim the existing processing slot before clock/control work; decoding still follows all checks.
                let received = poll_intake(&mut rx, &waker);
                let mut at = match u64::try_from(epoch.elapsed().as_millis()) {
                    Ok(at) => at,
                    Err(_) => return,
                };
                if source.observe_membership(*membership.borrow(), at).is_err() {
                    return;
                }
                if source.observe_schedule_loss(at).is_err() {
                    return;
                }
                for provider in [Provider::Kalshi, Provider::Coinbase, Provider::Kraken] {
                    if let Some((token, state, session)) =
                        notice_readers[provider.index()].send_replace(None)
                        && source.connection(provider, token, state, at).is_ok()
                        && let Some(session) = session
                    {
                        source.links[provider.index()] = Some((session.clone(), session.token()));
                    }
                }
                if source.observe_stops(at).is_err() || source.advance(at, |_| Ok(())).is_err() {
                    return;
                }
                if let Ok(command) = command_rx.try_recv() {
                    match command {
                        SourceCommand::Connection {
                            provider,
                            state,
                            reply,
                        } => {
                            let stream = match provider {
                                Provider::Kalshi => metten_engine::sources::Stream::Book,
                                Provider::Coinbase => metten_engine::sources::Stream::Coinbase,
                                _ => metten_engine::sources::Stream::Kraken,
                            };
                            let token = source.state.token(stream);
                            let result = source.connection(provider, token, state, at).map(|_| ());
                            let _ = reply.send(result);
                        }
                        SourceCommand::FixtureMarkets { reply } => {
                            let result = (|| {
                                let markets: Vec<_> = [0, 900000]
                                    .into_iter()
                                    .enumerate()
                                    .map(|(i, open)| metten_engine::strategy::MarketDescriptor {
                                        series_id: "KXBTC15M".into(),
                                        market_id: format!("KXBTC15M-{i}"),
                                        contract_id: format!("contract-{i}"),
                                        open_ms: open,
                                        close_ms: open + 900000,
                                    })
                                    .collect();
                                let token =
                                    source.state.token(metten_engine::sources::Stream::Metadata);
                                source.install_markets(&markets, token, at, |_| Ok(()))?;
                                for market in markets {
                                    source
                                        .state
                                        .apply(source_envelope(
                                            at,
                                            metten_engine::sources::SourceEvent::Metadata {
                                                token: source.state.token(
                                                    metten_engine::sources::Stream::Metadata,
                                                ),
                                                market_id: market.market_id,
                                                verified: true,
                                            },
                                        ))
                                        .map_err(|_| Failure::Protocol)?;
                                }
                                Ok(())
                            })();
                            let _ = reply.send(result);
                        }
                        SourceCommand::Markets {
                            discovered,
                            token,
                            reply,
                        } => {
                            let result = if reply.is_closed() {
                                Err(Failure::Stopped)
                            } else {
                                source.install_markets(&discovered.markets, token, at, |_| Ok(()))
                            };
                            let _ = reply.send(result);
                        }
                        SourceCommand::Metadata {
                            route,
                            bytes,
                            epoch,
                            now,
                            token,
                            reply,
                        } => {
                            if !reply.is_closed() {
                                let result = if token
                                    != source.state.token(metten_engine::sources::Stream::Metadata)
                                {
                                    Err(Failure::Stopped)
                                } else {
                                    discovery::parse(
                                        &route,
                                        bytes.as_slice(),
                                        epoch,
                                        now,
                                        &source.memory,
                                    )
                                };
                                let _ = reply.send(result);
                            }
                        }
                    }
                }
                let mut dispatched = None;
                let mut processed = false;
                match received {
                    Ok(mut item) => {
                        processed = true;
                        dispatched = item.completion.take();
                        at = at.max(u64::try_from(epoch.elapsed().as_millis()).unwrap_or(u64::MAX));
                        if source.observe_schedule_loss(at).is_err()
                            || source.observe_stops(at).is_err()
                            || source.advance(at, |_| Ok(())).is_err()
                        {
                            return;
                        }
                        let result = source.process(item, at, 0, |_| Ok(()));
                        if matches!(result, Err(Failure::Stopped | Failure::InvalidRequest))
                            && let Some(reply) = dispatched.take()
                        {
                            let _ = reply.send(Err(Failure::Stopped));
                        }
                    }
                    Err(mpsc::error::TryRecvError::Disconnected) => return,
                    Err(mpsc::error::TryRecvError::Empty) => (),
                }
                if source.observe_schedule_loss(at).is_err() {
                    return;
                }
                match source.snapshot(at) {
                    Ok(view) => {
                        publish.send_replace(Some(Arc::new(view)));
                        if let Some(reply) = dispatched {
                            let _ = reply.send(Ok(()));
                        }
                    }
                    Err(Failure::Stopped) if source.schedule.borrow().is_some() => (),
                    Err(_) => return,
                }
                if !processed {
                    idle.wait(Duration::from_micros(1500));
                }
            }
            let stopped = source
                .state
                .stop(u64::try_from(epoch.elapsed().as_millis()).unwrap_or(u64::MAX));
            let _ = stopped;
        }));
        Ok(SourceHandle {
            desired,
            gates,
            fixtures,
            intake: tx,
            latest,
            memory,
            epoch,
            schedule,
            command,
            controls: receivers,
            outbound,
            notices,
            _base_charge: base_charge,
        })
    }
}

impl SourceWorker {
    /// A discovery completion retains the metadata token captured before its request.
    pub fn install_markets(
        &mut self,
        markets: &[metten_engine::strategy::MarketDescriptor],
        token: metten_engine::sources::StreamToken,
        at: u64,
        mut consume: impl FnMut(metten_engine::sources::SourceEffect) -> Result<(), Failure>,
    ) -> Result<(), Failure> {
        use metten_engine::{sources::*, strategy::BindingScope};
        if markets.len() > 2
            || token != self.state.token(Stream::Metadata)
            || markets
                .iter()
                .any(|m| i128::from(m.close_ms) <= i128::from(at))
        {
            return Err(Failure::Stopped);
        }
        if markets.is_empty() {
            self.state.snapshot(at).map_err(|_| Failure::Protocol)?;
        }
        for descriptor in markets {
            let before = self.state.snapshot(at).map_err(|_| Failure::Protocol)?;
            if let Some(old) = before
                .markets
                .iter()
                .find(|m| m.descriptor.market_id == descriptor.market_id)
            {
                if old.descriptor != *descriptor {
                    self.kalshi.discard_book();
                    revoke_gates(
                        &self.gates,
                        &[Stream::Book, Stream::Trades, Stream::Metadata],
                    )?;
                    let effect = self
                        .state
                        .apply(source_envelope(
                            at,
                            SourceEvent::ReplaceMarket {
                                token,
                                expected: old.descriptor.clone(),
                                replacement: descriptor.clone(),
                            },
                        ))
                        .map_err(|_| Failure::Protocol)?;
                    consume(effect)?;
                    // Replacement deliberately needs a new request, not a re-labelled old response.
                    return Err(Failure::Stopped);
                }
            } else {
                let mut next = before
                    .bindings
                    .iter()
                    .map(|b| b.binding.binding_id)
                    .max()
                    .unwrap_or(0);
                let mut bindings = Vec::new();
                for requirement in self
                    .requirements
                    .iter()
                    .filter(|r| r.instrument != "btc_usd")
                {
                    use metten_engine::strategy::{BindingDescriptor, Side, SourceSide};
                    let scopes = if requirement.instrument == "current_market" {
                        vec![BindingScope::Market {
                            market_id: descriptor.market_id.clone(),
                        }]
                    } else {
                        let sides = match requirement.side {
                            Some(SourceSide::Selected) => vec![Side::Yes, Side::No],
                            Some(SourceSide::Fixed(s)) => vec![s],
                            None => return Err(Failure::Protocol),
                        };
                        sides
                            .into_iter()
                            .map(|side| BindingScope::Contract {
                                market_id: descriptor.market_id.clone(),
                                contract_id: descriptor.contract_id.clone(),
                                side,
                            })
                            .collect()
                    };
                    for scope in scopes {
                        next = next.checked_add(1).ok_or(Failure::Capacity)?;
                        bindings.push(BindingDescriptor {
                            binding_id: next,
                            input_id: requirement.input_id.clone(),
                            scope,
                        });
                    }
                }
                #[cfg(test)]
                let capabilities = if self.lease.is_some() {
                    bindings
                        .iter()
                        .map(|b| {
                            (
                                b.binding_id,
                                Capabilities {
                                    current: None,
                                    observations: None,
                                    history: None,
                                },
                            )
                        })
                        .collect()
                } else {
                    vec![]
                };
                #[cfg(not(test))]
                let capabilities = vec![];
                consume(
                    self.state
                        .apply(source_envelope(
                            at,
                            SourceEvent::RegisterMarket {
                                token,
                                descriptor: descriptor.clone(),
                                bindings,
                                capabilities,
                            },
                        ))
                        .map_err(|_| Failure::Protocol)?,
                )?;
            }
            consume(
                self.state
                    .apply(source_envelope(
                        at,
                        SourceEvent::Metadata {
                            token,
                            market_id: descriptor.market_id.clone(),
                            verified: true,
                        },
                    ))
                    .map_err(|_| Failure::Protocol)?,
            )?;
        }
        self.refresh_tickers()?;
        if let Some(progress) = &self.progress {
            progress.send_modify(|p| {
                p.tokens[Stream::Metadata as usize] = Some(self.state.token(Stream::Metadata))
            });
            let trade_token = self.state.token(Stream::Trades);
            let sent_trade_token = { progress.borrow().tokens[Stream::Trades as usize] };
            if sent_trade_token != Some(trade_token)
                && !self.tickers.is_empty()
                && !self.blocked[Stream::Trades as usize]
                && let Some(sid) = self.trade_sid
            {
                self.command_id = self.command_id.checked_add(1).ok_or(Failure::Capacity)?;
                progress.send_modify(|p| {
                    p.recover(Control::KalshiTradeUpdate {
                        id: self.command_id,
                        sid,
                        action: BookAction::Add,
                        markets: self.tickers.clone(),
                        token: trade_token,
                    })
                });
            }
            let token = self.state.token(Stream::Book);
            let sent_book_token = { progress.borrow().tokens[Stream::Book as usize] };
            if sent_book_token != Some(token)
                && !self.tickers.is_empty()
                && !self.blocked[Stream::Book as usize]
                && let Some(sid) = self.book_sid
            {
                self.command_id = self.command_id.checked_add(1).ok_or(Failure::Capacity)?;
                progress.send_modify(|p| p.desired_book = Some(token));
                progress.send_modify(|p| {
                    p.recover(Control::KalshiBookUpdate {
                        id: self.command_id,
                        sid,
                        action: BookAction::Snapshot,
                        markets: self.tickers.clone(),
                        token: Some(token),
                    })
                });
            }
        }
        Ok(())
    }
    fn refresh_tickers(&mut self) -> Result<(), Failure> {
        use metten_engine::{sources::Stream, strategy::Source};
        if self
            .state
            .market_ids()
            .eq(self.tickers.iter().map(String::as_str))
        {
            return Ok(());
        }
        let tickers: Vec<_> = self.state.market_ids().map(str::to_owned).collect();
        let added: Vec<_> = tickers
            .iter()
            .filter(|t| !self.tickers.contains(t))
            .cloned()
            .collect();
        let removed: Vec<_> = self
            .tickers
            .iter()
            .filter(|t| !tickers.contains(t))
            .cloned()
            .collect();
        self.kalshi.retain(&tickers);
        self.market_details.retain(|id, _| tickers.contains(id));
        if let Some(outbound) = &self.controls[0] {
            for (stream, sid, channel) in [
                (Stream::Book, self.book_sid, Channel::Book),
                (Stream::Trades, self.trade_sid, Channel::Trade),
            ] {
                let needed = self.requirements.iter().any(|r| {
                    r.source == Source::Kalshi
                        && r.instrument != "current_market"
                        && (r.field == "last_trade") == (stream == Stream::Trades)
                });
                if !needed || self.blocked[stream as usize] || self.kalshi.retiring(stream) {
                    continue;
                }
                for (markets, action) in [(&removed, BookAction::Delete), (&added, BookAction::Add)]
                {
                    if markets.is_empty() {
                        continue;
                    }
                    self.command_id = self.command_id.checked_add(1).ok_or(Failure::Capacity)?;
                    let token = self.state.token(stream);
                    let control = match sid {
                        Some(sid) if stream == Stream::Book => Control::KalshiBookUpdate {
                            id: self.command_id,
                            sid,
                            action,
                            markets: markets.clone(),
                            token: Some(token),
                        },
                        Some(sid) => Control::KalshiTradeUpdate {
                            id: self.command_id,
                            sid,
                            action,
                            markets: markets.clone(),
                            token,
                        },
                        None if matches!(action, BookAction::Add) => Control::KalshiSubscribe {
                            id: self.command_id,
                            channel,
                            markets: tickers.clone(),
                        },
                        None => continue,
                    };
                    outbound.try_send(control).map_err(|_| Failure::Capacity)?;
                }
            }
        }
        self.tickers = tickers;
        Ok(())
    }
    pub fn advance(
        &mut self,
        at: u64,
        mut consume: impl FnMut(metten_engine::sources::SourceEffect) -> Result<(), Failure>,
    ) -> Result<(), Failure> {
        if at < self.at {
            return Err(Failure::InvalidRequest);
        }
        let mut view = self.state.snapshot(at).map_err(|_| Failure::Protocol)?;
        if view
            .markets
            .iter()
            .any(|m| i128::from(m.descriptor.close_ms) <= i128::from(at))
        {
            revoke_gates(&self.gates, &MARKET_STREAMS)?;
        }
        view.markets.sort_by_key(|m| m.descriptor.open_ms);
        for market in &view.markets {
            let Ok(opening) = u64::try_from(market.descriptor.open_ms) else {
                continue;
            };
            if self.at <= opening
                && opening <= at
                && i128::from(at) < i128::from(market.descriptor.close_ms)
            {
                // Never give post-opening readiness or invalidity an earlier source clock.
                let preparation = match self.state.snapshot(opening) {
                    Ok(view) => view,
                    Err(error) if error.code == "invalid_request" => continue,
                    Err(_) => return Err(Failure::Protocol),
                };
                drop(preparation);
                consume(
                    self.state
                        .apply(source_envelope(
                            opening,
                            metten_engine::sources::SourceEvent::Boundary {
                                market_id: market.descriptor.market_id.clone(),
                            },
                        ))
                        .map_err(|_| Failure::Protocol)?,
                )?;
            }
        }
        if let Some(market) = view.markets.iter().find(|m| {
            m.verified
                && i128::from(m.descriptor.open_ms) < i128::from(at)
                && i128::from(at) < i128::from(m.descriptor.close_ms)
                && view.current_market.as_deref() != Some(m.descriptor.market_id.as_str())
        }) {
            consume(
                self.state
                    .apply(source_envelope(
                        at,
                        metten_engine::sources::SourceEvent::Metadata {
                            token: self.state.token(metten_engine::sources::Stream::Metadata),
                            market_id: market.descriptor.market_id.clone(),
                            verified: true,
                        },
                    ))
                    .map_err(|_| Failure::Protocol)?,
            )?;
        }
        consume(
            self.state
                .apply(source_envelope(
                    at,
                    metten_engine::sources::SourceEvent::Advance,
                ))
                .map_err(|_| Failure::Protocol)?,
        )?;
        self.at = at;
        self.refresh_tickers()
    }
}

impl Coordinator {
    /// Explicit native setup only; startup and screen refresh never call this.
    fn start_fixture(&mut self, provider: Provider, handle: &SourceHandle) -> Result<(), Failure> {
        let session = self.fence(provider).session()?;
        let link = handle.fixtures[provider.index()].clone();
        let notice = handle.notices[provider.index()].clone();
        let mut latest = handle.latest.clone();
        let mut stop = self.cancellation(provider);
        let charge = handle.memory.reserve(4 * MAX_BYTES)?;
        let slot = handle.controls[provider.index()].clone();
        let receiver = slot
            .lock()
            .map_err(|_| Failure::Unavailable)?
            .take()
            .ok_or(Failure::StillStopping)?;
        let mut controls = ControlOwner {
            slot,
            receiver: Some(receiver),
        };
        self.spawn(provider,async move {
            let _session_lifetime=ProviderLifetime(session.clone());
            let _charge=charge;
            loop {
                if *stop.borrow() {return}
                let snapshot=latest.borrow().clone();
                if let Some(view)=snapshot
                    && let Ok(snapshot)=view.snapshot() {
                        let stream=match provider {Provider::Kalshi=>metten_engine::sources::Stream::Book,Provider::Coinbase=>metten_engine::sources::Stream::Coinbase,_=>metten_engine::sources::Stream::Kraken};
                        let token=snapshot.streams.iter().find(|(s,_,_)|*s==stream).unwrap().2;
                        notice.send_replace(Some((token,metten_engine::sources::Connection::Open,Some(session.clone()))));
                        link.send_replace(Some(session.clone()));
                        break;
                }
                tokio::select! { _=stop.changed()=>return, _=latest.changed()=>() }
            }
            loop {tokio::select!{biased; _=stop.changed()=>break, command=controls.receiver.as_mut().unwrap().recv()=>{if command.is_none(){break}}}}
            link.send_replace(None);
        })
    }
    pub fn start_provider(
        &mut self,
        provider: Provider,
        transport: Arc<transport::Transport>,
        handle: &mut SourceHandle,
        credential: Option<(String, signing::Key)>,
    ) -> Result<(), Failure> {
        if transport.is_isolated() {
            return self.start_fixture(provider, handle);
        }
        if !handle.memory.same_owner(transport.memory()) {
            return Err(Failure::InvalidRequest);
        }
        let slot = handle.controls[provider.index()].clone();
        let receiver = slot
            .lock()
            .map_err(|_| Failure::Unavailable)?
            .take()
            .ok_or(Failure::InvalidRequest)?;
        let mut controls = ControlOwner {
            slot,
            receiver: Some(receiver),
        };
        let outbound = handle.outbound[provider.index()].clone();
        let intake = handle.intake.clone();
        let mut latest = handle.latest.clone();
        let metadata_command = handle.command.clone();
        let epoch = handle.epoch;
        let notice = handle.notices[provider.index()].clone();
        let parent = self.fence(provider);
        let mut stop = self.cancellation(provider);
        let base_charge = handle._base_charge.clone();
        self.spawn(provider, async move {
            let _base_charge = base_charge;
            let mut obsolete = None;
            let mut metadata_validated = false;
            let context_for = |view: &Arc<SourceView>| {
                let v = &view.snapshot;
                let tokens = SOURCE_STREAMS.map(|s| {
                    v.streams
                        .iter()
                        .find(|(stream, _, _)| *stream == s)
                        .unwrap()
                        .2
                });
                let tickers = v
                    .markets
                    .iter()
                    .map(|m| m.descriptor.market_id.clone())
                    .collect::<Vec<_>>();
                let mut streams = Vec::new();
                for requirement in &view.requirements {
                    let stream = match requirement.source {
                        metten_engine::strategy::Source::Kalshi
                            if requirement.instrument == "current_market" =>
                        {
                            metten_engine::sources::Stream::Metadata
                        }
                        metten_engine::strategy::Source::Kalshi
                            if requirement.field == "last_trade" =>
                        {
                            metten_engine::sources::Stream::Trades
                        }
                        metten_engine::strategy::Source::Kalshi => {
                            metten_engine::sources::Stream::Book
                        }
                        metten_engine::strategy::Source::Brti => {
                            metten_engine::sources::Stream::Brti
                        }
                        metten_engine::strategy::Source::Coinbase => {
                            metten_engine::sources::Stream::Coinbase
                        }
                        metten_engine::strategy::Source::Kraken => {
                            metten_engine::sources::Stream::Kraken
                        }
                    };
                    if view.enabled[requirement.source as usize]
                        && !view.blocked[stream as usize]
                        && !streams.contains(&stream)
                    {
                        streams.push(stream)
                    }
                }
                (tokens, tickers, streams, view.epoch_utc_ms)
            };
            loop {
                if !parent.current(parent.token()) || *stop.borrow() {
                    return;
                }
                let context = loop {
                    let context = latest
                        .borrow()
                        .as_ref()
                        .filter(|v| v.snapshot().is_ok())
                        .map(&context_for);
                    if let Some(context) = context {
                        let stream = match provider {
                            Provider::Kalshi => metten_engine::sources::Stream::Book,
                            Provider::Coinbase => metten_engine::sources::Stream::Coinbase,
                            Provider::Kraken => metten_engine::sources::Stream::Kraken,
                        };
                        if obsolete != Some(context.0[stream as usize]) {
                            break context;
                        }
                    }
                    if bounded(&mut stop, Instant::now() + SHUTDOWN, async {
                        latest.changed().await.map_err(|_| Failure::Stopped)
                    })
                    .await
                    .is_err()
                    {
                        return;
                    }
                };
                let (tokens, _, streams, _) = context;
                let connection_stream = match provider {
                    Provider::Kalshi => metten_engine::sources::Stream::Book,
                    Provider::Coinbase => metten_engine::sources::Stream::Coinbase,
                    Provider::Kraken => metten_engine::sources::Stream::Kraken,
                };
                if !provider_streams(provider)
                    .iter()
                    .any(|s| streams.contains(s))
                {
                    return;
                }
                let session = match parent.session() {
                    Ok(session) => session,
                    Err(_) => return,
                };
                while controls.receiver.as_mut().unwrap().try_recv().is_ok() {}
                let material = credential.as_ref().map(|(id, key)| (id.as_str(), key));
                let deadline = Instant::now() + Duration::from_secs(10);
                let connected = transport
                    .connect(provider, material, &mut stop, deadline, &session)
                    .await;
                let connection = match connected {
                    Ok(connection) => connection,
                    Err(error) => {
                        use metten_engine::sources::Connection;
                        obsolete = Some(tokens[connection_stream as usize]);
                        notice.send_replace(Some((
                            tokens[connection_stream as usize],
                            match error {
                                Failure::Denied => Connection::AccessRefused,
                                Failure::Missing => Connection::MissingConfiguration,
                                _ => Connection::Disconnected,
                            },
                            None,
                        )));
                        if matches!(error, Failure::Transport | Failure::Deadline)
                            && parent.current(parent.token())
                        {
                            continue;
                        } else {
                            return;
                        }
                    }
                };
                let fresh = latest
                    .borrow()
                    .as_ref()
                    .filter(|v| v.snapshot().is_ok())
                    .map(&context_for);
                let Some((fresh_tokens, tickers, streams, epoch_utc_ms)) = fresh else {
                    return;
                };
                if provider_streams(provider)
                    .iter()
                    .any(|s| fresh_tokens[*s as usize] != tokens[*s as usize])
                {
                    drop(connection);
                    obsolete = None;
                    continue;
                }
                let source_clock = (epoch, epoch_utc_ms);
                let mut commands = Vec::new();
                match provider {
                    Provider::Kalshi => {
                        for (stream, channel) in [
                            (metten_engine::sources::Stream::Book, Channel::Book),
                            (metten_engine::sources::Stream::Trades, Channel::Trade),
                            (metten_engine::sources::Stream::Brti, Channel::Brti),
                        ] {
                            if streams.contains(&stream)
                                && (!matches!(channel, Channel::Book | Channel::Trade)
                                    || !tickers.is_empty())
                            {
                                commands.push(Control::KalshiSubscribe {
                                    id: commands.len() as u32 + 1,
                                    channel,
                                    markets: if matches!(channel, Channel::Book | Channel::Trade) {
                                        tickers.clone()
                                    } else {
                                        vec![]
                                    },
                                })
                            }
                        }
                        if streams.iter().any(|s| {
                            matches!(
                                s,
                                metten_engine::sources::Stream::Book
                                    | metten_engine::sources::Stream::Trades
                                    | metten_engine::sources::Stream::Metadata
                            )
                        }) {
                            commands.push(Control::KalshiSubscribe {
                                id: commands.len() as u32 + 1,
                                channel: Channel::Lifecycle,
                                markets: vec![],
                            })
                        }
                    }
                    Provider::Coinbase if streams.contains(&connection_stream) => {
                        commands.push(Control::Coinbase { subscribe: true })
                    }
                    Provider::Kraken if streams.contains(&connection_stream) => {
                        commands.push(Control::Kraken { subscribe: true })
                    }
                    _ => return,
                }
                if commands.is_empty() {
                    return;
                }
                notice.send_replace(Some((
                    tokens[connection_stream as usize],
                    metten_engine::sources::Connection::Open,
                    Some(session.clone()),
                )));
                for command in commands {
                    if outbound.try_send(command).is_err() {
                        session.fail(Failure::Capacity);
                        return;
                    }
                }
                let service = transport.serve(
                    connection,
                    controls.receiver.as_mut().unwrap(),
                    intake.clone(),
                    tokens,
                    stop.clone(),
                    Some(watch::channel(transport::Progress::new()).0),
                );
                let metadata = async {
                    if provider == Provider::Kalshi {
                        run_metadata(
                            &transport,
                            latest.clone(),
                            metadata_command.clone(),
                            session.clone(),
                            stop.clone(),
                            source_clock,
                            &mut metadata_validated,
                        )
                        .await
                    } else {
                        std::future::pending::<Result<(), Failure>>().await
                    }
                };
                let result = tokio::try_join!(service, metadata).map(|_| ());
                let reason = session
                    .cause
                    .lock()
                    .ok()
                    .and_then(|cause| *cause)
                    .or(result.err());
                if !matches!(reason, Some(Failure::Transport | Failure::Deadline))
                    || !parent.current(parent.token())
                {
                    return;
                }
                obsolete = Some(tokens[connection_stream as usize]);
                notice.send_replace(Some((
                    tokens[connection_stream as usize],
                    metten_engine::sources::Connection::Disconnected,
                    None,
                )));
            }
        })
    }
}

impl SourceWorker {
    pub fn prepare_display(memory: memory::Memory) -> Result<Self, Failure> {
        let mut source = Self::new(
            metten_engine::sources::SourceState::display().map_err(|_| Failure::InvalidRequest)?,
            memory,
        )?;
        source.enabled = [false; 4];
        Ok(source)
    }
    pub fn prepare(
        strategy: &metten_engine::strategy::ValidatedStrategy,
        markets: &[metten_engine::strategy::MarketDescriptor],
        memory: memory::Memory,
    ) -> Result<Self, Failure> {
        use metten_engine::strategy::{BindingDescriptor, BindingScope, Side, SourceSide};
        if markets.len() > 2 {
            return Err(Failure::Capacity);
        }
        for market in markets {
            if [&market.series_id, &market.market_id, &market.contract_id]
                .iter()
                .any(|s| s.len() > 128 || s.capacity() > 128)
            {
                return Err(Failure::Capacity);
            }
        }
        let _construction = memory.reserve(64 * 1024)?;
        let requirements = strategy.source_requirements();
        let count: usize = requirements
            .iter()
            .map(|r| {
                if r.instrument == "btc_usd" {
                    1
                } else {
                    markets.len()
                        * if r.side == Some(SourceSide::Selected) {
                            2
                        } else {
                            1
                        }
                }
            })
            .sum();
        if count > 128 {
            return Err(Failure::Capacity);
        }
        let mut bindings = Vec::with_capacity(count);
        for requirement in requirements {
            if requirement.instrument == "btc_usd" {
                bindings.push(BindingDescriptor {
                    binding_id: bindings.len() as u32 + 1,
                    input_id: requirement.input_id,
                    scope: BindingScope::Underlying,
                });
                continue;
            }
            for market in markets {
                if requirement.instrument == "current_market" {
                    bindings.push(BindingDescriptor {
                        binding_id: bindings.len() as u32 + 1,
                        input_id: requirement.input_id.clone(),
                        scope: BindingScope::Market {
                            market_id: market.market_id.clone(),
                        },
                    });
                    continue;
                }
                let sides = match requirement.side {
                    Some(SourceSide::Selected) => vec![Side::Yes, Side::No],
                    Some(SourceSide::Fixed(side)) => vec![side],
                    None => return Err(Failure::InvalidRequest),
                };
                for side in sides {
                    bindings.push(BindingDescriptor {
                        binding_id: bindings.len() as u32 + 1,
                        input_id: requirement.input_id.clone(),
                        scope: BindingScope::Contract {
                            market_id: market.market_id.clone(),
                            contract_id: market.contract_id.clone(),
                            side,
                        },
                    })
                }
            }
        }
        let source = metten_engine::sources::SourceState::new(strategy, markets, &bindings, &[])
            .map_err(|_| Failure::InvalidRequest)?;
        let mut worker = Self::new(source, memory)?;
        worker.requirements = strategy.source_requirements();
        Ok(worker)
    }
}

async fn run_metadata(
    transport: &transport::Transport,
    mut latest: watch::Receiver<Option<Arc<SourceView>>>,
    command: mpsc::Sender<SourceCommand>,
    fence: Fence,
    mut stop: watch::Receiver<bool>,
    source_clock: (Instant, i64),
    metadata_validated: &mut bool,
) -> Result<(), Failure> {
    use metten_engine::sources::Stream;
    let mut epoch_utc = source_clock.1;
    let mut initial_attempt = !*metadata_validated;
    let mut previous_count = 0;
    let mut checks: Option<(i64, discovery::PreparationChecks)> = None;
    let mut attempted_metadata = None;
    let mut previous = (Instant::now(), utc_now_ms()?);
    let mut missing_after_open = None;
    loop {
        if *stop.borrow() || !fence.current(fence.token()) {
            return Err(Failure::Stopped);
        }
        let now = utc_now_ms()?;
        let instant = Instant::now();
        let elapsed = i64::try_from(instant.saturating_duration_since(previous.0).as_millis())
            .map_err(|_| Failure::Unavailable)?;
        let wall = now.checked_sub(previous.1).ok_or(Failure::Unavailable)?;
        let discontinuity = clock_discontinuity(elapsed, wall, true);
        previous = (instant, now);
        if latest.borrow().as_ref().is_none_or(|v| !v.enabled[0]) {
            bounded(&mut stop, Instant::now() + Duration::from_secs(60), async {
                latest.changed().await.map_err(|_| Failure::Stopped)
            })
            .await?;
            continue;
        }
        let view = latest.borrow().clone();
        if let Some(view) = view {
            if discontinuity {
                let at = i64::try_from(source_clock.0.elapsed().as_millis())
                    .map_err(|_| Failure::Unavailable)?;
                epoch_utc = now.checked_sub(at).ok_or(Failure::Unavailable)?;
                view.schedule_loss(ScheduleLoss {
                    epoch: Some(epoch_utc),
                    reason: metten_engine::sources::Unavailable::MetadataChanged,
                    terminal: false,
                });
                if let Some((_, checks)) = &mut checks {
                    checks.skip_through(now - epoch_utc);
                }
                attempted_metadata = None;
            } else if !view.blocked[Stream::Metadata as usize]
                && let Ok(snapshot) = view.snapshot()
            {
                let token = snapshot
                    .streams
                    .iter()
                    .find(|(s, _, _)| *s == Stream::Metadata)
                    .unwrap()
                    .2;
                let relative = now.checked_sub(epoch_utc).ok_or(Failure::Unavailable)?;
                let open = snapshot
                    .markets
                    .iter()
                    .map(|m| m.descriptor.close_ms)
                    .min()
                    .unwrap_or(0);
                if checks.as_ref().is_none_or(|(o, _)| *o != open) {
                    checks = Some((open, discovery::PreparationChecks::new(open, relative)));
                }
                let missing = snapshot.markets.len() < 2;
                let unverified: Vec<_> = snapshot
                    .markets
                    .iter()
                    .filter(|m| !m.verified)
                    .map(|m| m.descriptor.clone())
                    .collect();
                let needs_work = missing || !unverified.is_empty();
                let retired = previous_count == 2 && snapshot.markets.len() < 2;
                previous_count = snapshot.markets.len();
                let due = checks
                    .as_mut()
                    .is_some_and(|(_, c)| c.due(relative, !needs_work, false, false));
                let post_open = relative >= open;
                let permitted = !post_open || missing_after_open.is_none_or(|t| instant >= t);
                let request = initial_attempt
                    || (needs_work && (due || retired) && permitted)
                    || (!unverified.is_empty() && attempted_metadata != Some(token) && permitted);
                if request {
                    initial_attempt = false;
                    let startup = !*metadata_validated;
                    attempted_metadata = Some(token);
                    if post_open {
                        missing_after_open = Some(instant + Duration::from_secs(60));
                    }
                    let result = if !startup && !unverified.is_empty() {
                        discovery::revalidate(
                            transport,
                            &mut stop,
                            &fence,
                            &unverified,
                            epoch_utc,
                            now,
                            &command,
                            token,
                        )
                        .await
                    } else {
                        discovery::pair(
                            transport, &mut stop, &fence, now, epoch_utc, startup, &command, token,
                        )
                        .await
                    };
                    match result {
                        Ok(discovered) => {
                            *metadata_validated = true;
                            let (reply, receive) = tokio::sync::oneshot::channel();
                            let deadline = Instant::now() + SHUTDOWN;
                            let installed = bounded(&mut stop, deadline, async {
                                command
                                    .send(SourceCommand::Markets {
                                        discovered,
                                        token,
                                        reply,
                                    })
                                    .await
                                    .map_err(|_| Failure::Stopped)?;
                                receive.await.map_err(|_| Failure::Stopped)
                            })
                            .await?;
                            match installed {
                                Ok(()) => {
                                    *metadata_validated = true;
                                }
                                Err(Failure::Stopped | Failure::Protocol) => (),
                                Err(error) => return Err(error),
                            }
                        }
                        Err(
                            Failure::Budget
                            | Failure::Unavailable
                            | Failure::Deadline
                            | Failure::Stopped,
                        ) => (),
                        Err(error) => {
                            use metten_engine::sources::Unavailable;
                            view.schedule_loss(ScheduleLoss {
                                epoch: None,
                                reason: match error {
                                    Failure::Denied => Unavailable::Access,
                                    Failure::Capacity => Unavailable::Overflow,
                                    _ => Unavailable::InvalidValue,
                                },
                                terminal: true,
                            });
                            return std::future::pending::<Result<(), Failure>>().await;
                        }
                    }
                }
            }
        }
        let end = (Instant::now(), utc_now_ms()?);
        let elapsed = i64::try_from(end.0.saturating_duration_since(previous.0).as_millis())
            .map_err(|_| Failure::Unavailable)?;
        let wall = end.1.checked_sub(previous.1).ok_or(Failure::Unavailable)?;
        if clock_discontinuity(elapsed, wall, false)
            && let Some(view) = latest.borrow().as_ref()
        {
            epoch_utc = end
                .1
                .checked_sub(
                    i64::try_from(source_clock.0.elapsed().as_millis())
                        .map_err(|_| Failure::Unavailable)?,
                )
                .ok_or(Failure::Unavailable)?;
            view.schedule_loss(ScheduleLoss {
                epoch: Some(epoch_utc),
                reason: metten_engine::sources::Unavailable::MetadataChanged,
                terminal: false,
            });
            attempted_metadata = None;
        }
        previous = end;
        wait(
            &mut stop,
            Instant::now() + Duration::from_secs(2),
            Duration::from_secs(1),
        )
        .await?;
    }
}

fn clock_discontinuity(monotonic_ms: i64, utc_delta_ms: i64, idle_wait: bool) -> bool {
    let limit = MAX_INTAKE_AGE.as_millis() as u64;
    utc_delta_ms < 0
        || utc_delta_ms.abs_diff(monotonic_ms) > limit
        || (idle_wait && monotonic_ms as u64 > limit)
}

fn utc_now_ms() -> Result<i64, Failure> {
    i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| Failure::Unavailable)?
            .as_millis(),
    )
    .map_err(|_| Failure::Unavailable)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_market_installation_still_rejects_a_backward_clock() {
        let strategy = metten_engine::strategy::ValidatedStrategy::parse(include_str!(
            "../../../examples/learning.json"
        ))
        .unwrap();
        let mut worker = SourceWorker::prepare(&strategy, &[], memory::Memory::default()).unwrap();
        worker.advance(1, |_| Ok(())).unwrap();
        let token = worker.tokens()[metten_engine::sources::Stream::Metadata as usize];
        assert_eq!(
            worker.install_markets(&[], token, 0, |_| Ok(())),
            Err(Failure::Protocol)
        );
        assert!(worker.install_markets(&[], token, 1, |_| Ok(())).is_ok());
    }

    #[test]
    fn exhausted_native_epoch_permanently_revokes_views_without_wrapping() {
        let strategy = metten_engine::strategy::ValidatedStrategy::parse(include_str!(
            "../../../examples/learning.json"
        ))
        .unwrap();
        let worker = SourceWorker::prepare(&strategy, &[], memory::Memory::default()).unwrap();
        worker.gates[0].store(u64::MAX - 1, Ordering::SeqCst);
        let old = worker.snapshot(0).unwrap();
        revoke_gates(&worker.gates, &SOURCE_STREAMS[..1]).unwrap();
        assert!(!old.stream_current(metten_engine::sources::Stream::Book));
        assert!(old.snapshot().is_err());
        assert!(matches!(worker.snapshot(0), Err(Failure::Capacity)));
        assert_eq!(
            revoke_gates(&worker.gates, &SOURCE_STREAMS[..1]),
            Err(Failure::Capacity)
        );
    }
    #[test]
    fn pending_schedule_loss_cannot_be_relabelled_as_a_current_snapshot() {
        use metten_engine::sources::{Stream, Unavailable};
        let strategy = metten_engine::strategy::ValidatedStrategy::parse(include_str!(
            "../../../examples/learning.json"
        ))
        .unwrap();
        let mut worker = SourceWorker::prepare(
            &strategy,
            &super::super::tests::source_markets(),
            memory::Memory::default(),
        )
        .unwrap();
        worker
            .install_markets(
                &super::super::tests::source_markets(),
                worker.tokens()[Stream::Metadata as usize],
                0,
                |_| Ok(()),
            )
            .unwrap();
        let before = worker.snapshot(0).unwrap();
        assert!(
            before
                .snapshot()
                .unwrap()
                .markets
                .iter()
                .all(|m| m.verified)
        );
        before.schedule_loss(ScheduleLoss {
            epoch: Some(0),
            reason: Unavailable::MetadataChanged,
            terminal: false,
        });
        assert!(!before.stream_current(Stream::Book));
        assert!(before.stream_current(Stream::Brti));
        assert!(before.snapshot().is_err());
        assert!(
            matches!(worker.snapshot(0), Err(Failure::Stopped)),
            "new publication cannot capture the unapplied permission generation"
        );
        worker.observe_schedule_loss(0).unwrap();
        assert!(!before.stream_current(Stream::Metadata));
        let after = worker.snapshot(0).unwrap();
        assert!(
            after
                .snapshot()
                .unwrap()
                .markets
                .iter()
                .all(|m| !m.verified)
        );
        assert_eq!(after.epoch_utc_ms, 0);
    }
    #[tokio::test]
    async fn startup_checks_survive_recoverable_discovery_failure() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let transport = Arc::new(transport::Transport::controlled(
            listener.local_addr().unwrap(),
        ));
        let (first, received) = tokio::sync::oneshot::channel();
        let peer = tokio::spawn(async move {
            let mut first = Some(first);
            let mut paths = Vec::new();
            for _ in 0..2 {
                let (mut socket, _) = tokio::time::timeout(SHUTDOWN, listener.accept())
                    .await
                    .unwrap()
                    .unwrap();
                let mut request = Vec::new();
                let mut chunk = [0; 1024];
                while !request.ends_with(b"\r\n\r\n") {
                    let n = socket.read(&mut chunk).await.unwrap();
                    assert!(n > 0 && request.len() < 8192);
                    request.extend_from_slice(&chunk[..n]);
                }
                paths.push(
                    std::str::from_utf8(&request)
                        .unwrap()
                        .lines()
                        .next()
                        .unwrap()
                        .split_whitespace()
                        .nth(1)
                        .unwrap()
                        .to_owned(),
                );
                if paths.len() == 2 {
                    break;
                }
                let body = r#"{"exchange_active":false}"#;
                socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
                if let Some(first) = first.take() {
                    first.send(()).unwrap();
                }
            }
            paths
        });
        let worker = SourceWorker::prepare_display(transport.memory().clone()).unwrap();
        let clock = (worker.epoch, worker.epoch_utc_ms);
        let mut owner = Coordinator::new();
        let handle = owner.start_sources(worker).unwrap();
        handle.enable(0, true).unwrap();
        let prepared = handle.clone();
        tokio::task::spawn_blocking(move || prepared.prepare_fixture_markets())
            .await
            .unwrap()
            .unwrap();
        let latest = handle.latest.clone();
        let command = handle.command.clone();
        let (stop, stopped) = watch::channel(false);
        let task = tokio::spawn(async move {
            let mut validated = false;
            let result = run_metadata(
                &transport,
                latest,
                command,
                Fence::new(),
                stopped,
                clock,
                &mut validated,
            )
            .await;
            (result, validated)
        });
        received.await.unwrap();
        // Let the first failed attempt finish before triggering a fresh metadata-token check.
        tokio::time::sleep(Duration::from_millis(1100)).await;
        handle.schedule.send_replace(Some(ScheduleLoss {
            epoch: None,
            reason: metten_engine::sources::Unavailable::MetadataChanged,
            terminal: false,
        }));
        let paths = peer.await.unwrap();
        stop.send_replace(true);
        let (_, validated) = tokio::time::timeout(SHUTDOWN, task).await.unwrap().unwrap();
        owner.shutdown(Instant::now() + SHUTDOWN).await.unwrap();
        assert!(!validated);
        assert_eq!(
            paths,
            [
                "/trade-api/v2/exchange/status",
                "/trade-api/v2/exchange/status"
            ]
        );
    }

    #[test]
    fn ordinary_metadata_work_is_not_a_sleep_event_and_terminal_facts_are_not_lost() {
        use metten_engine::sources::Unavailable;
        assert!(!clock_discontinuity(6000, 6000, false));
        assert!(clock_discontinuity(6000, 6000, true));
        assert!(clock_discontinuity(100, 10000, false));
        assert!(clock_discontinuity(100, -1, false));
        let strategy = metten_engine::strategy::ValidatedStrategy::parse(include_str!(
            "../../../examples/learning.json"
        ))
        .unwrap();
        let worker = SourceWorker::prepare(&strategy, &[], memory::Memory::default()).unwrap();
        let view = worker.snapshot(0).unwrap();
        view.schedule_loss(ScheduleLoss {
            epoch: None,
            reason: Unavailable::Access,
            terminal: true,
        });
        view.schedule_loss(ScheduleLoss {
            epoch: Some(123),
            reason: Unavailable::MetadataChanged,
            terminal: false,
        });
        let loss = worker.schedule.borrow().unwrap();
        assert!(loss.terminal);
        assert_eq!(loss.reason, Unavailable::Access);
        assert_eq!(loss.epoch, Some(123));
    }
}
