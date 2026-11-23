use metten_engine::{Engine, paper::*, sources as source, strategy};
use serde_json::{Value, json};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
const SCENARIO: &str = include_str!("../../examples/paper-learning-scenario.json");

pub fn decode(body: &tauri::ipc::InvokeBody) -> Result<Request, Failure> {
    let tauri::ipc::InvokeBody::Json(value) = body else {
        return Err(Failure::new("invalid_request"));
    };
    let request = decode_request(value)?;
    bounded_response(value, 16384)?;
    Ok(request)
}
#[derive(Default)]
pub struct NativePaper {
    stop: Arc<AtomicBool>,
    playback: Option<Playback>,
    manual: bool,
    manual_driver: Option<Driver>,
}
struct Playback {
    run_id: i64,
    stop: Arc<AtomicBool>,
    error: Arc<Mutex<Option<&'static str>>>,
    thread: Option<thread::JoinHandle<()>>,
}
impl Playback {
    fn join_if_finished(&mut self) -> bool {
        if self
            .thread
            .as_ref()
            .is_some_and(|thread| !thread.is_finished())
        {
            return false;
        }
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            *self.error.lock().unwrap() = Some("unconfirmed");
        }
        true
    }
}
impl Drop for Playback {
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            self.stop.store(true, Ordering::SeqCst);
            let _ = thread.join();
        }
    }
}
impl NativePaper {
    pub fn stop_signal(&self) -> Arc<AtomicBool> {
        self.stop.clone()
    }
    pub fn stop_owner(owner: &Mutex<Self>, deadline: Instant) -> bool {
        loop {
            match owner.try_lock() {
                Ok(mut state) => return state.stop(deadline),
                Err(std::sync::TryLockError::Poisoned(_)) => return false,
                Err(std::sync::TryLockError::WouldBlock) => (),
            }
            if Instant::now() >= deadline {
                return false;
            }
            thread::sleep(Duration::from_millis(2));
        }
    }
    pub fn stop(&mut self, deadline: Instant) -> bool {
        self.stop.store(true, Ordering::SeqCst);
        self.manual_driver.take();
        let Some(playback) = self.playback.as_mut() else {
            return true;
        };
        while !playback.join_if_finished() {
            if Instant::now() >= deadline {
                return false;
            }
            thread::sleep(Duration::from_millis(2));
        }
        true
    }
    fn begin(
        &mut self,
        engine: Arc<Engine>,
        reply: &Reply,
        scenario: Scenario,
    ) -> Result<(), Failure> {
        let id = positive_id(&reply.summary.run_id)?;
        if self
            .manual_driver
            .as_ref()
            .is_some_and(|driver| driver.summary.run_id == reply.summary.run_id)
            || self.playback.as_ref().is_some_and(|p| p.run_id == id)
        {
            return Ok(());
        }
        if self
            .playback
            .as_mut()
            .is_some_and(|p| !p.join_if_finished())
        {
            return Err(Failure::new("busy"));
        }
        let Some(context) = reply.context else {
            return Ok(());
        };
        if reply.summary.control_state == "stopped" || reply.summary.control_state == "unresolved" {
            return Ok(());
        }
        let stop = self.stop.clone();
        let stopped = stop.clone();
        let error = Arc::new(Mutex::new(None));
        let failed = error.clone();
        let driver = Driver::new(
            engine.clone(),
            context,
            reply.bindings.clone(),
            reply.summary.clone(),
            scenario,
        );
        if self.manual {
            self.manual_driver = Some(driver);
            return Ok(());
        }

        let thread = thread::Builder::new()
            .name("metten-paper-playback".into())
            .spawn(move || {
                let result = (|| {
                    let mut driver = driver;
                    driver.prepare(&stopped)?;
                    while !stopped.load(Ordering::SeqCst)
                        && driver.summary.control_state != "stopped"
                    {
                        thread::sleep(Duration::from_millis(50));
                        if stopped.load(Ordering::SeqCst) {
                            break;
                        }
                        let next = driver
                            .virtual_ms
                            .checked_add(1000)
                            .ok_or_else(|| Failure::new("resource_limit"))?;
                        driver.advance(next, &stopped)?;
                    }
                    Ok::<(), Failure>(())
                })();
                if let Err(reason) = result
                    && reason.code != "stopped_context"
                {
                    *failed.lock().unwrap() = Some(reason.code);
                    let _ = engine.paper_control(id, Control::Pause);
                }
            })
            .map_err(|_| Failure::new("resource_limit"))?;
        self.playback = Some(Playback {
            run_id: id,
            stop,
            error,
            thread: Some(thread),
        });
        Ok(())
    }
}

