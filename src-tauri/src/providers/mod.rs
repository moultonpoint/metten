//! Fixed native provider lifecycle, request policy and transport controls.
#![allow(dead_code)] // Controlled source/evaluator interfaces are also exercised directly by owner tests.
pub mod budget;
mod coinbase;
mod discovery;
mod kalshi;
mod kraken;
pub mod memory;
pub mod signing;
mod source;
#[cfg(test)]
mod tests;
pub mod transport;
use source::SourceCommand;
#[cfg(test)]
use source::{SOURCE_STREAMS, ScheduleLoss, source_envelope};
#[allow(unused_imports)] // Source view types remain available to controlled native callers.
pub use source::{SourceHandle, SourceView, SourceWorker};
mod wire;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use tokio::{
    sync::{mpsc, watch},
    task::JoinHandle,
    time::{Duration, Instant},
};

pub const MAX_BYTES: usize = 1024 * 1024;
pub const SHUTDOWN: Duration = Duration::from_secs(5);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    InvalidRequest,
    InvalidCredential,
    Signing,
    Missing,
    AlreadySaved,
    Unavailable,
    Ambiguous,
    Stopped,
    StillStopping,
    Deadline,
    Budget,
    Capacity,
    Protocol,
    Transport,
    Denied,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Environment {
    Demo,
    Production,
}
impl Environment {
    pub fn name(self) -> &'static str {
        match self {
            Self::Demo => "demo",
            Self::Production => "production",
        }
    }
    pub fn account(self) -> &'static str {
        match self {
            Self::Demo => "kalshi:demo",
            Self::Production => "kalshi:production",
        }
    }
    fn rest(self) -> &'static str {
        match self {
            Self::Demo => "https://demo-api.kalshi.co",
            Self::Production => "https://external-api.kalshi.com",
        }
    }
    fn socket(self) -> &'static str {
        match self {
            Self::Demo => "wss://demo-api.kalshi.co/trade-api/ws/v2",
            Self::Production => "wss://external-api-ws.kalshi.com/trade-api/ws/v2",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provider {
    Kalshi,
    Coinbase,
    Kraken,
}
impl Provider {
    fn index(self) -> usize {
        match self {
            Self::Kalshi => 0,
            Self::Coinbase => 1,
            Self::Kraken => 2,
        }
    }
}
#[derive(Clone, Debug)]
pub enum Route {
    ExchangeStatus,
    Series,
    Markets {
        min_close_ts: u64,
        max_close_ts: u64,
        cursor: Option<String>,
    },
    Market(String),
    Socket,
}
fn machine(s: &str) -> bool {
    !s.is_empty() && s.len() <= 128 && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}
impl Route {
    pub fn path(&self) -> Result<String, Failure> {
        Ok(match self {
            Self::ExchangeStatus => "/trade-api/v2/exchange/status".into(),
            Self::Series => "/trade-api/v2/series/KXBTC15M".into(),
            Self::Markets {
                min_close_ts,
                max_close_ts,
                cursor,
            } => {
                if min_close_ts >= max_close_ts
                    || max_close_ts - min_close_ts > 3600
                    || cursor.as_ref().is_some_and(|c| {
                        c.is_empty() || c.len() > 1024 || c.chars().any(char::is_control)
                    })
                {
                    return Err(Failure::InvalidRequest);
                }
                format!(
                    "/trade-api/v2/markets?series_ticker=KXBTC15M&min_close_ts={min_close_ts}&max_close_ts={max_close_ts}&limit=200{}",
                    cursor
                        .as_ref()
                        .map(|c| format!(
                            "&cursor={}",
                            c.bytes()
                                .map(|b| if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                                    (b as char).to_string()
                                } else {
                                    format!("%{b:02X}")
                                })
                                .collect::<String>()
                        ))
                        .unwrap_or_default()
                )
            }
            Self::Market(ticker) if machine(ticker) && ticker.starts_with("KXBTC15M-") => {
                format!("/trade-api/v2/markets/{ticker}")
            }
            Self::Socket => "/trade-api/ws/v2".into(),
            _ => return Err(Failure::InvalidRequest),
        })
    }
}
// Raw data owns no source truth. Step 4 must decode before engine admission.
pub struct Intake {
    pub provider: Provider,
    pub generation: u64,
    pub bytes: memory::Bytes,
    pub received_at: Instant,
    #[cfg(test)]
    pub receipt_ms: Option<u64>,
    pub source_tokens: Option<[metten_engine::sources::StreamToken; 6]>,
    progress: Option<watch::Sender<transport::Progress>>,
    completion: Option<tokio::sync::oneshot::Sender<Result<(), Failure>>>,
    fence: Fence,
}
impl Intake {
    pub fn new(
        provider: Provider,
        fence: &Fence,
        bytes: memory::Bytes,
        received_at: Instant,
    ) -> Self {
        Self {
            provider,
            generation: fence.token(),
            bytes,
            received_at,
            #[cfg(test)]
            receipt_ms: None,
            source_tokens: None,
            progress: None,
            completion: None,
            fence: fence.clone(),
        }
    }
}
// Buffering ceiling is independent of price validity; issue 16 owns later capacity measurement.
pub const MAX_INTAKE_AGE: Duration = Duration::from_secs(2);
#[derive(Clone)]
pub struct Fence {
    generation: Arc<AtomicU64>,
    stopped: Arc<AtomicBool>,
    signal: watch::Sender<bool>,
    worker_stop: watch::Sender<bool>,
    rejected: Arc<AtomicU64>,
    parent: Option<(Arc<Fence>, u64)>,
    cause: Arc<std::sync::Mutex<Option<Failure>>>,
}
impl Fence {
    fn new() -> Self {
        Self::with_worker_stop(watch::channel(false).0)
    }
    fn with_worker_stop(worker_stop: watch::Sender<bool>) -> Self {
        Self {
            generation: Arc::new(AtomicU64::new(1)),
            stopped: Arc::new(AtomicBool::new(false)),
            signal: watch::channel(false).0,
            worker_stop,
            rejected: Arc::new(AtomicU64::new(0)),
            parent: None,
            cause: Arc::new(std::sync::Mutex::new(None)),
        }
    }
    pub fn session(&self) -> Result<Self, Failure> {
        if self.parent.is_some() || !self.current(self.token()) {
            return Err(Failure::Stopped);
        }
        let mut session = Self::with_worker_stop(self.worker_stop.clone());
        session.parent = Some((Arc::new(self.clone()), self.token()));
        Ok(session)
    }
    pub fn token(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }
    pub fn current(&self, token: u64) -> bool {
        !self.stopped.load(Ordering::SeqCst)
            && !*self.worker_stop.borrow()
            && token == self.token()
            && self
                .parent
                .as_ref()
                .is_none_or(|(parent, generation)| parent.current(*generation))
    }
    pub fn rejected(&self) -> u64 {
        self.rejected.load(Ordering::SeqCst)
    }
    fn reject(&self) {
        let _ = self
            .rejected
            .try_update(Ordering::SeqCst, Ordering::SeqCst, |count| {
                Some(count.saturating_add(1))
            });
    }
    pub async fn run<T>(
        &self,
        deadline: Instant,
        future: impl std::future::Future<Output = Result<T, Failure>>,
    ) -> Result<T, Failure> {
        let mut stopped = self.signal.subscribe();
        if !self.current(self.token()) {
            self.reject();
            return Err(Failure::Stopped);
        }
        let mut worker_stop = self.worker_stop.subscribe();
        let parent = async {
            if let Some((parent, _)) = &self.parent {
                let mut stop = parent.signal.subscribe();
                if !parent.current(parent.token()) {
                    return;
                }
                let _ = stop.changed().await;
            } else {
                std::future::pending::<()>().await;
            }
        };
        let result = tokio::select! { biased; _=parent=>Err(Failure::Stopped), result=async { tokio::select! { biased; _=worker_stop.changed()=>Err(Failure::Stopped), result=bounded(&mut stopped, deadline, future)=>result } }=>result };
        if !self.current(self.token()) {
            self.reject();
            Err(Failure::Stopped)
        } else {
            result
        }
    }
    fn fail(&self, error: Failure) {
        if let Ok(mut cause) = self.cause.lock() {
            *cause = Some(error)
        }
        self.revoke()
    }
    pub fn revoke(&self) {
        self.stopped.store(true, Ordering::SeqCst);
        self.signal.send_replace(true);
        let _ = self
            .generation
            .try_update(Ordering::SeqCst, Ordering::SeqCst, |g| g.checked_add(1));
    }
}
struct ProviderLifetime(Fence);
impl Drop for ProviderLifetime {
    fn drop(&mut self) {
        self.0.revoke();
    }
}
struct WorkerLifetime(watch::Sender<bool>);
impl Drop for WorkerLifetime {
    fn drop(&mut self) {
        self.0.send_replace(true);
    }
}
pub struct Coordinator {
    fences: [Fence; 3],
    stop: watch::Sender<bool>,
    provider_stop: [watch::Sender<bool>; 3],
    tasks: Vec<(Provider, JoinHandle<()>)>,
    worker: Option<std::thread::JoinHandle<()>>,
    stopping: bool,
}
impl Default for Coordinator {
    fn default() -> Self {
        Self::new()
    }
}
#[derive(Clone)]
pub struct Revocation {
    fences: [Fence; 3],
    stop: watch::Sender<bool>,
}
impl Revocation {
    pub fn revoke(&self) {
        self.stop.send_replace(true);
        for fence in &self.fences {
            fence.revoke();
        }
    }
}
impl Coordinator {
    pub fn revocation(&self) -> Revocation {
        Revocation {
            fences: self.fences.clone(),
            stop: self.stop.clone(),
        }
    }
    pub fn active(&self, provider: Provider) -> bool {
        self.tasks
            .iter()
            .any(|(p, task)| *p == provider && !task.is_finished())
    }

