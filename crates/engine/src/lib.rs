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
    /// Published with `paper_context`: retired scheduled markets, so admission can tell a late
    /// result from a source loss without locking the run.
    paper_retired: Vec<String>,
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
        Self::open_with_venue(path, |venue| venue)
    }

    /// Opens with the workspace's controlled paper venue, which `wrap` may observe or interrupt
    /// (tests and the packaged check lose replies or exit around it).
    pub fn open_with_venue(
        path: impl AsRef<Path>,
        wrap: impl FnOnce(Box<dyn paper::Venue>) -> Box<dyn paper::Venue>,
    ) -> Result<Self, String> {
        let (mut recorder, settings) =
            Recorder::open(path.as_ref()).map_err(|e| recording_error(&e))?;
        let mut venue = wrap(Box::new(
            storage::PaperVenue::open(path.as_ref()).map_err(|e| recording_error(&e))?,
        ));
        recorder.paper_validate_balances().map_err(|_| {
            recording_error(
                "Paper account integrity could not be established. Saved records are retained.",
            )
        })?;
        recorder
            .paper_stop_all()
            .map_err(|_| recording_error("Paper stopped boundary was not confirmed."))?;
        let recorder = Arc::new(Mutex::new(recorder));
        // Crash recovery looks up unconfirmed purchases without sending; runs stay stopped. A
        // failure leaves the attempt unconfirmed and its run visibly unresolved.
        let _ = paper::reconcile(&recorder, venue.as_mut(), None, "restart");
        let (history, initial_paper) = {
            let mut recorder = recorder
                .lock()
                .map_err(|_| recording_error("Recorder unavailable."))?;
            recorder
                .paper_apply_deferred(None, 0)
                .map_err(|_| recording_error("Deferred settlement could not be recorded."))?;
            let previous = recorder.latest().map_err(|e| recording_error(&e))?;
            let tick = previous.first().map_or(0, |r| r.tick);
            // Every open records a stopped boundary, including after an unexpected exit.
            let history = recorder
                .append(Status::Stopped, tick, "Workspace opened — stopped")
                .map_err(|e| recording_error(&e))?;
            let initial_paper = recorder
                .paper_latest()
                .map_err(|_| recording_error("Paper state could not be read."))?;
            (history, initial_paper)
        };
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
            paper_retired: Vec::new(),
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
                        let starting = matches!(&job, paper::Job::Start(..));
                        let resuming =
                            matches!(&job, paper::Job::Control(_, paper::Control::Resume));
                        let mut result =
                            paper_owner.handle(job, &writer, venue.as_mut(), &worker_fence);
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
                                fence.source_losses = 0;
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
                            .flatten()
                            // An open-context reply already contains committed current-run state.
                            // A historical reply cannot replace the current admission context.
                            .or_else(|| {
                                result
                                    .as_ref()
                                    .ok()
                                    .filter(|reply| reply.context.is_some())
                                    .map(|reply| reply.summary.clone())
                            });
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
                                reply.effective_paused = fence.purchases_blocked() || fence.quit;
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
                                    shared.paper_retired = reply.retired.clone();
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
                    let paper_stopped = stopping.then(|| paper_owner.stop(&writer, venue.as_mut()));
                    let mut recorder = writer.lock().expect("recorder poisoned");
                    let result = if let Some(paper_stopped) = paper_stopped {
                        paper_stopped
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
        self.paper_job(paper::Job::Start(start, false), None)
    }
    #[cfg(test)]
    fn fixture_live_start(&self, start: paper::Start) -> Result<paper::Reply, paper::Failure> {
        self.paper_job(paper::Job::Start(start, true), None)
    }

    #[cfg(test)]
    fn fixture_live_funds(&self, run: i64, funds: paper::LiveFunds) -> Result<(), paper::Failure> {
        self.paper_job(paper::Job::FixtureFunds(run, funds), None)
            .map(|_| ())
    }

    pub fn paper_input(&self, input: paper::Input) -> Result<paper::Reply, paper::Failure> {
        self.paper_job(paper::Job::Input(Box::new(input), false), None)
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
            fence.purchases_blocked() || fence.quit,
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
        mut job: paper::Job,
        admitted_generation: Option<u64>,
    ) -> Result<paper::Reply, paper::Failure> {
        let (reply, result) = mpsc::channel();
        {
            let mut state = self.shared.lock().unwrap();
            if state.stop_requested || self.paper_fence.lock().unwrap().quit {
                return Err(paper::Failure::new("stopped_context"));
            }
            let revocation = matches!(&job, paper::Job::Input(input, _)
                if state.paper_context.as_ref().is_some_and(|current| input.revokes(current, &state.paper_retired)));
            if let paper::Job::Input(_, loss) = &mut job {
                *loss = revocation;
            }
            if revocation {
                let mut fence = self.paper_fence.lock().unwrap();
                fence.source_losses = fence.source_losses.saturating_add(1);
                fence.generation = fence.generation.saturating_add(1);
                fence.input_stopped |= state.paper_busy;
            }
            if state.paper_busy {
                return Err(paper::Failure::new("busy"));
            }
            if let paper::Job::Input(input, _) = &job {
                input.canonical()?;
            }
            if let paper::Job::Start(start, _) = &job {
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
    fn refused_loss_after_start_summary_read_failure_blocks_until_resume_or_new_run() {
        for new_run in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let engine = Engine::open(dir.path().join("paper.sqlite")).unwrap();
            engine
                .recorder
                .lock()
                .unwrap()
                .fail_next_paper_latest
                .set(true);
            let mut driver = Driver::start(&engine);
            assert!(!engine.recorder.lock().unwrap().fail_next_paper_latest.get());
            // Refused before source apply, so only admission can remember the reported loss.
            assert_eq!(
                engine
                    .paper_input(kalshi_loss(driver.context, 0, 0))
                    .unwrap_err()
                    .code,
                "range"
            );
            assert!(engine.paper_effective().0);
            driver.prepare();
            driver.input(61000, Payload::Book(driver.book(100_000_000)));
            let market = &engine
                .paper_history(Some(driver.context.run_id), None)
                .unwrap()
                .markets[0];
            assert_eq!(market.attempt_count, "0");
            assert_eq!(driver.summary.cash, "10000000000");
            assert!(engine.paper_effective().0);
            if new_run {
                engine
                    .paper_control(driver.context.run_id, Control::StopAfterMarket)
                    .unwrap();
                driver.input(901000, Payload::Advance);
                let mut start = fixture::scenario(fixture::saved(&engine));
                start.start_uuid.replace_range(35.., "2");
                let reply = engine.paper_start(start).unwrap();
                assert_ne!(reply.context.unwrap().run_id, driver.context.run_id);
                assert!(!reply.effective_paused);
                driver = Driver {
                    engine: &engine,
                    context: reply.context.unwrap(),
                    ordinal: 0,
                    summary: reply.summary,
                };
                driver.prepare();
                driver.input(61000, Payload::Book(driver.book(100_000_000)));
            } else {
                let reply = engine
                    .paper_control(driver.context.run_id, Control::Resume)
                    .unwrap();
                assert!(!reply.effective_paused);
                driver.context = reply.context.unwrap();
                driver.observe(62000, 2, "80", "25");
                driver.input(
                    62000,
                    Payload::Book(driver.side_book(rules::Side::No, "25", 2, 62000, 100_000_000)),
                );
            }
            let history = engine
                .paper_history(Some(driver.context.run_id), None)
                .unwrap();
            assert_eq!(history.markets[0].attempt_count, "1");
            assert_eq!(history.markets[0].filled_quantity.as_deref(), Some("10"));
        }
    }

    #[test]
    fn valid_loss_after_start_summary_read_failure_is_applied_and_blocks_purchases() {
        let dir = tempfile::tempdir().unwrap();
        let engine = Engine::open(dir.path().join("paper.sqlite")).unwrap();
        engine
            .recorder
            .lock()
            .unwrap()
            .fail_next_paper_latest
            .set(true);
        let mut driver = Driver::start(&engine);
        assert!(!engine.recorder.lock().unwrap().fail_next_paper_latest.get());
        let before = driver.context;
        let loss = kalshi_loss(before, 1, 0);
        driver.input(0, loss.payload);
        assert!(driver.context.generation > before.generation);
        assert_ne!(driver.context.book_token, before.book_token);
        // The applied loss leaves no admission block: source state now owns the interruption.
        assert!(!engine.paper_effective().0);
        // Reconnect, establish qualifying evidence, then verify another applied loss prevents a buy.
        driver.prepare();
        let loss = kalshi_loss(driver.context, driver.ordinal + 1, 61000);
        driver.input(61000, loss.payload);
        driver.input(61000, Payload::Book(driver.book(100_000_000)));
        let history = engine
            .paper_history(Some(driver.context.run_id), None)
            .unwrap();
        assert_eq!(history.markets[0].attempt_count, "0");
        assert_eq!(driver.summary.cash, "10000000000");
        assert!(!engine.paper_effective().0);
    }

    #[test]
    fn historical_start_summary_read_failure_preserves_current_admission_context() {
        let dir = tempfile::tempdir().unwrap();
        let engine = Engine::open(dir.path().join("paper.sqlite")).unwrap();
        let old = learning_paper_start(&engine);
        let first = engine.paper_start(old.clone()).unwrap();
        engine
            .paper_control(first.context.unwrap().run_id, Control::StopAfterMarket)
            .unwrap();
        let mut fresh = old.clone();
        fresh.start_uuid.replace_range(35.., "2");
        let current = engine.paper_start(fresh).unwrap();
        engine
            .recorder
            .lock()
            .unwrap()
            .fail_next_paper_latest
            .set(true);
        let replay = engine.paper_start(old).unwrap();
        assert_eq!(replay.summary.run_id, first.summary.run_id);
        assert!(replay.context.is_none());
        assert_eq!(
            engine.paper_latest().unwrap().unwrap().run_id,
            current.summary.run_id
        );
        assert_eq!(
            engine
                .paper_input(kalshi_loss(current.context.unwrap(), 0, 0))
                .unwrap_err()
                .code,
            "range"
        );
        assert!(engine.paper_effective().0);
        let reply = engine
            .paper_control(current.context.unwrap().run_id, Control::Resume)
            .unwrap();
        assert!(!reply.effective_paused);
    }

    #[test]
    fn live_daily_limit_uses_workspace_results_and_sticky_rollover_permissions() {
        const MIDNIGHT: i64 = 20_779 * 86_400_000;
        for (resume, pause) in [(false, false), (true, false), (true, true)] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("workspace.sqlite");
            let engine = Engine::open(&path).unwrap();
            let mut paper_start = learning_paper_start(&engine);
            paper_start.go_utc_ms = MIDNIGHT - 1_802_000;
            paper_start.settings.limits = Some(DailyLimits {
                maximum_loss: 100 * DOLLAR,
                profit_target: None,
                resume_next_day: false,
            });
            let paper_reply = engine.paper_start(paper_start.clone()).unwrap();
            let mut paper = Driver {
                engine: &engine,
                context: paper_reply.context.unwrap(),
                ordinal: 0,
                summary: paper_reply.summary,
            };
            paper.prepare();
            paper.input(61000, Payload::Book(paper.book(100_000_000)));
            paper.input(
                901000,
                Payload::Settlement(paper_start.settlement_manifest.remove(0)),
            );
            assert_eq!(paper.summary.cash, "10736870000");

            let mut start = learning_paper_start(&engine);
            start.start_uuid.replace_range(35.., "2");
            start.go_utc_ms = MIDNIGHT - 1_802_000;
            start.settings.limits = Some(DailyLimits {
                maximum_loss: 100 * DOLLAR,
                profit_target: None,
                resume_next_day: false,
            });
            start.settlement_manifest[0].winning_side = rules::Side::Yes;
            start.settlement_manifest[0].available_ms = 1_201_000;
            let reply = engine.fixture_live_start(start.clone()).unwrap();
            let mut old = Driver {
                engine: &engine,
                context: reply.context.unwrap(),
                ordinal: 0,
                summary: reply.summary,
            };
            assert_eq!(old.summary.daily_risk.as_ref().unwrap().settled_net, "0");
            old.prepare();
            let funds = |cash, valid_until| LiveFunds {
                cash,
                valid_from: 0,
                valid_until,
            };
            engine
                .fixture_live_funds(old.context.run_id, funds(100 * DOLLAR, 61000))
                .unwrap();
            old.input(61000, Payload::Book(old.book(100_000_000)));
            let old_run = old.context.run_id;
            let history = engine.paper_history(Some(old_run), None).unwrap();
            assert_eq!(
                history.markets[0].risk_block.as_deref(),
                Some("funds_unavailable")
            );
            assert_eq!(history.markets[0].attempt_count, "0");
            engine
                .fixture_live_funds(old_run, funds(5 * DOLLAR, 62000))
                .unwrap();
            old.input(61000, Payload::Book(old.book(100_000_000)));
            let history = engine.paper_history(Some(old_run), None).unwrap();
            assert_eq!(
                history.markets[0].risk_block.as_deref(),
                Some("insufficient_funds")
            );
            assert_eq!(history.markets[0].attempt_count, "0");
            engine
                .fixture_live_funds(old_run, funds(100 * DOLLAR, 62000))
                .unwrap();
            old.input(61000, Payload::Book(old.book(100_000_000)));
            assert_eq!(
                old.summary.daily_risk.as_ref().unwrap().pending_holdings,
                "263130000"
            );
            old.input(901000, Payload::Advance);
            assert_eq!(old.summary.control_state, "stopped");

            start.start_uuid.replace_range(35.., "3");
            start.go_utc_ms = MIDNIGHT - 901_000;
            start.settings.limits = Some(DailyLimits {
                maximum_loss: 263130000,
                profit_target: None,
                resume_next_day: resume,
            });
            start.settlement_manifest.clear();
            start.markets[0].open_ms = 1000;
            start.markets[0].close_ms = 901000;
            start.markets[1].open_ms = 901000;
            start.markets[1].close_ms = 1801000;
            let reply = engine.fixture_live_start(start.clone()).unwrap();
            let mut current = Driver {
                engine: &engine,
                context: reply.context.unwrap(),
                ordinal: 0,
                summary: reply.summary,
            };
            assert_eq!(
                current
                    .summary
                    .daily_risk
                    .as_ref()
                    .unwrap()
                    .pending_holdings,
                "263130000"
            );
            current.source(
                0,
                src::SourceEvent::Connection {
                    provider: src::Provider::Kalshi,
                    token: current.context.book_token,
                    state: src::Connection::Open,
                },
            );
            for market in &start.markets {
                current.source(
                    0,
                    src::SourceEvent::Metadata {
                        token: current.context.metadata_token,
                        market_id: market.market_id.clone(),
                        verified: true,
                    },
                );
            }
            current.update(0, true);
            current.source(
                1000,
                src::SourceEvent::Boundary {
                    market_id: start.markets[0].market_id.clone(),
                },
            );
            assert_eq!(current.summary.control_state, "active");
            let early = engine.paper_check_settlement(old_run).unwrap().summary;
            assert!(early.pending_holdings);
            let early = engine
                .paper_summary(current.context.run_id)
                .unwrap()
                .daily_risk
                .unwrap();
            assert_eq!(early.pending_holdings, "263130000");
            assert_eq!(early.stop_reason, None);
            current.input(301000, Payload::Advance);
            engine.paper_check_settlement(old_run).unwrap();
            let run = current.context.run_id;
            let daily = engine.paper_summary(run).unwrap().daily_risk.unwrap();
            assert_eq!(daily.settled_net, "-263130000");
            assert_eq!(daily.pending_holdings, "0");
            assert_eq!(daily.stop_reason, Some("daily_loss".into()));
            engine
                .fixture_live_funds(run, funds(100 * DOLLAR, 1_000_000))
                .unwrap();
            if pause {
                engine.paper_control(run, Control::Pause).unwrap();
            }
            current.source(
                901000,
                src::SourceEvent::Boundary {
                    market_id: start.markets[1].market_id.clone(),
                },
            );
            let history = engine.paper_history(Some(run), None).unwrap();
            let next = history
                .markets
                .iter()
                .find(|r| r.source_market_id == start.markets[1].market_id)
                .unwrap();
            assert_eq!(next.participated, resume && !pause);
            assert_eq!(
                current.summary.daily_risk.as_ref().unwrap().settled_net,
                "0"
            );
            assert_eq!(
                current
                    .summary
                    .daily_risk
                    .as_ref()
                    .unwrap()
                    .stop_reason
                    .is_none(),
                resume && !pause
            );

            drop(engine);
            let reopened = Engine::open(&path).unwrap();
            assert_eq!(
                reopened.paper_summary(run).unwrap().control_state,
                "stopped"
            );
            assert!(reopened.paper_context(run).is_err());
            start.start_uuid.replace_range(35.., "4");
            assert_eq!(
                reopened.fixture_live_start(start).unwrap_err().code,
                "range"
            );
            assert_eq!(
                reopened
                    .paper_summary(old_run)
                    .unwrap()
                    .daily_risk
                    .unwrap()
                    .settled_net,
                "-263130000"
            );
        }
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
                // A possibly sent order the venue cannot find is an unresolved purchase.
                db.execute("INSERT INTO paper_attempts(run_id,paper_market_id,proposal_ordinal,side,quantity,price,available_millionths,evidence_identity,intent_virtual_ms,reserved,dispatched_virtual_ms,state) SELECT run_id,paper_market_id,1,'no',10,25000000,10000000,'{}',0,263130000,0,'dispatched' FROM paper_markets WHERE run_id=?1 AND source_market_id='KXBTC15M-PAPER-NEXT'",[context.run_id]).unwrap();
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
                    "dispatched"
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
                // Settlement guards the unconfirmed order even if a control row is inconsistent.
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
                // Source loss blocks through its own flag, never by imitating a user Pause.
                let fence = engine.paper_fence.lock().unwrap();
                assert!(fence.source_losses == 1 && fence.input_stopped && !fence.paused);
                drop(fence);
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
    fn kalshi_loss(context: InputContext, ordinal: i64, at: u64) -> Input {
        source_input(
            context,
            ordinal,
            at,
            src::SourceEvent::Connection {
                provider: src::Provider::Kalshi,
                token: context.book_token,
                state: src::Connection::Disconnected,
            },
        )
    }
    fn source_input(
        context: InputContext,
        ordinal: i64,
        at: u64,
        event: src::SourceEvent,
    ) -> Input {
        Input {
            context,
            ordinal,
            virtual_ms: at,
            payload: Payload::Source(src::Envelope {
                clock: src::Clock {
                    at_ms: at,
                    receipt_ms: at,
                    received_through_ms: at,
                    completed_frontier_ms: at,
                },
                provider_identity: None,
                provider_time_utc: None,
                event,
            }),
        }
    }
    #[test]
    fn pause_or_loss_during_a_long_jump_and_a_slow_dispatch_marker_is_serialized_with_dispatch() {
        for arm in ["pause", "loss", "marker"] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("paper.sqlite");
            let engine = Arc::new(Engine::open(&path).unwrap());
            let mut driver = Driver::start(&engine);
            let run = driver.context.run_id;
            driver.enter();
            // A qualifying ask and its book before the purchase window; then one 60 s jump.
            driver.observe(30000, 1, "80", "25");
            driver.input(
                30000,
                Payload::Book(driver.side_book(rules::Side::No, "25", 1, 30000, 10_000_000)),
            );
            let (entered, at_stop) = mpsc::channel();
            let (resume, proceed) = mpsc::channel();
            engine.paper_fence.lock().unwrap().dispatch_barrier = Some((entered, proceed));
            let jump = Input {
                context: driver.context,
                ordinal: driver.ordinal + 1,
                virtual_ms: 90000,
                payload: Payload::Advance,
            };
            let worker = engine.clone();
            let jump = thread::spawn(move || worker.paper_input(jump));
            // The jump's intent is committed; its dispatch marker is not.
            at_stop.recv_timeout(Duration::from_secs(5)).unwrap();
            let pause = if arm == "pause" {
                // Pause is admitted at once while the jump is still in flight.
                engine.fence_paper_control(run, Control::Pause).unwrap();
                resume.send(()).unwrap();
                None
            } else if arm == "loss" {
                // A Kalshi loss is admitted between intent and marker (the worker is busy).
                let loss = kalshi_loss(driver.context, driver.ordinal + 2, 90000);
                assert_eq!(engine.paper_input(loss).unwrap_err().code, "busy");
                resume.send(()).unwrap();
                None
            } else {
                resume.send(()).unwrap();
                // The worker now holds the fence while the marker is (slowly) recorded.
                at_stop.recv_timeout(Duration::from_secs(5)).unwrap();
                assert!(engine.paper_fence.try_lock().is_err());
                let worker = engine.clone();
                let (done, finished) = mpsc::channel();
                let pause = thread::spawn(move || {
                    let result = worker.fence_paper_control(run, Control::Pause);
                    let _ = done.send(());
                    result
                });
                // Pause holds the session lock while it waits for the fence: it cannot overtake.
                let deadline = std::time::Instant::now() + Duration::from_secs(5);
                while engine.shared.try_lock().is_ok() {
                    assert!(std::time::Instant::now() < deadline, "Pause never waited");
                    thread::yield_now();
                }
                assert!(finished.try_recv().is_err());
                resume.send(()).unwrap();
                Some(pause)
            };
            let reply = jump.join().unwrap().unwrap();
            if let Some(pause) = pause {
                pause.join().unwrap().unwrap();
            }
            assert!(engine.paper_effective().0);
            let db = rusqlite::Connection::open(&path).unwrap();
            let outcome = || {
                db.query_row(
                    "SELECT (SELECT group_concat(state) FROM paper_attempts),(SELECT count(*) FROM paper_venue_orders)",
                    [],
                    |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)),
                )
                .unwrap()
            };
            if arm == "marker" {
                assert_eq!(outcome(), ("full".into(), 1));
                assert_eq!(reply.summary.cash, "9736870000");
            } else {
                assert_eq!(outcome(), ("canceled".into(), 0));
                assert_eq!(reply.summary.cash, "10000000000");
            }
            assert_eq!(reply.summary.reserved, "0");
            if arm == "loss" {
                // Known limit: a loss admitted while busy stops the run.
                assert_eq!(reply.summary.control_state, "stopped");
                continue;
            }
            // A later qualifying observation sends nothing while paused.
            driver.ordinal += 1;
            driver.context = reply.context.unwrap();
            driver.summary = reply.summary;
            let before = outcome();
            driver.observe(91000, 2, "80", "25");
            driver.input(
                91000,
                Payload::Book(driver.side_book(rules::Side::No, "25", 2, 91000, 10_000_000)),
            );
            assert_eq!(outcome(), before);
        }
    }
    #[test]
    fn feed_loss_while_a_user_request_is_in_flight_stops_the_run_but_a_stale_result_does_not() {
        for stale in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("paper.sqlite");
            let engine = Arc::new(Engine::open(&path).unwrap());
            let mut driver = Driver::start(&engine);
            let run = driver.context.run_id;
            driver.prepare();
            let (entered, waiting) = mpsc::channel();
            let (resume, proceed) = mpsc::channel();
            engine.shared.lock().unwrap().paper_publication_barrier = Some((entered, proceed));
            let worker = engine.clone();
            let pause = thread::spawn(move || worker.paper_control(run, Control::Pause));
            waiting.recv_timeout(Duration::from_secs(5)).unwrap();
            let late = if stale {
                // CURRENT closed at 1000 and was retired: this is a late result, not a loss.
                source_input(
                    driver.context,
                    driver.ordinal + 1,
                    61000,
                    src::SourceEvent::Metadata {
                        token: driver.context.metadata_token,
                        market_id: "KXBTC15M-PAPER-CURRENT".into(),
                        verified: false,
                    },
                )
            } else {
                kalshi_loss(driver.context, driver.ordinal + 1, 61000)
            };
            assert_eq!(engine.paper_input(late).unwrap_err().code, "busy");
            resume.send(()).unwrap();
            let reply = pause.join().unwrap().unwrap();
            if stale {
                assert_eq!(reply.summary.control_state, "paused");
                engine.paper_control(run, Control::Resume).unwrap();
                assert!(!engine.paper_effective().0);
                driver.observe(62000, 2, "80", "25");
                driver.input(
                    62000,
                    Payload::Book(driver.side_book(rules::Side::No, "25", 2, 62000, 10_000_000)),
                );
                assert_eq!(driver.summary.control_state, "active");
                continue;
            }
            // Known limit: the undeliverable loss permanently stops the run.
            assert_eq!(reply.summary.control_state, "stopped");
            assert!(reply.context.is_none());
            let book = Input {
                context: driver.context,
                ordinal: driver.ordinal + 1,
                virtual_ms: 61000,
                payload: Payload::Book(driver.book(10_000_000)),
            };
            assert_eq!(
                engine.paper_input(book).unwrap_err().code,
                "stopped_context"
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