pub fn admit_resume(engine: &Engine, request: &mut Request) {
    if let Request::Resume {
        admitted_generation,
        ..
    } = request
    {
        *admitted_generation = Some(engine.paper_effective().2);
    }
}

pub fn dispatch(
    engine: Arc<Engine>,
    state: &mut NativePaper,
    request: Request,
) -> Result<Value, Failure> {
    let mut value = match request {
        Request::Start {
            start_uuid,
            version_id,
            settings,
        } => {
            let scenario = Scenario::parse(SCENARIO)?;
            let utc = i64::try_from(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(|_| Failure::new("range"))?
                    .as_millis(),
            )
            .map_err(|_| Failure::new("range"))?;
            let start = scenario.start(start_uuid, version_id, settings, utc)?;
            if state
                .playback
                .as_mut()
                .is_some_and(|playback| !playback.join_if_finished())
                && engine
                    .paper_latest()?
                    .is_none_or(|run| run.start_uuid != start.start_uuid)
            {
                return Err(Failure::new("busy"));
            }
            let reply = engine.paper_start(start)?;
            if let Err(error) = state.begin(engine.clone(), &reply, scenario) {
                if error.code != "busy" {
                    let _ = engine.paper_control(
                        positive_id(&reply.summary.run_id)?,
                        Control::StopAfterMarket,
                    );
                }
                return Err(error);
            }
            reply_value(reply)
        }
        Request::Pause { run_id } => reply_value(engine.paper_control(run_id, Control::Pause)?),
        Request::Resume {
            run_id,
            admitted_generation,
        } => reply_value(engine.paper_resume_admitted(
            run_id,
            admitted_generation.unwrap_or_else(|| engine.paper_effective().2),
        )?),
        Request::StopAfterMarket { run_id } => {
            reply_value(engine.paper_control(run_id, Control::StopAfterMarket)?)
        }
        Request::CheckSettlement { run_id } => {
            if state
                .manual_driver
                .as_ref()
                .is_some_and(|d| d.summary.control_state != "stopped")
            {
                return Err(Failure::new("run_active"));
            }
            if state
                .playback
                .as_mut()
                .is_some_and(|p| !p.join_if_finished())
            {
                return Err(Failure::new("run_active"));
            }
            reply_value(engine.paper_check_settlement(run_id)?)
        }
        Request::Snapshot => {
            let run = engine.paper_latest()?;
            let (paused, stop_after, generation) = engine.paper_effective();
            let error = state
                .playback
                .as_ref()
                .filter(|p| {
                    run.as_ref()
                        .is_some_and(|r| r.run_id == p.run_id.to_string())
                })
                .and_then(|p| *p.error.lock().unwrap());
            json!({"kind":"snapshot","run":run,"effective_paused":paused,"stop_after_market":stop_after,"effective_generation":generation.to_string(),"playback_error":error,"origin":"synthetic_controlled","clock_multiplier":"20"})
        }
        Request::History {
            run_id,
            before_paper_market_id,
        } => json!({"kind":"history","page":engine.paper_history(run_id,before_paper_market_id)?}),
        Request::Market {
            run_id,
            paper_market_id,
        } => {
            let run = engine.paper_summary(run_id)?;
            json!({"kind":"market","receipt":engine.paper_market(run_id,paper_market_id)?,"assumptions":{"origin":"synthetic_controlled","simulation_model":run.simulation_model,"fee_model":run.fee_model,"depth":simulation_depth(&run.simulation_model),"settlement":"identified synthetic binary outcome; no settlement fee"},"run":run})
        }
        Request::Evidence {
            run_id,
            paper_market_id,
            before_evidence_id,
        } => {
            json!({"kind":"evidence","page":engine.paper_evidence(run_id,paper_market_id,before_evidence_id)?})
        }
    };
    if value["kind"] == "snapshot" {
        let (paused, stop_after, generation) = engine.paper_effective();
        value["effective_paused"] = json!(paused);
        value["stop_after_market"] = json!(stop_after);
        value["effective_generation"] = json!(generation.to_string());
    }
    if value["kind"] == "market" {
        bounded_response(&value, 64 * 1024)?;
    }
    bounded_response(&value, 256 * 1024)?;
    Ok(value)
}
fn reply_value(reply: Reply) -> Value {
    json!({"kind":"snapshot","run":reply.summary,"effective_paused":reply.effective_paused,"effective_generation":reply.effective_generation.to_string(),"completion_revision":reply.completion_revision,"origin":"synthetic_controlled","clock_multiplier":"20"})
}