    pub fn new() -> Self {
        let (stop, _) = watch::channel(false);
        Self {
            fences: std::array::from_fn(|_| Fence::with_worker_stop(stop.clone())),
            stop,
            provider_stop: std::array::from_fn(|_| watch::channel(false).0),
            tasks: Vec::new(),
            worker: None,
            stopping: false,
        }
    }
    pub fn fence(&self, provider: Provider) -> Fence {
        self.fences[provider.index()].clone()
    }
    pub fn cancellation(&self, provider: Provider) -> watch::Receiver<bool> {
        self.provider_stop[provider.index()].subscribe()
    }
    pub fn spawn(
        &mut self,
        provider: Provider,
        work: impl std::future::Future<Output = ()> + Send + 'static,
    ) -> Result<(), Failure> {
        if self.stopping || *self.stop.borrow() {
            return Err(Failure::Stopped);
        }
        if self.tasks.len() >= 3 || self.tasks.iter().any(|(owned, _)| *owned == provider) {
            return Err(Failure::Capacity);
        }
        let lifetime = ProviderLifetime(self.fence(provider));
        self.tasks.push((
            provider,
            tokio::spawn(async move {
                let _lifetime = lifetime;
                work.await;
            }),
        ));
        Ok(())
    }
    pub fn start_worker(
        &mut self,
        mut receiver: mpsc::Receiver<Intake>,
        mut consume: impl FnMut(Intake) + Send + 'static,
    ) -> Result<(), Failure> {
        if self.stopping || self.worker.is_some() {
            return Err(Failure::Stopped);
        }
        let stop = self.stop.subscribe();
        let lifetime = WorkerLifetime(self.stop.clone());
        self.worker = Some(std::thread::spawn(move || {
            let _lifetime = lifetime;
            while !*stop.borrow() {
                match receiver.try_recv() {
                    Ok(item) if item.fence.current(item.generation) => {
                        if Instant::now().saturating_duration_since(item.received_at)
                            > MAX_INTAKE_AGE
                        {
                            item.fence.revoke();
                            continue;
                        }
                        consume(item)
                    }
                    Ok(item) => item.fence.reject(),
                    Err(mpsc::error::TryRecvError::Empty) => {
                        std::thread::sleep(Duration::from_millis(5))
                    }
                    Err(mpsc::error::TryRecvError::Disconnected) => return,
                }
            }
        }));
        Ok(())
    }
    pub async fn stop_provider(
        &mut self,
        provider: Provider,
        deadline: Instant,
    ) -> Result<(), Failure> {
        self.fences[provider.index()].revoke();
        self.provider_stop[provider.index()].send_replace(true);
        if let Some(index) = self.tasks.iter().position(|(owned, _)| *owned == provider) {
            if tokio::time::timeout_at(deadline, &mut self.tasks[index].1)
                .await
                .is_err()
            {
                self.tasks[index].1.abort();
                return Err(Failure::StillStopping);
            }
            drop(self.tasks.remove(index));
        }
        Ok(())
    }
    pub fn revoke(&mut self) {
        self.stopping = true;
        for fence in &self.fences {
            fence.revoke();
        }
        self.stop.send_replace(true);
        for stop in &self.provider_stop {
            stop.send_replace(true);
        }
    }
    pub fn is_stopping(&self) -> bool {
        self.stopping || *self.stop.borrow()
    }
    pub fn finished(&self) -> bool {
        self.tasks.iter().all(|(_, task)| task.is_finished())
            && self.worker.as_ref().is_none_or(|w| w.is_finished())
    }
    pub async fn shutdown(&mut self, deadline: Instant) -> Result<(), Failure> {
        self.revoke();
        while let Some((_, task)) = self.tasks.first_mut() {
            match tokio::time::timeout_at(deadline, &mut *task).await {
                Ok(_) => {
                    drop(self.tasks.remove(0));
                }
                Err(_) => break,
            }
        }
        while self.worker.as_ref().is_some_and(|w| !w.is_finished()) && Instant::now() < deadline {
            tokio::time::sleep_until((Instant::now() + Duration::from_millis(5)).min(deadline))
                .await;
        }
        if !self.finished() {
            for (_, task) in &self.tasks {
                if !task.is_finished() {
                    task.abort();
                }
            }
            return Err(Failure::StillStopping);
        }
        self.tasks.clear();
        if let Some(worker) = self.worker.take() {
            worker.join().map_err(|_| Failure::Unavailable)?;
        }
        Ok(())
    }
    pub fn recover_provider(&mut self, provider: Provider, quitting: bool) -> Result<(), Failure> {
        if quitting
            || self.stopping
            || *self.stop.borrow()
            || self.tasks.iter().any(|(owned, _)| *owned == provider)
        {
            return Err(Failure::StillStopping);
        }
        self.fences[provider.index()] = Fence::with_worker_stop(self.stop.clone());
        self.provider_stop[provider.index()] = watch::channel(false).0;
        Ok(())
    }
    pub fn recover(&mut self, quitting: bool) -> Result<(), Failure> {
        if quitting || !self.finished() {
            return Err(Failure::StillStopping);
        }
        // Completed handles must first be observed by shutdown, never detached.
        if !self.tasks.is_empty() || self.worker.is_some() {
            return Err(Failure::StillStopping);
        }
        self.fences = std::array::from_fn(|_| Fence::with_worker_stop(self.stop.clone()));
        self.stop.send_replace(false);
        self.provider_stop = std::array::from_fn(|_| watch::channel(false).0);
        self.stopping = false;
        Ok(())
    }
}
pub(super) const RAW_SLOTS: usize = 8;

