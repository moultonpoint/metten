//! A controlled non-trading session. No desktop, network, or exchange dependency.
pub mod paper;
pub mod sources;
mod storage;
pub mod strategy;
pub use strategy::{
    AdvanceBatch, Decision, Evaluator, RuleError, RunContext, SliceControl, SliceResult,
    ValidatedStrategy,
};

pub const STRATEGY_DOCUMENT_BYTES: usize = 256 * 1024;
pub const STRATEGY_DETAIL_BYTES: usize = 1024 * 1024;
pub const STRATEGY_PAGE_BYTES: usize = 32 * 1024;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct StrategyFailure {
    pub code: &'static str,
    pub message: &'static str,
}
impl StrategyFailure {
    pub fn new(code: &'static str) -> Self {
        Self {
            code,
            message: match code {
                "busy" => "Strategy action already in progress. Wait and retry.",
                "quitting" => "Quitting — wait for the stopped state to be saved.",
                "not_reviewed" => "Preview the current text before saving.",
                "not_found" => "This version was not found. Refresh saved versions.",
                "resource_limit" => {
                    "This strategy response is too large to show completely. Stored history is preserved."
                }
                "storage" => {
                    "Local strategy action failed. Inspect saved versions before retrying a save."
                }
                _ => "Invalid strategy request.",
            },
        }
    }
}
#[derive(Clone, Debug)]
pub enum StrategyRequest {
    Preview { json: String },
    Save { json: String, reviewed_json: String },
    List { before_id: Option<i64> },
    Detail { version_id: i64 },
}
/// Decode before cloning any caller-provided text; never echo framework diagnostics.
pub fn decode_strategy_request(
    value: &serde_json::Value,
) -> Result<StrategyRequest, StrategyFailure> {
    let invalid = || StrategyFailure::new("invalid_request");
    let object = value.as_object().ok_or_else(invalid)?;
    let operation = object
        .get("operation")
        .and_then(|v| v.as_str())
        .ok_or_else(invalid)?;
    let allowed: &[&str] = match operation {
        "preview" => &["operation", "json"],
        "save" => &["operation", "json", "reviewed_json"],
        "list" => &["operation", "before_id"],
        "detail" => &["operation", "version_id"],
        _ => return Err(invalid()),
    };
    if object.keys().any(|k| !allowed.contains(&k.as_str())) {
        return Err(invalid());
    }
    let text = |key| -> Result<&str, StrategyFailure> {
        let text = object
            .get(key)
            .and_then(|v| v.as_str())
            .ok_or_else(invalid)?;
        if text.len() > STRATEGY_DOCUMENT_BYTES {
            return Err(StrategyFailure::new("resource_limit"));
        }
        Ok(text)
    };
    let id = |key| -> Result<i64, StrategyFailure> {
        let s = object
            .get(key)
            .and_then(|v| v.as_str())
            .ok_or_else(invalid)?;
        if s.is_empty()
            || s.len() > 19
            || s.starts_with('0')
            || !s.bytes().all(|c| c.is_ascii_digit())
        {
            return Err(invalid());
        }
        s.parse::<i64>().ok().filter(|n| *n > 0).ok_or_else(invalid)
    };
    Ok(match operation {
        "preview" => StrategyRequest::Preview {
            json: text("json")?.into(),
        },
        "save" => {
            let json = text("json")?;
            let reviewed = text("reviewed_json")?;
            StrategyRequest::Save {
                json: json.into(),
                reviewed_json: reviewed.into(),
            }
        }
        "list" => StrategyRequest::List {
            before_id: if object.contains_key("before_id") {
                Some(id("before_id")?)
            } else {
                None
            },
        },
        "detail" => StrategyRequest::Detail {
            version_id: id("version_id")?,
        },
        _ => unreachable!(),
    })
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VersionStatus {
    Supported,
    LegacyReadOnly,
    Unsupported,
    InvalidRecord,
}
#[derive(Clone, Debug, Serialize)]
pub struct StrategySummary {
    pub id: String,
    pub recorded_at: String,
    pub display_name: Option<String>,
    pub format_revision: Option<i64>,
    pub status: VersionStatus,
}
#[derive(Clone, Debug, Serialize)]
pub struct StrategyDetail {
    pub summary: StrategySummary,
    pub json: Option<String>,
    pub explanation: Option<String>,
    pub meaning_id: Option<String>,
    pub notice: &'static str,
}
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StrategyResponse {
    Preview {
        format_revision: u8,
        meaning_id: &'static str,
        explanation: String,
        requirements: strategy::Requirements,
        notice: &'static str,
    },
    RuleErrors {
        errors: Vec<RuleError>,
        truncated: bool,
    },
    Saved {
        version: StrategySummary,
    },
    Versions {
        versions: Vec<StrategySummary>,
        next_cursor: Option<String>,
    },
    Detail {
        version: StrategyDetail,
    },
}

use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    sync::{Arc, Mutex, mpsc},
    thread,
};
use storage::Recorder;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Start,
    Pause,
    Advance,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Stopped,
    Running,
    Paused,
}
impl Status {
    fn as_str(self) -> &'static str {
        match self {
            Self::Stopped => "stopped",
            Self::Running => "running",
            Self::Paused => "paused",
        }
    }
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Record {
    pub id: i64,
    pub status: Status,
    pub tick: i64,
    pub reason: String,
    pub recorded_at: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct WriterSettings {
    pub journal_mode: String,
    pub synchronous: i64,
    pub fullfsync: i64,
    pub checkpoint_fullfsync: i64,
    pub sqlite_version: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Snapshot {
    /// Effective control state; pause takes effect before its recording completes.
    pub running: bool,
    pub pending: usize,
    pub recording_error: Option<String>,
    /// Only committed rows read back from SQLite, newest first.
    pub history: Vec<Record>,
    pub writer_settings: WriterSettings,
}

struct Shared {
    snapshot: Snapshot,
    generation: u64,
    stop_requested: bool,
    paper_busy: bool,
    paper_run_id: Option<i64>,
    paper_summary: Option<paper::RunSummary>,
    paper_context: Option<paper::InputContext>,
    #[cfg(test)]
    pause_recording_barrier: Option<(mpsc::Sender<()>, mpsc::Receiver<()>)>,
    #[cfg(test)]
    paper_publication_barrier: Option<(mpsc::Sender<()>, mpsc::Receiver<()>)>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Operation {
    Control(Action),
    Stop,
    Paper,
}
type PaperCompletion = mpsc::Sender<Result<paper::Reply, paper::Failure>>;
struct Request {
    paper: Option<(paper::Job, u64, PaperCompletion)>,
    operation: Operation,
    generation: u64,
    completion: Option<mpsc::Sender<Result<(), String>>>,
}

pub struct Engine {
    recorder: Arc<Mutex<Recorder>>,
    sender: Option<mpsc::SyncSender<Request>>,
    shared: Arc<Mutex<Shared>>,
    worker: Option<thread::JoinHandle<()>>,
    paper_fence: Arc<Mutex<paper::Fence>>,
}

fn recording_error(detail: &str) -> String {
    format!(
        "Session stopped: recording failed. The requested change is not confirmed saved. Check free disk space and workspace permissions, close other workspace tools, then quit and reopen Metten. Saved history is retained. Details: {detail}"
    )
}

impl Engine {
    /// The caller selects a local, nonsecret workspace; the renderer cannot select paths.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let (mut recorder, settings) =
            Recorder::open(path.as_ref()).map_err(|e| recording_error(&e))?;
        recorder.paper_validate_balances().map_err(|_| {
            recording_error(
                "Paper account integrity could not be established. Saved records are retained.",
            )
        })?;
        recorder
            .paper_stop_all()
            .map_err(|_| recording_error("Paper stopped boundary was not confirmed."))?;
        let previous = recorder.latest().map_err(|e| recording_error(&e))?;
        let tick = previous.first().map_or(0, |r| r.tick);
        // Every open records a stopped boundary, including after an unexpected exit.
        let history = recorder
            .append(Status::Stopped, tick, "Workspace opened — stopped")
            .map_err(|e| recording_error(&e))?;
        let initial_paper = recorder
            .paper_latest()
            .map_err(|_| recording_error("Paper state could not be read."))?;
        let recorder = Arc::new(Mutex::new(recorder));
        let shared = Arc::new(Mutex::new(Shared {
            snapshot: Snapshot {
                running: false,
                pending: 0,
                recording_error: None,
                history,
                writer_settings: settings,
            },
            generation: 0,
            stop_requested: false,
            paper_busy: false,
            paper_run_id: initial_paper.as_ref().and_then(|r| r.run_id.parse().ok()),
            paper_summary: initial_paper,
            paper_context: None,
            #[cfg(test)]
            pause_recording_barrier: None,
            #[cfg(test)]
            paper_publication_barrier: None,
        }));
        let (sender, receiver) = mpsc::sync_channel::<Request>(8);
        let state = shared.clone();
        let writer = recorder.clone();
        let paper_fence = Arc::new(Mutex::new(paper::Fence::default()));
        let worker_fence = paper_fence.clone();
        let worker = thread::Builder::new()
            .name("metten-recorder".into())
            .spawn(move || {
                let mut paper_owner = paper::Owner::new();
                for request in receiver {
                    if let Some((job, admitted_generation, completion)) = request.paper {
                        let starting = matches!(&job, paper::Job::Start(_));
                        let resuming =
                            matches!(&job, paper::Job::Control(_, paper::Control::Resume));
                        let mut result = paper_owner.handle(
                            job,
                            &mut writer.lock().expect("recorder poisoned"),
                            &worker_fence,
                        );
                        #[cfg(test)]
                        {
                            let barrier = state.lock().unwrap().paper_publication_barrier.take();
                            if let Some((entered, proceed)) = barrier {
                                let _ = entered.send(());
                                let _ = proceed.recv_timeout(std::time::Duration::from_secs(5));
                            }
                        }
                        let created_context = result
                            .as_ref()
                            .is_ok_and(|reply| starting && reply.created_context);
                        if result.is_ok() && (created_context || resuming) {
                            let mut fence = worker_fence.lock().unwrap();
                            if fence.generation == admitted_generation
                                && !fence.quit
                                && (!fence.input_stopped || created_context)
                            {
                                fence.paused = false;
                                if created_context {
                                    fence.stop_after_market = false;
                                    fence.input_stopped = false;
                                }
                            }
                        }
                        let mut confirmed = writer
                            .lock()
                            .expect("recorder poisoned")
                            .paper_latest()
                            .ok()
                            .flatten();
                        let mut revocation_recorded = false;
                        // Admission and idle publication share this lock so a Busy revocation cannot be missed.
                        loop {
                            let mut shared = state.lock().unwrap();
                            let must_stop = worker_fence.lock().unwrap().input_stopped;
                            if must_stop && !revocation_recorded {
                                drop(shared);
                                let mut recorder = writer.lock().expect("recorder poisoned");
                                if let Err(error) = paper_owner.revoke_input(&mut recorder) {
                                    result = Err(error);
                                }
                                confirmed = recorder.paper_latest().ok().flatten();
                                if let Ok(reply) = &mut result
                                    && let Some(summary) = &confirmed
                                    && reply.summary.run_id == summary.run_id
                                {
                                    reply.summary = summary.clone();
                                    reply.context = None;
                                }
                                revocation_recorded = true;
                                continue;
                            }
                            if let Ok(reply) = &mut result {
                                let fence = worker_fence.lock().unwrap();
                                reply.effective_paused = fence.paused || fence.quit;
                                reply.effective_generation = fence.generation;
                            }
                            shared.paper_busy = false;
                            if let Some(summary) = confirmed {
                                shared.paper_run_id = summary.run_id.parse().ok();
                                if summary.control_state == "stopped"
                                    || summary.control_state == "unresolved"
                                {
                                    shared.paper_context = None;
                                } else if let Ok(reply) = &result
                                    && reply.summary.run_id == summary.run_id
                                {
                                    shared.paper_context = reply.context;
                                }
                                shared.paper_summary = Some(summary);
                            }
                            break;
                        }
                        let _ = completion.send(result);
                        continue;
                    }
                    let (tick, failed) = {
                        let state = state.lock().expect("engine state poisoned");
                        (
                            state.snapshot.history[0].tick,
                            state.snapshot.recording_error.is_some(),
                        )
                    };
                    let stopping = request.operation == Operation::Stop;
                    if failed && !stopping {
                        state
                            .lock()
                            .expect("engine state poisoned")
                            .snapshot
                            .pending -= 1;
                        continue;
                    }
                    #[cfg(test)]
                    if request.operation == Operation::Control(Action::Pause) {
                        let barrier = state.lock().unwrap().pause_recording_barrier.take();
                        if let Some((entered, resume)) = barrier {
                            entered.send(()).unwrap();
                            resume.recv().unwrap();
                        }
                    }
                    let (status, tick, reason) = match request.operation {
                        Operation::Control(Action::Start) => {
                            (Status::Running, tick, "Controlled session started")
                        }
                        Operation::Control(Action::Pause) => {
                            (Status::Paused, tick, "Controlled session paused")
                        }
                        Operation::Control(Action::Advance) => (
                            Status::Running,
                            tick.checked_add(1).unwrap_or(-1),
                            "Test clock advanced one second",
                        ),
                        Operation::Stop => (Status::Stopped, tick, "Workspace closed — stopped"),
                        Operation::Paper => unreachable!(),
                    };
                    let mut recorder = writer.lock().expect("recorder poisoned");
                    let result = if stopping {
                        paper_owner
                            .stop(&mut recorder)
                            .map_err(|_| "Paper stopped state is not confirmed saved.".to_string())
                            .and_then(|()| {
                                recorder
                                    .latest()
                                    .and_then(|rows| recorder.append(status, rows[0].tick, reason))
                            })
                    } else if tick < 0 {
                        Err("The test clock reached its supported limit.".into())
                    } else {
                        recorder.append(status, tick, reason)
                    };
                    let completion = result.as_ref().map(|_| ()).map_err(|error| {
                        let error = recording_error(error);
                        if stopping {
                            format!(
                                "Quit canceled. {error} After fixing the problem, try Quit again."
                            )
                        } else {
                            error
                        }
                    });
                    let mut state = state.lock().expect("engine state poisoned");
                    state.snapshot.pending -= 1;
                    if stopping {
                        state.paper_summary = recorder.paper_latest().ok().flatten();
                    }
                    match result {
                        Ok(history) => {
                            state.snapshot.history = history;
                            if stopping {
                                state.snapshot.recording_error = None;
                            }
                            if state.generation == request.generation {
                                state.snapshot.running = status == Status::Running;
                            }
                        }
                        Err(_) => {
                            if stopping {
                                state.stop_requested = false;
                            }
                            state.snapshot.running = false;
                            state.snapshot.recording_error = completion.as_ref().err().cloned();
                        }
                    }
                    drop(state);
                    let stopped = stopping && completion.is_ok();
                    if let Some(reply) = request.completion {
                        let _ = reply.send(completion);
                    }
                    if stopped {
                        break;
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            recorder,
            sender: Some(sender),
            shared,
            worker: Some(worker),
            paper_fence,
        })
    }

    /// Acceptance means queued, never saved. Observe pending/error and committed history.
    pub fn request(&self, action: Action) -> Result<(), String> {
        self.enqueue(Operation::Control(action), None)
    }

    /// Stop immediately, then wait for the saved stop. Call off the desktop event thread.
    /// On failure the engine stays stopped and this operation may be retried.
    pub fn stop_and_save(&self) -> Result<(), String> {
        self.fence_paper_quit();
        let (reply, result) = mpsc::channel();
        self.enqueue(Operation::Stop, Some(reply))?;
        result.recv().map_err(|_| {
            "Quit canceled: recorder unavailable. Reopen Metten after checking the workspace."
                .to_string()
        })?
    }

    fn enqueue(
        &self,
        operation: Operation,
        completion: Option<mpsc::Sender<Result<(), String>>>,
    ) -> Result<(), String> {
        let mut state = self
            .shared
            .lock()
            .map_err(|_| "Engine unavailable. Quit and reopen Metten.".to_string())?;
        if state.stop_requested {
            return Err("The session is stopping or closed.".into());
        }
        if matches!(
            operation,
            Operation::Control(Action::Pause) | Operation::Stop
        ) {
            state.snapshot.running = false;
            state.generation += 1;
        }
        if operation != Operation::Stop
            && let Some(error) = &state.snapshot.recording_error
        {
            return Err(error.clone());
        }
        if state.snapshot.pending > 0
            && !matches!(
                operation,
                Operation::Control(Action::Pause) | Operation::Stop
            )
        {
            return Err("Wait for recording to finish.".into());
        }
        match operation {
            Operation::Control(Action::Start) if state.snapshot.running => {
                return Err("The session is already running.".into());
            }
            Operation::Control(Action::Advance) if !state.snapshot.running => {
                return Err("Start the controlled session before advancing the clock.".into());
            }
            _ => (),
        }
        let request = Request {
            paper: None,
            operation,
            generation: state.generation,
            completion,
        };
        self.sender
            .as_ref()
            .ok_or("Engine closed")?
            .try_send(request)
            .map_err(|_| {
                "Controls are busy. The session remains paused; wait for recording to finish."
                    .to_string()
            })?;
        if operation == Operation::Stop {
            state.stop_requested = true;
        }
        state.snapshot.pending += 1;
        Ok(())
    }

    pub fn fence_paper_quit(&self) {
        let mut fence = self.paper_fence.lock().unwrap();
        fence.quit = true;
        fence.paused = true;
        fence.generation = fence.generation.saturating_add(1);
    }

    pub fn paper_start(&self, start: paper::Start) -> Result<paper::Reply, paper::Failure> {
        self.paper_job(paper::Job::Start(start), None)
    }
    pub fn paper_input(&self, input: paper::Input) -> Result<paper::Reply, paper::Failure> {
        self.paper_job(paper::Job::Input(Box::new(input)), None)
    }
    pub fn paper_context(&self, run: i64) -> Result<paper::InputContext, paper::Failure> {
        self.paper_job(paper::Job::Context(run), None)?
            .context
            .ok_or_else(|| paper::Failure::new("stopped_context"))
    }
    pub fn paper_control(
        &self,
        run: i64,
        control: paper::Control,
    ) -> Result<paper::Reply, paper::Failure> {
        let generation = self.fence_paper_control(run, control)?;
        self.paper_job(paper::Job::Control(run, control), Some(generation))
    }
    pub fn paper_resume_admitted(
        &self,
        run: i64,
        generation: u64,
    ) -> Result<paper::Reply, paper::Failure> {
        self.fence_paper_control(run, paper::Control::Resume)?;
        self.paper_job(
            paper::Job::Control(run, paper::Control::Resume),
            Some(generation),
        )
    }
    pub fn fence_paper_control(
        &self,
        run: i64,
        control: paper::Control,
    ) -> Result<u64, paper::Failure> {
        {
            let state = self.shared.lock().unwrap();
            if state.paper_run_id != Some(run) {
                return Err(paper::Failure::new("stale_run"));
            }
            let mut fence = self.paper_fence.lock().unwrap();
            match control {
                paper::Control::Pause => {
                    fence.paused = true;
                    fence.generation = fence.generation.saturating_add(1);
                }
                paper::Control::StopAfterMarket => {
                    fence.stop_after_market = true;
                    fence.generation = fence.generation.saturating_add(1);
                }
                paper::Control::Resume => (),
            }
            Ok(fence.generation)
        }
    }
    pub fn paper_check_settlement(&self, run: i64) -> Result<paper::Reply, paper::Failure> {
        self.paper_job(paper::Job::CheckSettlement(run), None)
    }
    pub fn paper_summary(&self, run: i64) -> Result<paper::RunSummary, paper::Failure> {
        self.recorder
            .lock()
            .map_err(|_| paper::Failure::new("storage"))?
            .paper_summary(run)
    }
    pub fn paper_effective(&self) -> (bool, bool, u64) {
        let fence = self.paper_fence.lock().unwrap();
        (
            fence.paused || fence.quit,
            fence.stop_after_market,
            fence.generation,
        )
    }
    pub fn paper_latest(&self) -> Result<Option<paper::RunSummary>, paper::Failure> {
        Ok(self
            .shared
            .lock()
            .map_err(|_| paper::Failure::new("storage"))?
            .paper_summary
            .clone())
    }
    pub fn paper_history(
        &self,
        run: Option<i64>,
        before: Option<i64>,
    ) -> Result<paper::HistoryPage, paper::Failure> {
        self.recorder
            .lock()
            .map_err(|_| paper::Failure::new("storage"))?
            .paper_history(run, before, None)
    }
    pub fn paper_market(
        &self,
        run: i64,
        market: i64,
    ) -> Result<paper::MarketReceipt, paper::Failure> {
        self.recorder
            .lock()
            .map_err(|_| paper::Failure::new("storage"))?
            .paper_history(Some(run), None, Some(market))?
            .markets
            .into_iter()
            .next()
            .ok_or_else(|| paper::Failure::new("not_found"))
    }
    pub fn paper_evidence(
        &self,
        run: i64,
        market: i64,
        before: Option<i64>,
    ) -> Result<paper::EvidencePage, paper::Failure> {
        self.paper_market(run, market)?;
        self.recorder
            .lock()
            .map_err(|_| paper::Failure::new("storage"))?
            .paper_evidence(run, market, before)
    }

    fn paper_job(
        &self,
        job: paper::Job,
        admitted_generation: Option<u64>,
    ) -> Result<paper::Reply, paper::Failure> {
        let (reply, result) = mpsc::channel();
        {
            let mut state = self.shared.lock().unwrap();
            if state.stop_requested || self.paper_fence.lock().unwrap().quit {
                return Err(paper::Failure::new("stopped_context"));
            }
            let revocation = matches!(&job, paper::Job::Input(input)
                if state.paper_context.as_ref().is_some_and(|current| input.revokes(current)));
            if revocation {
                let mut fence = self.paper_fence.lock().unwrap();
                fence.paused = true;
                fence.generation = fence.generation.saturating_add(1);
                fence.input_stopped |= state.paper_busy;
            }
            if state.paper_busy {
                return Err(paper::Failure::new("busy"));
            }
            if let paper::Job::Input(input) = &job {
                input.canonical()?;
            }
            if let paper::Job::Start(start) = &job {
                start.validate()?;
            }
            let generation =
                admitted_generation.unwrap_or_else(|| self.paper_fence.lock().unwrap().generation);
            self.sender
                .as_ref()
                .ok_or_else(|| paper::Failure::new("stopped_context"))?
                .try_send(Request {
                    paper: Some((job, generation, reply)),
                    operation: Operation::Paper,
                    generation: 0,
                    completion: None,
                })
                .map_err(|_| {
                    if revocation {
                        self.paper_fence.lock().unwrap().input_stopped = true;
                    }
                    paper::Failure::new("busy")
                })?;
            state.paper_busy = true;
        }
        result
            .recv()
            .map_err(|_| paper::Failure::new("unconfirmed"))?
    }

    /// All writes revalidate the exact reviewed bytes under the fixed format-2 meaning.
    pub fn strategies(
        &self,
        request: StrategyRequest,
    ) -> Result<StrategyResponse, StrategyFailure> {
        self.strategies_unless_quitting(request, &std::sync::atomic::AtomicBool::new(false))
    }

    /// Recheck shutdown after validation and storage admission, immediately before a write.
    pub fn strategies_unless_quitting(
        &self,
        request: StrategyRequest,
        quitting: &std::sync::atomic::AtomicBool,
    ) -> Result<StrategyResponse, StrategyFailure> {
        let response = match request {
            StrategyRequest::Preview { json } => match ValidatedStrategy::parse(&json) {
                Ok(rules) => StrategyResponse::Preview {
                    format_revision: 2,
                    meaning_id: "metten_rules_2",
                    explanation: rules.explanation().into(),
                    requirements: rules.requirements().clone(),
                    notice: "Rule support only. Source access is unverified; no trading is connected.",
                },
                Err(errors) => StrategyResponse::RuleErrors {
                    truncated: errors.len() > 20,
                    errors: errors.into_iter().take(20).collect(),
                },
            },
            StrategyRequest::Save {
                json,
                reviewed_json,
            } => {
                if json != reviewed_json {
                    return Err(StrategyFailure::new("not_reviewed"));
                }
                let rules = match ValidatedStrategy::parse(&json) {
                    Ok(rules) => rules,
                    Err(errors) => {
                        return Ok(StrategyResponse::RuleErrors {
                            truncated: errors.len() > 20,
                            errors: errors.into_iter().take(20).collect(),
                        });
                    }
                };
                let mut recorder = self
                    .recorder
                    .lock()
                    .map_err(|_| StrategyFailure::new("storage"))?;
                if quitting.load(std::sync::atomic::Ordering::SeqCst) {
                    return Err(StrategyFailure::new("quitting"));
                }
                StrategyResponse::Saved {
                    version: recorder.save_strategy(&rules)?,
                }
            }
            StrategyRequest::List { before_id } => self
                .recorder
                .lock()
                .map_err(|_| StrategyFailure::new("storage"))?
                .strategy_versions(before_id)?,
            StrategyRequest::Detail { version_id } => self
                .recorder
                .lock()
                .map_err(|_| StrategyFailure::new("storage"))?
                .strategy_detail(version_id)?,
        };
        if serde_json::to_vec(&response)
            .map_err(|_| StrategyFailure::new("storage"))?
            .len()
            > STRATEGY_DETAIL_BYTES
        {
            return Err(StrategyFailure::new("resource_limit"));
        }
        Ok(response)
    }

    pub fn snapshot(&self) -> Snapshot {
        self.shared
            .lock()
            .expect("engine state poisoned")
            .snapshot
            .clone()
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.fence_paper_quit();
        self.sender.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    use crate::{paper::*, sources as src, strategy as rules};
    fn price(text: &str) -> rules::Decimal {
        rules::Decimal::parse(text).unwrap()
    }
    mod fixture {
        include!("../tests/support/paper.rs");
    }
    use fixture::Driver;
    fn learning_paper_start(engine: &Engine) -> paper::Start {
        fixture::scenario(fixture::saved(engine))
    }
    #[test]
    fn pause_stays_effective_after_start_saves_before_pause_saves() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.sqlite");
        let engine = Engine::open(&path).unwrap();
        let lock = rusqlite::Connection::open(&path).unwrap();
        lock.execute_batch("BEGIN IMMEDIATE").unwrap();
        let (entered, at_pause) = mpsc::channel();
        let (resume, proceed) = mpsc::channel();
        engine.shared.lock().unwrap().pause_recording_barrier = Some((entered, proceed));
        engine.request(Action::Start).unwrap();
        engine.request(Action::Pause).unwrap();
        lock.execute_batch("ROLLBACK").unwrap();
        at_pause.recv_timeout(Duration::from_secs(5)).unwrap();
        // Start has published; Pause has not begun recording. No timing-based sleep.
        let between_records = engine.snapshot();
        resume.send(()).unwrap();
        engine.stop_and_save().unwrap();
        assert_eq!(between_records.pending, 1);
        assert_eq!(between_records.history[0].status, Status::Running);
        assert!(
            !between_records.running,
            "a saved Start must not undo Pause"
        );
        assert!(
            engine
                .snapshot()
                .history
                .iter()
                .any(|r| r.status == Status::Paused)
        );
    }
    #[test]
    fn late_revocation_publishes_stopped_and_preserves_unresolved_purchase() {
        use paper::*;
        for unresolved in [true, false] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("paper.sqlite");
            let engine = Arc::new(Engine::open(&path).unwrap());
            let start = learning_paper_start(&engine);
            let first = engine.paper_start(start).unwrap();
            let context = first.context.unwrap();
            let db = rusqlite::Connection::open(&path).unwrap();
            if unresolved {
                // A durable intent with no terminal outcome is an unresolved purchase.
                db.execute("INSERT INTO paper_attempts(run_id,paper_market_id,proposal_ordinal,side,quantity,price,available_millionths,evidence_identity,intent_virtual_ms,reserved,state) SELECT run_id,paper_market_id,1,'no',10,25000000,10000000,'{}',0,263130000,'intent' FROM paper_markets WHERE run_id=?1 AND source_market_id='KXBTC15M-PAPER-NEXT'",[context.run_id]).unwrap();
                db.execute(
                    "UPDATE paper_runs SET reserved=263130000 WHERE run_id=?1",
                    [context.run_id],
                )
                .unwrap();
                db.execute_batch("CREATE TRIGGER fail_input BEFORE UPDATE OF pending_ordinal ON paper_runs BEGIN SELECT RAISE(ABORT,'controlled write failure'); END").unwrap();
            }
            let (entered, waiting) = mpsc::channel();
            let (resume, proceed) = mpsc::channel();
            engine.shared.lock().unwrap().paper_publication_barrier = Some((entered, proceed));
            let worker = engine.clone();
            let pending = thread::spawn(move || {
                worker.paper_input(Input {
                    context,
                    ordinal: 1,
                    virtual_ms: 0,
                    payload: Payload::Advance,
                })
            });
            waiting.recv_timeout(Duration::from_secs(5)).unwrap();
            let revocation = Input {
                context,
                ordinal: 2,
                virtual_ms: 0,
                payload: Payload::Source(sources::Envelope {
                    clock: sources::Clock {
                        at_ms: 0,
                        receipt_ms: 0,
                        received_through_ms: 0,
                        completed_frontier_ms: 0,
                    },
                    provider_identity: None,
                    provider_time_utc: None,
                    event: sources::SourceEvent::Connection {
                        provider: sources::Provider::Kalshi,
                        token: context.book_token,
                        state: sources::Connection::Disconnected,
                    },
                }),
            };
            let busy = engine.paper_input(revocation).unwrap_err();
            resume.send(()).unwrap();
            let result = pending.join().unwrap();
            assert_eq!(busy.code, "busy");
            assert!(engine.paper_effective().0);
            let summary = engine.paper_latest().unwrap().unwrap();
            if unresolved {
                assert_eq!(result.unwrap_err().code, "storage");
                assert_eq!(summary.control_state, "unresolved");
                assert_eq!(summary.error_code.as_deref(), Some("storage"));
                assert_eq!(summary.reserved, "263130000");
                assert_eq!(summary.cash, "10000000000");
                assert_eq!(
                    db.query_row("SELECT state FROM paper_attempts", [], |row| row
                        .get::<_, String>(0))
                        .unwrap(),
                    "intent"
                );
                assert_eq!(
                    db.query_row("SELECT count(*) FROM paper_settlements", [], |row| row
                        .get::<_, i64>(0))
                        .unwrap(),
                    0
                );
                assert_eq!(
                    engine
                        .paper_check_settlement(context.run_id)
                        .unwrap_err()
                        .code,
                    "unresolved"
                );
                // Settlement guards the durable intent even if a control row is inconsistent.
                db.execute(
                    "UPDATE paper_runs SET control_state='stopped' WHERE run_id=?1",
                    [context.run_id],
                )
                .unwrap();
                assert_eq!(
                    engine
                        .paper_check_settlement(context.run_id)
                        .unwrap_err()
                        .code,
                    "unresolved"
                );
            } else {
                let reply = result.unwrap();
                assert_eq!(reply.summary.control_state, "stopped");
                assert!(reply.context.is_none());
                assert_eq!(summary.control_state, "stopped");
                assert_eq!(
                    engine.paper_summary(context.run_id).unwrap().control_state,
                    "stopped"
                );
            }
        }
    }
    #[test]
    fn queue_full_revocation_survives_same_go_retry_and_only_fresh_go_restarts() {
        use paper::*;
        let dir = tempfile::tempdir().unwrap();
        let engine = Arc::new(Engine::open(dir.path().join("paper.sqlite")).unwrap());
        let start = learning_paper_start(&engine);
        let first = engine.paper_start(start.clone()).unwrap();
        let context = first.context.unwrap();
        let (entered, waiting) = mpsc::channel();
        let (resume, proceed) = mpsc::channel();
        engine.shared.lock().unwrap().pause_recording_barrier = Some((entered, proceed));
        engine.request(Action::Pause).unwrap();
        waiting.recv_timeout(Duration::from_secs(5)).unwrap();
        let filled: Vec<_> = (0..8).map(|_| engine.request(Action::Pause)).collect();
        let revocation = Input {
            context,
            ordinal: 1,
            virtual_ms: 0,
            payload: Payload::Source(sources::Envelope {
                clock: sources::Clock {
                    at_ms: 0,
                    receipt_ms: 0,
                    received_through_ms: 0,
                    completed_frontier_ms: 0,
                },
                provider_identity: None,
                provider_time_utc: None,
                event: sources::SourceEvent::Connection {
                    provider: sources::Provider::Kalshi,
                    token: context.book_token,
                    state: sources::Connection::Disconnected,
                },
            }),
        };
        let worker = engine.clone();
        let (done, result) = mpsc::channel();
        let revoke = thread::spawn(move || done.send(worker.paper_input(revocation)).unwrap());
        let busy = result.recv_timeout(Duration::from_secs(1));
        resume.send(()).unwrap();
        revoke.join().unwrap();
        assert!(filled.iter().all(Result::is_ok));
        assert_eq!(busy.unwrap().unwrap_err().code, "busy");
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let replay = loop {
            match engine.paper_start(start.clone()) {
                Err(error) if error.code == "busy" => {
                    assert!(std::time::Instant::now() < deadline);
                    thread::yield_now();
                }
                other => break other.unwrap(),
            }
        };
        assert_eq!(replay.summary.run_id, first.summary.run_id);
        assert!(replay.effective_paused);
        assert!(replay.context.is_none());
        assert_eq!(replay.summary.control_state, "stopped");
        assert_eq!(
            engine
                .paper_input(Input {
                    context,
                    ordinal: 1,
                    virtual_ms: 0,
                    payload: Payload::Advance
                })
                .unwrap_err()
                .code,
            "stopped_context"
        );
        let mut fresh = start;
        fresh.start_uuid = "00000000-0000-4000-8000-000000000002".into();
        let next = engine.paper_start(fresh).unwrap();
        assert_ne!(next.summary.run_id, first.summary.run_id);
        assert!(!next.effective_paused);
        engine
            .paper_input(Input {
                context: next.context.unwrap(),
                ordinal: 1,
                virtual_ms: 0,
                payload: Payload::Advance,
            })
            .unwrap();
    }
    #[test]
    fn pause_and_busy_revocation_fence_qualifying_input_before_worker_handles_it() {
        for revocation in [false, true] {
            use std::sync::Arc;
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("paper.sqlite");
            let engine = Arc::new(Engine::open(&path).unwrap());
            let mut driver = Driver::start(&engine);
            driver.prepare();
            let context = driver.context;
            let input = Input {
                context,
                ordinal: driver.ordinal + 1,
                virtual_ms: 61000,
                payload: Payload::Book(driver.book(100_000_000)),
            };
            let recorder = engine.recorder.lock().unwrap();
            let worker = engine.clone();
            let thread = std::thread::spawn(move || {
                loop {
                    match worker.paper_input(input.clone()) {
                        Err(error) if error.code == "busy" => std::thread::yield_now(),
                        reply => break reply,
                    }
                }
            });
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
            while !engine.shared.lock().unwrap().paper_busy {
                assert!(std::time::Instant::now() < deadline);
                std::thread::yield_now();
            }
            if revocation {
                let mut input = Input {
                    context,
                    ordinal: driver.ordinal + 2,
                    virtual_ms: 61000,
                    payload: Payload::Source(src::Envelope {
                        clock: src::Clock {
                            at_ms: 61000,
                            receipt_ms: 61000,
                            received_through_ms: 61000,
                            completed_frontier_ms: 61000,
                        },
                        provider_identity: None,
                        provider_time_utc: None,
                        event: src::SourceEvent::Connection {
                            provider: src::Provider::Kalshi,
                            token: context.book_token,
                            state: src::Connection::Disconnected,
                        },
                    }),
                };
                if let Payload::Source(envelope) = &mut input.payload
                    && let src::SourceEvent::Connection { token, .. } = &mut envelope.event
                {
                    *token = context.metadata_token;
                }
                assert_eq!(engine.paper_input(input.clone()).unwrap_err().code, "busy");
                assert!(!engine.paper_effective().0);
                if let Payload::Source(envelope) = &mut input.payload
                    && let src::SourceEvent::Connection { token, .. } = &mut envelope.event
                {
                    *token = context.book_token;
                }
                assert_eq!(engine.paper_input(input).unwrap_err().code, "busy");
                assert!(engine.paper_effective().0);
            } else {
                engine
                    .fence_paper_control(context.run_id, Control::Pause)
                    .unwrap();
            }
            drop(recorder);
            let result = thread.join().unwrap();
            let summary = match result {
                Ok(reply) => reply.summary,
                Err(error) if revocation && error.code == "stopped_context" => {
                    engine.paper_latest().unwrap().unwrap()
                }
                Err(error) => panic!("unexpected input error: {error:?}"),
            };
            assert_eq!(summary.cash, "10000000000");
            assert_eq!(summary.reserved, "0");
            assert_eq!(
                summary.control_state,
                if revocation { "stopped" } else { "paused" }
            );
            let db = rusqlite::Connection::open(&path).unwrap();
            assert_eq!(
                db.query_row("SELECT count(*) FROM paper_attempts", [], |r| r
                    .get::<_, i64>(0))
                    .unwrap(),
                0
            );
        }
    }
}