struct Driver {
    engine: Arc<Engine>,
    context: InputContext,
    bindings: Vec<strategy::BindingDescriptor>,
    summary: RunSummary,
    scenario: Scenario,
    ordinal: i64,
    virtual_ms: u64,
}
impl Driver {
    fn new(
        engine: Arc<Engine>,
        context: InputContext,
        bindings: Vec<strategy::BindingDescriptor>,
        summary: RunSummary,
        scenario: Scenario,
    ) -> Self {
        Self {
            engine,
            context,
            bindings,
            summary,
            scenario,
            ordinal: 0,
            virtual_ms: 0,
        }
    }
    fn submit(&mut self, at: u64, payload: Payload, stop: &AtomicBool) -> Result<(), Failure> {
        let input = Input {
            context: self.context,
            ordinal: self
                .ordinal
                .checked_add(1)
                .ok_or_else(|| Failure::new("resource_limit"))?,
            virtual_ms: at,
            payload,
        };
        let mut attempts = 0;
        let reply = loop {
            if stop.load(Ordering::SeqCst) {
                return Err(Failure::new("stopped_context"));
            }
            match self.engine.paper_input(input.clone()) {
                Err(error) if error.code == "busy" && attempts < 100 => {
                    attempts += 1;
                    thread::sleep(Duration::from_millis(5));
                }
                result => break result?,
            }
        };
        self.ordinal = input.ordinal;
        self.context = reply
            .context
            .ok_or_else(|| Failure::new("stopped_context"))?;
        self.summary = reply.summary;
        self.virtual_ms = at;
        Ok(())
    }
    fn event(
        &mut self,
        at: u64,
        event: source::SourceEvent,
        stop: &AtomicBool,
    ) -> Result<(), Failure> {
        self.submit(
            at,
            Payload::Source(source::Envelope {
                clock: source::Clock {
                    at_ms: at,
                    receipt_ms: at,
                    received_through_ms: at,
                    completed_frontier_ms: self
                        .summary
                        .last_evaluated_through_ms
                        .parse()
                        .map_err(|_| Failure::new("storage"))?,
                },
                provider_identity: Some("synthetic_controlled".into()),
                provider_time_utc: None,
                event,
            }),
            stop,
        )
    }
    fn prepare(&mut self, stop: &AtomicBool) -> Result<(), Failure> {
        self.event(
            0,
            source::SourceEvent::Connection {
                provider: source::Provider::Kalshi,
                token: self.context.book_token,
                state: source::Connection::Open,
            },
            stop,
        )?;
        let markets = self
            .scenario
            .markets
            .iter()
            .map(|m| m.source_market_id.clone())
            .collect::<Vec<_>>();
        for market_id in markets {
            self.event(
                0,
                source::SourceEvent::Metadata {
                    token: self.context.metadata_token,
                    market_id,
                    verified: true,
                },
                stop,
            )?;
        }
        self.observations(0, stop)
    }
    fn observations(&mut self, at: u64, stop: &AtomicBool) -> Result<(), Failure> {
        let mut updates = Vec::new();
        let mut books = Vec::new();
        for observation in self.scenario.observations.iter().filter(|o| o.at_ms == at) {
            let initialized = self
                .scenario
                .observations
                .iter()
                .any(|o| o.source_market_id == observation.source_market_id && o.at_ms < at);
            let number = self
                .scenario
                .observations
                .iter()
                .filter(|o| {
                    o.source_market_id == observation.source_market_id
                        && o.at_ms > 0
                        && o.at_ms <= at
                })
                .count() as u64;
            for binding in &self.bindings {
                let strategy::BindingScope::Contract {
                    market_id,
                    contract_id,
                    side,
                } = &binding.scope
                else {
                    return Err(Failure::new("unsupported_input"));
                };
                if *market_id != observation.source_market_id {
                    continue;
                }
                let price = strategy::Decimal::parse(if *side == strategy::Side::Yes {
                    &observation.yes_ask
                } else {
                    &observation.no_ask
                })
                .map_err(|_| Failure::new("range"))?;
                updates.push(source::BindingUpdate {
                    binding: binding.clone(),
                    unit: strategy::RuleUnit::ContractCents,
                    value: Some(if initialized {
                        source::ValueUpdate::Observation {
                            number,
                            value: price,
                        }
                    } else {
                        source::ValueUpdate::Initialize(price)
                    }),
                    valid_until_ms: Some(observation.valid_until_ms),
                });
                books.push(Book {
                    source_market_id: market_id.clone(),
                    source_contract_id: contract_id.clone(),
                    side: *side,
                    binding_id: binding.binding_id,
                    source_generation: self.context.generation,
                    value_transaction: 0,
                    observation_number: number,
                    ask: price,
                    available_millionths: observation
                        .available_millionths
                        .parse()
                        .map_err(|_| Failure::new("range"))?,
                    accepted_ms: at,
                    valid_until_ms: observation.valid_until_ms,
                });
            }
        }
        if !updates.is_empty() {
            self.event(
                at,
                source::SourceEvent::Update {
                    stream: source::Stream::Book,
                    token: self.context.book_token,
                    updates,
                    coverage: Vec::new(),
                },
                stop,
            )?;
            for mut book in books {
                book.value_transaction = self.context.last_value_transaction;
                book.source_generation = self.context.generation;
                self.submit(at, Payload::Book(book), stop)?;
            }
        }
        Ok(())
    }
    fn advance(&mut self, target: u64, stop: &AtomicBool) -> Result<(), Failure> {
        let mut times = self
            .scenario
            .observations
            .iter()
            .map(|o| o.at_ms)
            .chain(
                self.scenario
                    .markets
                    .iter()
                    .filter_map(|m| u64::try_from(m.open_ms).ok()),
            )
            .chain(self.scenario.settlements.iter().map(|s| s.available_ms))
            .filter(|at| *at > self.virtual_ms && *at <= target)
            .collect::<Vec<_>>();
        times.sort_unstable();
        times.dedup();
        for at in times {
            let boundaries = self
                .scenario
                .markets
                .iter()
                .filter(|m| m.open_ms == at as i64)
                .map(|m| m.source_market_id.clone())
                .collect::<Vec<_>>();
            for market_id in boundaries {
                self.event(at, source::SourceEvent::Boundary { market_id }, stop)?;
            }
            self.observations(at, stop)?;
            let settlements = self
                .scenario
                .settlements
                .iter()
                .filter(|s| s.available_ms == at)
                .cloned()
                .collect::<Vec<_>>();
            for settlement in settlements {
                self.submit(at, Payload::Settlement(settlement), stop)?;
            }
            if self.summary.control_state == "stopped" {
                return Ok(());
            }
        }
        if target > self.virtual_ms {
            self.submit(target, Payload::Advance, stop)?;
        }
        Ok(())
    }
}