pub fn intake() -> (mpsc::Sender<Intake>, mpsc::Receiver<Intake>) {
    mpsc::channel(RAW_SLOTS)
}
// The queue wakes its existing owner; no second receiver or source scheduler.
pub(super) struct IntakeWake {
    notified: AtomicBool,
    worker: std::thread::Thread,
}
impl Default for IntakeWake {
    fn default() -> Self {
        Self {
            notified: AtomicBool::new(false),
            worker: std::thread::current(),
        }
    }
}
impl IntakeWake {
    pub(super) fn wait(&self, duration: Duration) {
        // A separate sticky flag survives other waits consuming the thread's park token.
        if !self.notified.swap(false, Ordering::Acquire) {
            std::thread::park_timeout(duration);
        }
    }
}
impl std::task::Wake for IntakeWake {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.notified.store(true, Ordering::Release);
        self.worker.unpark();
    }
}
#[cfg(test)]
pub(super) fn intake_waker() -> std::task::Waker {
    std::task::Waker::from(Arc::new(IntakeWake::default()))
}
pub(super) fn poll_intake(
    rx: &mut mpsc::Receiver<Intake>,
    waker: &std::task::Waker,
) -> Result<Intake, mpsc::error::TryRecvError> {
    match rx.poll_recv(&mut std::task::Context::from_waker(waker)) {
        std::task::Poll::Ready(Some(item)) => Ok(item),
        std::task::Poll::Ready(None) => Err(mpsc::error::TryRecvError::Disconnected),
        std::task::Poll::Pending => Err(mpsc::error::TryRecvError::Empty),
    }
}
pub fn admit(sender: &mpsc::Sender<Intake>, fence: &Fence, item: Intake) -> Result<(), Failure> {
    if !Arc::ptr_eq(&fence.generation, &item.fence.generation) || !fence.current(item.generation) {
        fence.reject();
        return Err(Failure::Stopped);
    }
    if item.bytes.len() > MAX_BYTES {
        fence.revoke();
        return Err(Failure::Capacity);
    }
    sender.try_send(item).map_err(|_| {
        fence.revoke();
        Failure::Capacity
    })
}
pub async fn bounded<T>(
    stop: &mut watch::Receiver<bool>,
    deadline: Instant,
    future: impl std::future::Future<Output = Result<T, Failure>>,
) -> Result<T, Failure> {
    if *stop.borrow() {
        return Err(Failure::Stopped);
    }
    if Instant::now() >= deadline {
        return Err(Failure::Deadline);
    }
    let result = tokio::select! { biased; _ = stop.changed() => Err(Failure::Stopped), result = tokio::time::timeout_at(deadline, future) => result.map_err(|_| Failure::Deadline)? };
    if *stop.borrow() {
        Err(Failure::Stopped)
    } else if Instant::now() >= deadline {
        Err(Failure::Deadline)
    } else {
        result
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BookAction {
    Add,
    Delete,
    Snapshot,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Book,
    Trade,
    Brti,
    Lifecycle,
}
#[derive(Clone, PartialEq, Eq)]
pub enum Control {
    KalshiSubscribe {
        id: u32,
        channel: Channel,
        markets: Vec<String>,
    },
    KalshiBookUpdate {
        id: u32,
        sid: u32,
        action: BookAction,
        markets: Vec<String>,
        token: Option<metten_engine::sources::StreamToken>,
    },
    KalshiTradeUpdate {
        id: u32,
        sid: u32,
        action: BookAction,
        markets: Vec<String>,
        token: metten_engine::sources::StreamToken,
    },
    KalshiIndexes {
        id: u32,
        sid: u32,
        token: Option<metten_engine::sources::StreamToken>,
    },
    KalshiUnsubscribe {
        id: u32,
        subscriptions: Vec<u32>,
    },
    Coinbase {
        subscribe: bool,
    },
    Kraken {
        subscribe: bool,
    },
}
impl Control {
    pub fn encode(&self, provider: Provider) -> Result<String, Failure> {
        use serde_json::json;
        let value = match (provider, self) {
            (
                Provider::Kalshi,
                Self::KalshiSubscribe {
                    id,
                    channel,
                    markets,
                },
            ) if *id > 0 && markets.len() <= 2 => {
                for market in markets {
                    if !machine(market) || !market.starts_with("KXBTC15M-") {
                        return Err(Failure::InvalidRequest);
                    }
                }
                let (name, scoped) = match channel {
                    Channel::Book => ("orderbook_delta", true),
                    Channel::Trade => ("trade", true),
                    Channel::Brti => ("cfbenchmarks_value", false),
                    Channel::Lifecycle => ("market_lifecycle_v2", false),
                };
                if scoped == markets.is_empty() {
                    return Err(Failure::InvalidRequest);
                }
                let params = if scoped {
                    json!({"channels":[name],"market_tickers":markets})
                } else if matches!(channel, Channel::Brti) {
                    json!({"channels":[name],"index_ids":["BRTI"]})
                } else {
                    json!({"channels":[name]})
                };
                json!({"id":id,"cmd":"subscribe","params":params})
            }
            (
                Provider::Kalshi,
                Self::KalshiBookUpdate {
                    id,
                    sid,
                    action,
                    markets,
                    ..
                }
                | Self::KalshiTradeUpdate {
                    id,
                    sid,
                    action,
                    markets,
                    ..
                },
            ) if *id > 0
                && *sid > 0
                && !markets.is_empty()
                && markets.len() <= 2
                && markets
                    .iter()
                    .all(|m| machine(m) && m.starts_with("KXBTC15M-")) =>
            {
                if matches!(
                    self,
                    Self::KalshiTradeUpdate {
                        action: BookAction::Snapshot,
                        ..
                    }
                ) {
                    return Err(Failure::InvalidRequest);
                }
                let action = match action {
                    BookAction::Add => "add_markets",
                    BookAction::Delete => "delete_markets",
                    BookAction::Snapshot => "get_snapshot",
                };
                json!({"id":id,"cmd":"update_subscription","params":{"sid":sid,"action":action,"market_tickers":markets}})
            }
            (Provider::Kalshi, Self::KalshiIndexes { id, sid, .. }) if *id > 0 && *sid > 0 => {
                json!({"id":id,"cmd":"update_subscription","params":{"sid":sid,"action":"indexlist"}})
            }
            (Provider::Kalshi, Self::KalshiUnsubscribe { id, subscriptions })
                if *id > 0
                    && !subscriptions.is_empty()
                    && subscriptions.len() <= 4
                    && subscriptions.iter().all(|s| *s > 0) =>
            {
                json!({"id":id,"cmd":"unsubscribe","params":{"sids":subscriptions}})
            }
            (Provider::Coinbase, Self::Coinbase { subscribe }) => {
                json!({"type":if *subscribe {"subscribe"} else {"unsubscribe"},"product_ids":["BTC-USD"],"channels":["matches","heartbeat"]})
            }
            (Provider::Kraken, Self::Kraken { subscribe }) => {
                if *subscribe {
                    json!({"method":"subscribe","params":{"channel":"trade","symbol":["BTC/USD"],"snapshot":true}})
                } else {
                    json!({"method":"unsubscribe","params":{"channel":"trade","symbol":["BTC/USD"]}})
                }
            }
            _ => return Err(Failure::InvalidRequest),
        };
        serde_json::to_string(&value).map_err(|_| Failure::InvalidRequest)
    }
}

pub async fn wait(
    stop: &mut watch::Receiver<bool>,
    deadline: Instant,
    delay: Duration,
) -> Result<(), Failure> {
    bounded(stop, deadline, async {
        tokio::time::sleep(delay).await;
        Ok(())
    })
    .await
}