impl NativePaper {
    fn manual() -> Self {
        Self {
            manual: true,
            ..Self::default()
        }
    }
    fn advance_manual(&mut self, target: u64, stop: &AtomicBool) -> Result<(), Failure> {
        let driver = self
            .manual_driver
            .as_mut()
            .ok_or_else(|| Failure::new("stopped_context"))?;
        if target < driver.virtual_ms || target - driver.virtual_ms > 1000 {
            return Err(Failure::new("range"));
        }
        if driver.ordinal == 0 {
            driver.prepare(stop)?;
        }
        driver.advance(target, stop)
    }
}

pub fn check_entry() -> Option<i32> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new("--paper-check")) {
        return None;
    }
    let result = (|| {
        let path = args.next().ok_or_else(|| Failure::new("invalid_request"))?;
        if args.next().is_some() {
            return Err(Failure::new("invalid_request"));
        }
        let mut stop = StopInput::new()?;
        let value = check(std::path::Path::new(&path), &mut || stop.check())?;
        println!("{}", value);
        Ok::<(), Failure>(())
    })();
    Some(match result {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("paper_check_failed: {}", error.code);
            1
        }
    })
}

struct StopInput {
    flags: i32,
    bytes: Vec<u8>,
}
impl StopInput {
    fn new() -> Result<Self, Failure> {
        // Headless entry owns stdin exclusively; no Tauri or credential setup has run.
        let flags = unsafe { libc::fcntl(libc::STDIN_FILENO, libc::F_GETFL) };
        if flags < 0
            || unsafe { libc::fcntl(libc::STDIN_FILENO, libc::F_SETFL, flags | libc::O_NONBLOCK) }
                < 0
        {
            return Err(Failure::new("unavailable"));
        }
        Ok(Self {
            flags,
            bytes: Vec::new(),
        })
    }
    fn check(&mut self) -> Result<(), Failure> {
        let mut bytes = [0_u8; 64];
        let read =
            unsafe { libc::read(libc::STDIN_FILENO, bytes.as_mut_ptr().cast(), bytes.len()) };
        if read > 0 {
            self.bytes.extend_from_slice(&bytes[..read as usize]);
            if self.bytes.len() > 64 {
                return Err(Failure::new("invalid_request"));
            }
            if self.bytes.ends_with(b"\n") {
                return Err(Failure::new(if self.bytes == b"STOP\n" {
                    "canceled"
                } else {
                    "invalid_request"
                }));
            }
        } else if read < 0
            && std::io::Error::last_os_error().kind() != std::io::ErrorKind::WouldBlock
        {
            return Err(Failure::new("unavailable"));
        }
        Ok(())
    }
}
impl Drop for StopInput {
    fn drop(&mut self) {
        unsafe { libc::fcntl(libc::STDIN_FILENO, libc::F_SETFL, self.flags) };
    }
}

fn check(
    path: &std::path::Path,
    cancel: &mut dyn FnMut() -> Result<(), Failure>,
) -> Result<Value, Failure> {
    use std::os::unix::fs::MetadataExt;
    let metadata =
        std::fs::symlink_metadata(path).map_err(|_| Failure::new("invalid_workspace"))?;
    if !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || metadata.uid() != unsafe { libc::getuid() }
        || metadata.mode() & 0o077 != 0
        || std::fs::read_dir(path)
            .map_err(|_| Failure::new("invalid_workspace"))?
            .next()
            .is_some()
    {
        return Err(Failure::new("invalid_workspace"));
    }
    cancel()?;
    let engine =
        Arc::new(Engine::open(path.join("session.sqlite")).map_err(|_| Failure::new("storage"))?);
    let text = include_str!("../../examples/learning.json");
    let version = match engine
        .strategies(metten_engine::StrategyRequest::Save {
            json: text.into(),
            reviewed_json: text.into(),
        })
        .map_err(|_| Failure::new("storage"))?
    {
        metten_engine::StrategyResponse::Saved { version } => version.id,
        _ => return Err(Failure::new("unconfirmed")),
    };
    let mut state = NativePaper::manual();
    let invoke = |engine: Arc<Engine>, state: &mut NativePaper, body: Value| {
        let request = decode(&tauri::ipc::InvokeBody::Json(body))?;
        dispatch(engine, state, request)
    };
    let first = invoke(
        engine.clone(),
        &mut state,
        json!({"operation":"start","start_uuid":"00000000-0000-4000-8000-000000000001","version_id":version,"settings":{"quantity":"10","minimum_price":"0","maximum_price":"100","starting_balance":"100"}}),
    )?;
    let run = first["run"]["run_id"]
        .as_str()
        .ok_or_else(|| Failure::new("unconfirmed"))?
        .to_string();
    let stopped = AtomicBool::new(false);
    for at in (0..=61000).step_by(1000) {
        cancel()?;
        state.advance_manual(at, &stopped)?;
    }
    let pending = invoke(engine.clone(), &mut state, json!({"operation":"snapshot"}))?;
    if pending["run"]["cash"] != "9736870000" {
        return Err(Failure::new("unconfirmed"));
    }
    let history = invoke(
        engine.clone(),
        &mut state,
        json!({"operation":"history","run_id":run}),
    )?;
    let rows = history["page"]["markets"]
        .as_array()
        .ok_or_else(|| Failure::new("unconfirmed"))?;
    if rows.len() != 1
        || rows[0]["source_market_id"] != "KXBTC15M-PAPER-NEXT"
        || rows[0]["side"] != "no"
        || rows[0]["filled_quantity"] != "10"
        || rows[0]["cost"] != "250000000"
        || rows[0]["fee"] != "13130000"
        || !rows[0]["settled_net"].is_null()
    {
        return Err(Failure::new("unconfirmed"));
    }
    let market = rows[0]["paper_market_id"]
        .as_str()
        .ok_or_else(|| Failure::new("unconfirmed"))?
        .to_string();
    cancel()?;
    engine.fence_paper_quit();
    state.stop(Instant::now() + Duration::from_secs(2));
    engine
        .stop_and_save()
        .map_err(|_| Failure::new("storage"))?;
    drop(state);
    drop(engine);
    let engine =
        Arc::new(Engine::open(path.join("session.sqlite")).map_err(|_| Failure::new("storage"))?);
    let mut state = NativePaper::manual();
    let reopen = invoke(engine.clone(), &mut state, json!({"operation":"snapshot"}))?;
    if reopen["run"]["cash"] != "9736870000"
        || reopen["run"]["control_state"] != "stopped"
        || reopen["run"]["virtual_frontier_ms"] != "61000"
    {
        return Err(Failure::new("unconfirmed"));
    }
    cancel()?;
    let settled = invoke(
        engine.clone(),
        &mut state,
        json!({"operation":"check_settlement","run_id":run}),
    )?;
    let duplicate = invoke(
        engine.clone(),
        &mut state,
        json!({"operation":"check_settlement","run_id":run}),
    )?;
    if settled != duplicate
        || settled["run"]["cash"] != "10736870000"
        || settled["run"]["virtual_frontier_ms"] != "901000"
        || settled["run"]["last_evaluated_through_ms"] != reopen["run"]["last_evaluated_through_ms"]
        || settled["run"]["completed_ordinal"] != reopen["run"]["completed_ordinal"]
    {
        return Err(Failure::new("unconfirmed"));
    }
    let receipt = invoke(
        engine.clone(),
        &mut state,
        json!({"operation":"market","run_id":run,"paper_market_id":market}),
    )?;
    if receipt["receipt"]["settled_net"] != "736870000"
        || receipt["assumptions"]["simulation_model"] != SIMULATION_MODEL
    {
        return Err(Failure::new("unconfirmed"));
    }
    let second = invoke(
        engine.clone(),
        &mut state,
        json!({"operation":"start","start_uuid":"00000000-0000-4000-8000-000000000002","version_id":version,"settings":{"quantity":"101","minimum_price":"0","maximum_price":"100","starting_balance":"100"}}),
    )?;
    let second_run = second["run"]["run_id"]
        .as_str()
        .ok_or_else(|| Failure::new("unconfirmed"))?
        .to_string();
    for at in (0..=901000).step_by(1000) {
        cancel()?;
        state.advance_manual(at, &stopped)?;
    }
    // 101 requested against 100 available: one partial IOC, then no newer observation to retry.
    let partial = invoke(
        engine.clone(),
        &mut state,
        json!({"operation":"history","run_id":second_run}),
    )?;
    let rows = partial["page"]["markets"]
        .as_array()
        .ok_or_else(|| Failure::new("unconfirmed"))?;
    if rows.len() != 1
        || rows[0]["attempt_state"] != "partial"
        || rows[0]["attempt_count"] != "1"
        || rows[0]["filled_quantity"] != "100"
        || rows[0]["cost"] != "2500000000"
        || rows[0]["fee"] != "131250000"
        || rows[0]["settled_net"] != "7368750000"
        || rows[0]["paper_market_id"] == market
        || engine
            .paper_latest()?
            .is_none_or(|r| r.cash != "17368750000")
    {
        return Err(Failure::new("unconfirmed"));
    }
    state.stop(Instant::now() + Duration::from_secs(2));
    engine
        .stop_and_save()
        .map_err(|_| Failure::new("storage"))?;
    Ok(
        json!({"protocol":"metten-paper-check-v1","kind":"paper_check","origin":"synthetic_controlled","pending_debit":"263130000","settled_cash":"10736870000","settled_net":"736870000","partial_fill_quantity":"100","checks":["native_decode_dispatch","shared_learning_evaluator","finite_full_partial","exact_fee_settlement","numeric_identity_history","stopped_reopen","settlement_idempotency"]}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::DirBuilderExt;
    struct Workspace(std::path::PathBuf);
    impl Workspace {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "metten-paper-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::DirBuilder::new()
                .mode(0o700)
                .create(&path)
                .unwrap();
            Self(path)
        }
    }
    impl Drop for Workspace {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn packaged_check_logic_uses_native_dispatch_and_shared_engine_without_accounts() {
        let workspace = Workspace::new();
        let result = check(&workspace.0, &mut || Ok(())).unwrap();
        assert_eq!(result["settled_cash"], "10736870000");
        assert_eq!(result["partial_fill_quantity"], "100");
    }
    #[test]
    fn headless_check_rejects_existing_and_symlink_workspaces_and_cancels_before_open() {
        let workspace = Workspace::new();
        assert_eq!(
            check(&workspace.0, &mut || Err(Failure::new("canceled")))
                .unwrap_err()
                .code,
            "canceled"
        );
        assert_eq!(std::fs::read_dir(&workspace.0).unwrap().count(), 0);
        let alias = workspace.0.join("alias");
        std::os::unix::fs::symlink(&workspace.0, &alias).unwrap();
        assert_eq!(
            check(&alias, &mut || Ok(())).unwrap_err().code,
            "invalid_workspace"
        );
        assert_eq!(
            check(&workspace.0, &mut || Ok(())).unwrap_err().code,
            "invalid_workspace"
        );
    }
    #[test]
    fn pause_after_native_resume_admission_survives_delayed_dispatch() {
        let workspace = Workspace::new();
        let engine = Arc::new(Engine::open(workspace.0.join("session.sqlite")).unwrap());
        let text = include_str!("../../examples/learning.json");
        let metten_engine::StrategyResponse::Saved { version } = engine
            .strategies(metten_engine::StrategyRequest::Save {
                json: text.into(),
                reviewed_json: text.into(),
            })
            .unwrap()
        else {
            panic!("save failed")
        };
        let mut state = NativePaper::manual();
        let first = dispatch(engine.clone(), &mut state, decode_request(&json!({"operation":"start","start_uuid":"00000000-0000-4000-8000-000000000001","version_id":version.id,"settings":{"quantity":"10","minimum_price":"0","maximum_price":"100","starting_balance":"100"}})).unwrap()).unwrap();
        let id = positive_id(first["run"]["run_id"].as_str().unwrap()).unwrap();
        engine.paper_control(id, Control::Pause).unwrap();
        let mut resume =
            decode_request(&json!({"operation":"resume","run_id":id.to_string()})).unwrap();
        admit_resume(&engine, &mut resume);
        let owner = Mutex::new(state);
        let held = owner.lock().unwrap();
        engine.fence_paper_control(id, Control::Pause).unwrap();
        drop(held);
        let result = dispatch(engine.clone(), &mut owner.lock().unwrap(), resume).unwrap();
        assert_eq!(result["effective_paused"], true);
        assert!(engine.paper_effective().0);
        let mut current =
            decode_request(&json!({"operation":"resume","run_id":id.to_string()})).unwrap();
        admit_resume(&engine, &mut current);
        let result = dispatch(engine.clone(), &mut owner.lock().unwrap(), current).unwrap();
        assert_eq!(result["effective_paused"], false);
    }

    #[test]
    fn completed_playback_does_not_cancel_the_next_run() {
        let workspace = Workspace::new();
        let engine = Arc::new(Engine::open(workspace.0.join("session.sqlite")).unwrap());
        let text = include_str!("../../examples/learning.json");
        let metten_engine::StrategyResponse::Saved { version } = engine
            .strategies(metten_engine::StrategyRequest::Save {
                json: text.into(),
                reviewed_json: text.into(),
            })
            .unwrap()
        else {
            panic!("save failed")
        };
        let start = |uuid| {
            decode_request(&json!({
            "operation":"start", "start_uuid":uuid, "version_id":version.id,
            "settings":{"quantity":"10","minimum_price":"0","maximum_price":"100","starting_balance":"100"},
        })).unwrap()
        };
        let mut state = NativePaper::default();
        let first = dispatch(
            engine.clone(),
            &mut state,
            start("00000000-0000-4000-8000-000000000001"),
        )
        .unwrap();
        let first_id = positive_id(first["run"]["run_id"].as_str().unwrap()).unwrap();
        engine
            .fence_paper_control(first_id, Control::StopAfterMarket)
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !state
            .playback
            .as_ref()
            .unwrap()
            .thread
            .as_ref()
            .unwrap()
            .is_finished()
        {
            assert!(Instant::now() < deadline, "first playback did not stop");
            thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(
            engine.paper_latest().unwrap().unwrap().control_state,
            "stopped"
        );
        let second = dispatch(
            engine.clone(),
            &mut state,
            start("00000000-0000-4000-8000-000000000002"),
        )
        .unwrap();
        assert_ne!(first["run"]["run_id"], second["run"]["run_id"]);
        assert!(!state.stop_signal().load(Ordering::SeqCst));
        let deadline = Instant::now() + Duration::from_secs(5);
        while engine.paper_latest().unwrap().unwrap().virtual_frontier_ms == "0" {
            assert!(Instant::now() < deadline, "second playback did not advance");
            thread::sleep(Duration::from_millis(2));
        }
        assert!(state.stop(Instant::now() + Duration::from_secs(5)));
    }
    #[test]
    fn quit_signals_and_times_out_while_paper_owner_is_busy() {
        let state = NativePaper::default();
        let stop = state.stop_signal();
        let owner = Arc::new(Mutex::new(state));
        let guard = owner.lock().unwrap();
        let waiting = owner.clone();
        let (sent, received) = std::sync::mpsc::channel();
        stop.store(true, Ordering::SeqCst);
        let worker = thread::spawn(move || {
            sent.send(NativePaper::stop_owner(&waiting, Instant::now()))
                .unwrap();
        });
        let result = received.recv_timeout(Duration::from_secs(1));
        assert!(guard.stop.load(Ordering::SeqCst));
        drop(guard);
        worker.join().unwrap();
        assert!(!result.unwrap());
        assert!(NativePaper::stop_owner(
            &owner,
            Instant::now() + Duration::from_secs(1)
        ));
    }
    #[test]
    fn playback_timeout_keeps_the_handle_until_actual_exit() {
        let gate = Arc::new(AtomicBool::new(false));
        let worker_gate = gate.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let mut state = NativePaper {
            stop: stop.clone(),
            playback: Some(Playback {
                run_id: 1,
                stop,
                error: Arc::new(Mutex::new(None)),
                thread: Some(thread::spawn(move || {
                    while !worker_gate.load(Ordering::SeqCst) {
                        thread::sleep(Duration::from_millis(1));
                    }
                })),
            }),
            ..Default::default()
        };
        assert!(!state.stop(Instant::now()));
        assert!(state.playback.as_ref().unwrap().thread.is_some());
        assert!(state.playback.as_ref().unwrap().stop.load(Ordering::SeqCst));
        gate.store(true, Ordering::SeqCst);
        assert!(state.stop(Instant::now() + Duration::from_secs(1)));
        assert!(state.playback.as_ref().unwrap().thread.is_none());
    }
    #[test]
    fn paper_decode_rejects_raw_unknown_live_source_and_noncanonical_ids() {
        for body in [
            json!({"operation":"snapshot","mode":"live"}),
            json!({"operation":"resume","run_id":"1","admitted_generation":0}),
            json!({"operation":"source","payload":{}}),
            json!({"operation":"market","run_id":"1","paper_market_id":"KXBTC15M-PAPER-NEXT"}),
            json!({"operation":"history","before_paper_market_id":"01"}),
            json!({"operation":"pause","run_id":1}),
            json!({"operation":"check_settlement","run_id":"1","result":"no"}),
        ] {
            assert!(decode(&tauri::ipc::InvokeBody::Json(body)).is_err());
        }
        assert!(decode(&tauri::ipc::InvokeBody::Raw(vec![])).is_err());
        assert!(decode(&tauri::ipc::InvokeBody::Json(json!({"operation":"evidence","run_id":"1","paper_market_id":"2","before_evidence_id":"3"}))).is_ok());
    }
}
