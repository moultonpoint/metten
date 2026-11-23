//! Controlled paper economics; all amounts are integer 10^-8 USD.
use crate::strategy::Decimal;

pub const DOLLAR: i64 = 100_000_000;
pub const FEE_MODEL: &str = "controlled_general_taker_2026_07_07_m1";
pub const SIMULATION_MODEL: &str = "controlled_best_ask_finite_ioc_v2";

/// Depth assumption for a run's recorded simulation model; runs recorded before finite IOC
/// retries keep their original model's description.
pub fn simulation_depth(model: &str) -> &'static str {
    if model == SIMULATION_MODEL {
        "one finite best ask per observation; each IOC fills whole contracts up to that quantity and cancels the rest; retries need a newer observation and complete only the unfilled remainder"
    } else {
        "one best ask; full fill or zero fill; no partial fills or retries"
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaperError {
    Range,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    pub quantity: u64,
    pub minimum_price: Decimal,
    pub maximum_price: Decimal,
    pub starting_balance: i64,
}
impl Settings {
    pub fn validate(&self) -> Result<(), PaperError> {
        if !(1..=1_000_000).contains(&self.quantity)
            || !(1..=1_000_000 * DOLLAR).contains(&self.starting_balance)
            || self.minimum_price.millionths() < 0
            || self.maximum_price.millionths() > i128::from(DOLLAR)
            || self.minimum_price.millionths() > self.maximum_price.millionths()
        {
            return Err(PaperError::Range);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Charge {
    pub cost: i64,
    pub fee: i64,
    pub debit: i64,
}

/// Combined cost and unrounded taker fee rounded upward to $0.0001.
pub fn charge(quantity: u64, ask: Decimal) -> Result<Charge, PaperError> {
    let p = ask.millionths();
    if !(1..=1_000_000).contains(&quantity) || !(0..=i128::from(DOLLAR)).contains(&p) {
        return Err(PaperError::Range);
    }
    let q = i128::from(quantity);
    let cost = q.checked_mul(p).ok_or(PaperError::Range)?;
    // fee units = 7*q*p*(DOLLAR-p)/(100*DOLLAR); round the combined rational once.
    let denominator = 100 * i128::from(DOLLAR);
    let fee_numerator = 7_i128
        .checked_mul(q)
        .and_then(|n| n.checked_mul(p))
        .and_then(|n| n.checked_mul(i128::from(DOLLAR) - p))
        .ok_or(PaperError::Range)?;
    let numerator = cost
        .checked_mul(denominator)
        .and_then(|n| n.checked_add(fee_numerator))
        .ok_or(PaperError::Range)?;
    let quantum = 10_000;
    let rounding_divisor = denominator * quantum;
    let debit = numerator
        .checked_add(rounding_divisor - 1)
        .and_then(|n| (n / rounding_divisor).checked_mul(quantum))
        .ok_or(PaperError::Range)?;
    Ok(Charge {
        cost: i64::try_from(cost).map_err(|_| PaperError::Range)?,
        fee: i64::try_from(debit - cost).map_err(|_| PaperError::Range)?,
        debit: i64::try_from(debit).map_err(|_| PaperError::Range)?,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fill {
    Full { quantity: u64, charge: Charge },
    Partial { quantity: u64, charge: Charge },
    Zero { available_millionths: u64 },
}

/// One IOC against a finite book: fill whole contracts up to the available quantity; the
/// exchange cancels the remainder. The caller must first commit intent and bind the exact
/// evaluated ask/book identity, and must never dispatch twice against the same book.
pub fn simulate(
    quantity: u64,
    ask: Decimal,
    available_millionths: u64,
) -> Result<Fill, PaperError> {
    let whole = charge(quantity, ask)?;
    let filled = quantity.min(available_millionths / 1_000_000);
    Ok(if filled == 0 {
        Fill::Zero {
            available_millionths,
        }
    } else if filled == quantity {
        Fill::Full {
            quantity,
            charge: whole,
        }
    } else {
        Fill::Partial {
            quantity: filled,
            charge: charge(filled, ask)?,
        }
    })
}

/// One controlled IOC request, rebuilt from its durable attempt for dispatch and lookup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Order {
    pub client_order_id: String,
    pub source_market_id: String,
    pub source_contract_id: String,
    pub side: Side,
    pub quantity: u64,
    pub price: Decimal,
    pub available_millionths: u64,
}

/// The execution venue. The engine sends each client order id at most once, only after its
/// dispatch marker commits, and never relies on the venue to deduplicate a repeat.
pub trait Venue: Send {
    /// Executes one IOC. `None` means no reply: the order may or may not have been accepted.
    fn submit(&mut self, order: &Order) -> Option<Fill>;
    /// The venue's durable record of one order. `None` covers not found, incomplete or
    /// unreadable records, none of which establishes a zero fill.
    fn lookup(&mut self, client_order_id: &str) -> Option<(Order, Fill)>;
}

/// A run's or market's latest interrupted or unconfirmed attempt, for explaining the check.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Recovery {
    pub paper_market_id: String,
    pub source_market_id: String,
    pub side: String,
    pub requested_quantity: String,
    /// `intent`, `dispatched` or `unknown` while checking; otherwise the confirmed result.
    pub state: String,
    pub filled_quantity: String,
    /// Confirmed fills in that market, including earlier attempts.
    pub confirmed_quantity: String,
    /// `no_outcome`, `restart` or `legacy`.
    pub interrupted: Option<String>,
}

pub fn payout(quantity: u64, won: bool) -> Result<i64, PaperError> {
    if quantity > 1_000_000 {
        return Err(PaperError::Range);
    }
    Ok(if won {
        i64::try_from(quantity)
            .ok()
            .and_then(|n| n.checked_mul(DOLLAR))
            .ok_or(PaperError::Range)?
    } else {
        0
    })
}

use crate::strategy::{AskWitness, Side};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Book {
    pub source_market_id: String,
    pub source_contract_id: String,
    pub side: Side,
    pub binding_id: u32,
    pub source_generation: u64,
    pub value_transaction: u64,
    pub observation_number: u64,
    pub ask: Decimal,
    pub available_millionths: u64,
    pub accepted_ms: u64,
    pub valid_until_ms: u64,
}
impl Book {
    pub fn matches(
        &self,
        market: &str,
        contract: &str,
        side: Side,
        source_generation: u64,
        evaluated_through_ms: u64,
        ask: &AskWitness,
    ) -> bool {
        self.source_market_id == market
            && self.source_contract_id == contract
            && self.side == side
            && self.source_generation == source_generation
            && self.binding_id == ask.binding_id
            && self.value_transaction == ask.value_transaction
            && self.observation_number == ask.observation_number
            && self.ask == ask.price
            && self.accepted_ms <= evaluated_through_ms
            && evaluated_through_ms < self.valid_until_ms
            && evaluated_through_ms < ask.valid_until_ms
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SettlementManifestEntry {
    pub source_market_id: String,
    pub source_contract_id: String,
    pub close_ms: u64,
    pub outcome_id: String,
    pub winning_side: Side,
    pub available_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Start {
    pub start_uuid: String,
    pub strategy_version_id: i64,
    pub settings: Settings,
    pub scenario_revision: String,
    pub markets: Vec<crate::strategy::MarketDescriptor>,
    pub settlement_manifest: Vec<SettlementManifestEntry>,
    pub go_utc_ms: i64,
}
impl Start {
    pub fn validate(&self) -> Result<(), PaperError> {
        self.settings.validate()?;
        if !valid_uuid(&self.start_uuid)
            || self.start_uuid.capacity() > 128
            || self.strategy_version_id <= 0
            || self.scenario_revision.is_empty()
            || self.scenario_revision.len() > 128
            || self.scenario_revision.capacity() > 128
            || self.go_utc_ms < 899000
            || self.markets.is_empty()
            || self.markets.len() > 2
            || self.markets.capacity() > 4
            || self.settlement_manifest.len() > 2
            || self.settlement_manifest.capacity() > 4
        {
            return Err(PaperError::Range);
        }
        for (index, market) in self.markets.iter().enumerate() {
            if market.series_id != "btc15m"
                || market.series_id.capacity() > 128
                || market.market_id.is_empty()
                || market.market_id.len() > 128
                || market.market_id.capacity() > 128
                || market.contract_id.is_empty()
                || market.contract_id.len() > 128
                || market.contract_id.capacity() > 128
                || market.open_ms.checked_add(900000) != Some(market.close_ms)
                || market
                    .open_ms
                    .checked_add(self.go_utc_ms)
                    .is_none_or(|n| n < 0)
                || market.close_ms.checked_add(self.go_utc_ms).is_none()
                || self.markets[..index]
                    .iter()
                    .any(|old| old.market_id == market.market_id || old.close_ms > market.open_ms)
            {
                return Err(PaperError::Range);
            }
        }
        for (index, entry) in self.settlement_manifest.iter().enumerate() {
            if entry.outcome_id.is_empty()
                || entry.outcome_id.len() > 128
                || entry.outcome_id.capacity() > 128
                || entry.source_market_id.capacity() > 128
                || entry.source_contract_id.capacity() > 128
                || entry.available_ms < entry.close_ms
                || entry.available_ms > i64::MAX as u64
                || !self.markets.iter().any(|market| {
                    market.market_id == entry.source_market_id
                        && market.contract_id == entry.source_contract_id
                        && u64::try_from(market.close_ms).ok() == Some(entry.close_ms)
                })
                || self.settlement_manifest[..index]
                    .iter()
                    .any(|old| old.source_market_id == entry.source_market_id)
            {
                return Err(PaperError::Range);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Failure {
    pub code: &'static str,
}
impl Failure {
    pub fn new(code: &'static str) -> Self {
        Self { code }
    }
}
impl From<PaperError> for Failure {
    fn from(error: PaperError) -> Self {
        Self::new(match error {
            PaperError::Range => "range",
        })
    }
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RunSummary {
    pub run_id: String,
    pub strategy_version_id: String,
    pub revision: String,
    pub control_state: String,
    pub cash: String,
    pub reserved: String,
    pub virtual_frontier_ms: String,
    pub last_evaluated_through_ms: String,
    pub completed_ordinal: String,
    pub pending_ordinal: Option<String>,
    pub error_code: Option<String>,
    pub quantity: String,
    pub minimum_price: String,
    pub maximum_price: String,
    pub starting_balance: String,
    pub fee_model: String,
    pub simulation_model: String,
    pub scenario_revision: String,
    pub go_utc_ms: String,
    pub history_revision: String,
    pub start_uuid: String,
    pub pending_holdings: bool,
    pub recovery: Option<Recovery>,
}

use crate::{sources, storage::Recorder, strategy};
use std::sync::{Arc, Mutex, MutexGuard};

fn lock(store: &Mutex<Recorder>) -> Result<MutexGuard<'_, Recorder>, Failure> {
    store.lock().map_err(|_| Failure::new("storage"))
}

/// Settles each unconfirmed attempt from venue evidence alone, never by sending again. An intent
/// without its dispatch marker was never sent and is canceled. A possibly sent order keeps its
/// reservation and blocks purchases until the venue reports that exact order; a lookup miss or
/// incomplete record leaves it unknown. Lookups run without the recorder lock. A report that
/// contradicts the request is recorded on the run and returned as `report_conflict`.
pub(crate) fn reconcile(
    store: &Mutex<Recorder>,
    venue: &mut dyn Venue,
    run: Option<i64>,
    cause: &str,
) -> Result<(), Failure> {
    let pending = lock(store)?.paper_unconfirmed(run)?;
    for (run, attempt, state) in pending {
        let order = {
            let mut recorder = lock(store)?;
            recorder.paper_interrupt(run, attempt, cause)?;
            if state == "intent" {
                recorder.paper_finish_attempt(run, attempt, None)?;
                continue;
            }
            recorder.paper_order(attempt)?
        };
        let Some((reported, fill)) = venue.lookup(&order.client_order_id) else {
            continue;
        };
        let mut recorder = lock(store)?;
        let result = if reported == order {
            recorder.paper_finish_attempt(run, attempt, Some(fill))
        } else {
            Err(Failure::new("input_conflict"))
        };
        conflict(&mut recorder, run, attempt, result)?;
    }
    Ok(())
}

/// A report that contradicts the request or an earlier report blocks the run's purchases for good.
fn conflict(
    recorder: &mut Recorder,
    run: i64,
    attempt: i64,
    result: Result<(), Failure>,
) -> Result<(), Failure> {
    match result {
        Err(error) if error.code == "input_conflict" => {
            // The durable block first: an exit before the cause is saved still leaves it blocked.
            recorder.paper_control(run, "unresolved", true, Some("report_conflict"))?;
            recorder.paper_interrupt(run, attempt, "no_outcome")?;
            Err(Failure::new("report_conflict"))
        }
        other => other,
    }
}

#[derive(Clone, Copy, Debug)]
pub struct InputContext {
    pub run_id: i64,
    pub generation: u64,
    pub last_value_transaction: u64,
    pub book_token: sources::StreamToken,
    pub metadata_token: sources::StreamToken,
}
#[derive(Clone, Debug)]
pub enum Payload {
    Source(sources::Envelope),
    Book(Book),
    Advance,
    Settlement(SettlementManifestEntry),
}
#[derive(Clone, Debug)]
pub struct Input {
    pub context: InputContext,
    pub ordinal: i64,
    pub virtual_ms: u64,
    pub payload: Payload,
}
#[derive(Clone, Copy, Debug)]
pub enum Control {
    Pause,
    Resume,
    StopAfterMarket,
}
#[derive(Clone, Debug)]
pub struct Reply {
    pub(crate) created_context: bool,
    pub summary: RunSummary,
    pub context: Option<InputContext>,
    pub effective_paused: bool,
    pub effective_generation: u64,
    pub completion_revision: Option<String>,
    pub bindings: Vec<strategy::BindingDescriptor>,
}

#[derive(Default)]
pub(crate) struct Fence {
    pub paused: bool,
    pub input_stopped: bool,
    pub quit: bool,
    pub stop_after_market: bool,
    pub generation: u64,
}
pub(crate) enum Job {
    Start(Start),
    Input(Box<Input>),
    Control(i64, Control),
    Context(i64),
    CheckSettlement(i64),
}
pub(crate) struct Owner {
    run: Option<Run>,
}
struct Run {
    id: i64,
    generation: u64,
    evaluator: strategy::Evaluator,
    sources: sources::SourceState,
    start: Start,
    bindings: Vec<strategy::BindingDescriptor>,
    books: Vec<Book>,
    open: bool,
    current_market: Option<String>,
    participating: bool,
    last_value_transaction: u64,
    minimum_book_ms: u64,
}
impl Owner {
    pub(crate) fn new() -> Self {
        Self { run: None }
    }
    pub(crate) fn handle(
        &mut self,
        job: Job,
        store: &Mutex<Recorder>,
        venue: &mut dyn Venue,
        fence: &Arc<Mutex<Fence>>,
    ) -> Result<Reply, Failure> {
        let include_bindings = matches!(&job, Job::Start(_) | Job::Context(_));
        let settlement_check = matches!(&job, Job::CheckSettlement(_));
        let mut created_context = false;
        // Only the run this job addresses may be closed by its failure; a Go addresses the open
        // run only by repeating that run's own Go id.
        let target = match &job {
            Job::Start(start) => self
                .run
                .as_ref()
                .filter(|run| run.start.start_uuid == start.start_uuid)
                .map(|run| run.id),
            Job::Input(input) => Some(input.context.run_id),
            Job::Control(id, _) | Job::Context(id) | Job::CheckSettlement(id) => Some(*id),
        };
        let result = (|| {
            let (summary, context) = match job {
                Job::Start(start) => {
                    let mut recorder = lock(store)?;
                    let version = match recorder
                        .strategy_detail(start.strategy_version_id)
                        .map_err(|_| Failure::new("storage"))?
                    {
                        crate::StrategyResponse::Detail { version }
                            if version.summary.status == crate::VersionStatus::Supported =>
                        {
                            version
                        }
                        _ => return Err(Failure::new("unsupported_input")),
                    };
                    let rules = crate::ValidatedStrategy::parse(
                        version
                            .json
                            .as_deref()
                            .ok_or_else(|| Failure::new("storage"))?,
                    )
                    .map_err(|_| Failure::new("unsupported_input"))?;
                    if rules.source_requirements().iter().any(|r| {
                        r.source != strategy::Source::Kalshi
                            || r.field != "buyable_ask"
                            || r.startup_ms != 0
                    }) {
                        return Err(Failure::new("unsupported_input"));
                    }
                    let (id, existing) = recorder.paper_prepare_id(&start.start_uuid)?;
                    let prepared = if existing {
                        None
                    } else {
                        Some(Run::new(id, start.clone(), rules)?)
                    };
                    let summary = recorder.paper_start(&start, id)?;
                    if self.run.as_ref().is_some_and(|run| run.id == id) {
                        (
                            summary,
                            self.run.as_ref().filter(|r| r.open).map(Run::context),
                        )
                    } else {
                        if summary.control_state == "stopped"
                            || summary.control_state == "unresolved"
                        {
                            return Ok(Reply {
                                created_context: false,
                                summary,
                                context: None,
                                effective_paused: true,
                                effective_generation: fence.lock().unwrap().generation,
                                completion_revision: recorder.paper_completion_revision(id)?,
                                bindings: Vec::new(),
                            });
                        }
                        let run = prepared.ok_or_else(|| Failure::new("stopped_context"))?;
                        let context = run.context();
                        self.run = Some(run);
                        created_context = true;
                        (summary, Some(context))
                    }
                }
                Job::Context(id) => {
                    let run = self
                        .run
                        .as_ref()
                        .filter(|r| r.id == id && r.open)
                        .ok_or_else(|| Failure::new("stopped_context"))?;
                    (lock(store)?.paper_summary(id)?, Some(run.context()))
                }
                Job::Input(input) => {
                    let run = self
                        .run
                        .as_mut()
                        .filter(|r| r.id == input.context.run_id && r.open)
                        .ok_or_else(|| Failure::new("stopped_context"))?;
                    run.input(&input, store, venue, fence)?;
                    (lock(store)?.paper_summary(run.id)?, Some(run.context()))
                }
                Job::Control(id, control) => {
                    let run = self
                        .run
                        .as_mut()
                        .filter(|r| r.id == id && r.open)
                        .ok_or_else(|| Failure::new("stale_run"))?;
                    let mut recorder = lock(store)?;
                    let state = match control {
                        Control::Pause => "paused",
                        // Resume never lifts the block on a purchase still being checked.
                        Control::Resume if recorder.paper_has_unconfirmed_purchase(id)? => {
                            return Err(Failure::new("checking_purchase"));
                        }
                        Control::Resume => {
                            if run.participating {
                                "active"
                            } else {
                                "waiting"
                            }
                        }
                        Control::StopAfterMarket => {
                            if run.participating {
                                "active"
                            } else {
                                run.open = false;
                                "stopped"
                            }
                        }
                    };
                    if state == "stopped" {
                        recorder.paper_finalize_markets(id)?;
                    }
                    let summary = recorder.paper_control(
                        id,
                        state,
                        matches!(control, Control::StopAfterMarket),
                        None,
                    )?;
                    if matches!(control, Control::Resume) {
                        run.books.clear();
                        run.minimum_book_ms = run
                            .evaluator
                            .snapshot()
                            .evaluated_through_ms
                            .checked_add(1)
                            .ok_or_else(|| Failure::new("resource_limit"))?;
                    }
                    (summary, Some(run.context()))
                }
                Job::CheckSettlement(id) => {
                    if self.run.as_ref().is_some_and(|r| r.id == id && r.open) {
                        return Err(Failure::new("run_active"));
                    }
                    // A stopped run retries its purchase check before settling from the manifest.
                    reconcile(store, venue, Some(id), "no_outcome")?;
                    (lock(store)?.paper_check_settlement(id)?, None)
                }
            };
            let recorder = lock(store)?;
            let completion_revision = recorder.paper_completion_revision(
                summary
                    .run_id
                    .parse()
                    .map_err(|_| Failure::new("storage"))?,
            )?;
            let f = fence.lock().unwrap();
            Ok(Reply {
                created_context,
                summary,
                context,
                effective_paused: f.paused || f.quit,
                effective_generation: f.generation,
                completion_revision,
                bindings: if include_bindings {
                    self.run
                        .as_ref()
                        .map_or_else(Vec::new, |r| r.bindings.clone())
                } else {
                    Vec::new()
                },
            })
        })();
        if let Err(error) = &result
            && !settlement_check
            && ![
                "stale_input",
                "busy",
                "range",
                "unsupported_input",
                "stale_run",
                "run_active",
                "stopped_context",
                "checking_purchase",
            ]
            .contains(&error.code)
            && let Some(run) = self.run.as_mut().filter(|run| Some(run.id) == target)
        {
            fence.lock().unwrap().paused = true;
            run.open = false;
            if let Ok(mut recorder) = lock(store) {
                let _ = recorder.paper_control(
                    run.id,
                    if error.code == "storage" || error.code == "unconfirmed" {
                        "unresolved"
                    } else {
                        "stopped"
                    },
                    true,
                    Some(error.code),
                );
            }
        }
        result
    }
    pub(crate) fn revoke_input(&mut self, recorder: &mut Recorder) -> Result<(), Failure> {
        if let Some(run) = self.run.as_mut() {
            run.open = false;
            let saved = recorder.paper_summary(run.id)?;
            if saved.control_state == "unresolved" {
                return Ok(());
            }
            let unknown_purchase = recorder.paper_has_unconfirmed_purchase(run.id)?;
            if saved.control_state != "stopped" || unknown_purchase {
                recorder.paper_control(
                    run.id,
                    if unknown_purchase {
                        "unresolved"
                    } else {
                        "stopped"
                    },
                    true,
                    saved.error_code.as_deref().or(Some(if unknown_purchase {
                        "unconfirmed"
                    } else {
                        "unavailable_evidence"
                    })),
                )?;
            }
        }
        Ok(())
    }
    pub(crate) fn stop(
        &mut self,
        store: &Mutex<Recorder>,
        venue: &mut dyn Venue,
    ) -> Result<(), Failure> {
        if let Some(run) = self.run.as_mut() {
            run.open = false;
        }
        // Quit checks in-progress purchases; a conflict is already recorded on its run.
        match reconcile(store, venue, None, "no_outcome") {
            Err(error) if error.code != "report_conflict" => return Err(error),
            _ => (),
        }
        let mut recorder = lock(store)?;
        if let Some(run) = &self.run {
            recorder.paper_finalize_markets(run.id)?;
        }
        recorder.paper_stop_all()
    }
}
impl Run {
    fn new(id: i64, start: Start, rules: crate::ValidatedStrategy) -> Result<Self, Failure> {
        let mut bindings = Vec::new();
        for market in &start.markets {
            for requirement in rules.source_requirements() {
                let sides = match requirement.side {
                    Some(strategy::SourceSide::Fixed(side)) => vec![side],
                    Some(strategy::SourceSide::Selected) => vec![Side::Yes, Side::No],
                    None => return Err(Failure::new("unsupported_input")),
                };
                for side in sides {
                    bindings.push(strategy::BindingDescriptor {
                        binding_id: (bindings.len() + 1) as u32,
                        input_id: requirement.input_id.clone(),
                        scope: strategy::BindingScope::Contract {
                            market_id: market.market_id.clone(),
                            contract_id: market.contract_id.clone(),
                            side,
                        },
                    });
                }
            }
        }
        let caps = bindings
            .iter()
            .map(|b| {
                (
                    b.binding_id,
                    sources::Capabilities {
                        current: None,
                        observations: None,
                        history: None,
                    },
                )
            })
            .collect::<Vec<_>>();
        let sources = sources::SourceState::new(&rules, &start.markets, &bindings, &caps)
            .map_err(|_| Failure::new("unsupported_input"))?;
        let evaluator = strategy::Evaluator::new(
            rules,
            strategy::RunContext {
                strategy_version: start.strategy_version_id.to_string(),
                run_id: id.to_string(),
                go_time: start.go_utc_ms.to_string(),
                series_id: "btc15m".into(),
                markets: start.markets.clone(),
                bindings: bindings.clone(),
            },
        )
        .map_err(|_| Failure::new("resource_limit"))?;
        Ok(Self {
            id,
            generation: 1,
            evaluator,
            sources,
            start,
            bindings,
            books: Vec::new(),
            open: true,
            current_market: None,
            participating: false,
            last_value_transaction: 0,
            minimum_book_ms: 0,
        })
    }
    fn context(&self) -> InputContext {
        InputContext {
            run_id: self.id,
            generation: self.generation,
            last_value_transaction: self.last_value_transaction,
            book_token: self.sources.token(sources::Stream::Book),
            metadata_token: self.sources.token(sources::Stream::Metadata),
        }
    }
    fn input(
        &mut self,
        input: &Input,
        store: &Mutex<Recorder>,
        venue: &mut dyn Venue,
        fence: &Arc<Mutex<Fence>>,
    ) -> Result<(), Failure> {
        if input.context.generation != self.generation
            || input.context.book_token != self.sources.token(sources::Stream::Book)
            || input.context.metadata_token != self.sources.token(sources::Stream::Metadata)
        {
            return Err(Failure::new("stopped_context"));
        }
        let stopped = {
            let f = fence.lock().unwrap();
            f.quit || f.input_stopped
        };
        if stopped {
            return Err(Failure::new("stopped_context"));
        }
        if let Payload::Source(envelope) = &input.payload {
            let supplied = match &envelope.event {
                sources::SourceEvent::Connection {
                    provider, token, ..
                } => Some((
                    match provider {
                        sources::Provider::Kalshi => sources::Stream::Book,
                        sources::Provider::Coinbase => sources::Stream::Coinbase,
                        sources::Provider::Kraken => sources::Stream::Kraken,
                    },
                    *token,
                )),
                sources::SourceEvent::Subscription { stream, token, .. }
                | sources::SourceEvent::Update { stream, token, .. } => Some((*stream, *token)),
                sources::SourceEvent::Metadata { token, .. } => {
                    Some((sources::Stream::Metadata, *token))
                }
                _ => None,
            };
            if supplied.is_some_and(|(stream, token)| token != self.sources.token(stream)) {
                return Err(Failure::new("stopped_context"));
            }
        }
        let canonical = input.canonical()?;
        let at = i64::try_from(input.virtual_ms).map_err(|_| Failure::new("range"))?;
        let replayed = lock(store)?.paper_begin_input(self.id, input.ordinal, &canonical, at)?;
        if replayed.is_some() {
            return Ok(());
        }
        // Monitoring continues while a purchase is checked: consult the venue before deciding.
        reconcile(store, venue, Some(self.id), "no_outcome")?;
        let mut recorder = lock(store)?;
        if let Payload::Settlement(entry) = &input.payload {
            if !self.start.settlement_manifest.contains(entry) {
                return Err(Failure::new("input_conflict"));
            }
            recorder.paper_settle(self.id, entry, at)?;
        } else {
            if let Payload::Book(book) = &input.payload {
                if book.accepted_ms > input.virtual_ms
                    || book.source_generation != self.generation
                    || book.available_millionths > i64::MAX as u64
                    || book.valid_until_ms <= book.accepted_ms
                    || !self.bindings.iter().any(|binding| binding.binding_id==book.binding_id && matches!(&binding.scope,strategy::BindingScope::Contract {market_id,contract_id,side} if market_id==&book.source_market_id && contract_id==&book.source_contract_id && side==&book.side))
                { return Err(Failure::new("unavailable_evidence")); }
                if let Some(previous) = self
                    .books
                    .iter_mut()
                    .find(|b| b.binding_id == book.binding_id)
                {
                    *previous = book.clone();
                } else {
                    self.books.push(book.clone());
                }
            }
            let envelope = match &input.payload {
                Payload::Source(envelope) => envelope.clone(),
                _ => sources::Envelope {
                    clock: sources::Clock {
                        at_ms: input.virtual_ms,
                        receipt_ms: input.virtual_ms,
                        received_through_ms: input.virtual_ms,
                        completed_frontier_ms: self.evaluator.snapshot().evaluated_through_ms,
                    },
                    provider_identity: None,
                    provider_time_utc: None,
                    event: sources::SourceEvent::Advance,
                },
            };
            let previous = self.sources.token(sources::Stream::Book);
            let effect = self
                .sources
                .apply(envelope)
                .map_err(|_| Failure::new("unavailable_evidence"))?;
            if previous != self.sources.token(sources::Stream::Book) {
                self.generation = self
                    .generation
                    .checked_add(1)
                    .ok_or_else(|| Failure::new("resource_limit"))?;
                self.books.clear();
            }
            if let Some(transaction) = effect.batch.transactions.iter().find(|t| {
                t.changes.iter().any(|c| {
                    matches!(
                        c,
                        strategy::EvidenceChange::Observation { .. }
                            | strategy::EvidenceChange::Resynchronize { .. }
                    )
                })
            }) {
                self.last_value_transaction = transaction.number;
            }
            self.evaluator
                .begin(effect.batch)
                .map_err(|_| Failure::new("unavailable_evidence"))?;
            let source_view = self
                .sources
                .snapshot(input.virtual_ms)
                .map_err(|_| Failure::new("unavailable_evidence"))?;
            let market = source_view.current_market.clone();
            if market != self.current_market {
                if self.current_market.is_some() && fence.lock().unwrap().stop_after_market {
                    self.open = false;
                }
                self.current_market = market;
                self.participating = false;
            }
            // The durable record owns confirmed fills; the evaluator sees only this projection.
            let objective = match &self.current_market {
                None => None,
                Some(market) => {
                    let (confirmed, locked_side, attempted, unresolved) =
                        recorder.paper_objective(self.id, market)?;
                    Some(strategy::ObjectiveView {
                        market_id: market.clone(),
                        requested_quantity: self.start.settings.quantity,
                        confirmed_quantity: confirmed,
                        locked_side,
                        attempt_state: if unresolved {
                            strategy::AttemptState::Unknown
                        } else if attempted {
                            strategy::AttemptState::Finished
                        } else {
                            strategy::AttemptState::None
                        },
                        safeguards: strategy::Safeguards {
                            allowed: !unresolved && confirmed < self.start.settings.quantity,
                            yes: strategy::PriceRange {
                                minimum: self.start.settings.minimum_price,
                                maximum: self.start.settings.maximum_price,
                            },
                            no: strategy::PriceRange {
                                minimum: self.start.settings.minimum_price,
                                maximum: self.start.settings.maximum_price,
                            },
                        },
                    })
                }
            };
            let purchasable = objective.as_ref().is_some_and(|o| o.safeguards.allowed);
            let decision = loop {
                match self
                    .evaluator
                    .process_slice(strategy::SliceControl::Continue, objective.clone())
                {
                    strategy::SliceResult::Pending { .. } => {
                        if fence.lock().unwrap().quit {
                            self.open = false;
                            return Err(Failure::new("stopped_context"));
                        }
                        continue;
                    }
                    strategy::SliceResult::Complete(decision) => break decision,
                    _ => return Err(Failure::new("unavailable_evidence")),
                }
            };
            if !self.participating && fence.lock().unwrap().stop_after_market {
                self.open = false;
            }
            if let Some(market) = self
                .current_market
                .as_ref()
                .filter(|_| decision.phase == "Active" && self.open)
            {
                self.participating = true;
                recorder.paper_decision(self.id, market, &decision)?;
                recorder.paper_market_progress(
                    self.id,
                    market,
                    decision
                        .leaves
                        .iter()
                        .find(|leaf| {
                            decision
                                .witnesses
                                .first()
                                .and_then(|w| w.option_id.as_ref())
                                == Some(&leaf.option_id)
                        })
                        .map_or("unavailable_evidence", |leaf| {
                            leaf.purchase_block.unwrap_or(leaf.reason)
                        }),
                    decision.leaves.iter().any(|l| l.qualified_side.is_some()),
                )?;
            }
            if self.open
                && purchasable
                && fence.lock().unwrap().paused
                && decision
                    .leaves
                    .iter()
                    .any(|leaf| leaf.qualified_side.is_some())
                && let Some(market) = &self.current_market
            {
                recorder.paper_market_progress(self.id, market, "purchases_paused", true)?;
            }
            if self.open
                && purchasable
                && let Some(candidate) = decision.candidate
            {
                let descriptor = self
                    .start
                    .markets
                    .iter()
                    .find(|m| m.market_id == candidate.market_id)
                    .ok_or_else(|| Failure::new("unavailable_evidence"))?;
                let book = self.books.iter().find(|book| {
                    book.accepted_ms >= self.minimum_book_ms
                        && book.matches(
                            &candidate.market_id,
                            &descriptor.contract_id,
                            candidate.side,
                            self.generation,
                            decision.evaluated_through_ms,
                            &candidate.ask,
                        )
                });
                if let Some(book) = book {
                    let dependencies = source_view
                        .bindings
                        .iter()
                        .map(|b| b.binding.binding_id)
                        .collect::<Vec<_>>();
                    let stamp = self
                        .sources
                        .stamp(&dependencies)
                        .map_err(|_| Failure::new("unavailable_evidence"))?;
                    if self.sources.is_current(&stamp) && !fence.lock().unwrap().paused {
                        let intent = recorder.paper_intent(self.id, &candidate, book, at);
                        match intent {
                            Err(error)
                                if error.code == "insufficient_funds" || error.code == "price" =>
                            {
                                recorder.paper_market_progress(
                                    self.id,
                                    &candidate.market_id,
                                    error.code,
                                    true,
                                )?
                            }
                            Err(error) => return Err(error),
                            Ok(None) => {}
                            Ok(Some(attempt)) => {
                                let f = fence.lock().unwrap();
                                if !f.paused && !f.quit && self.open {
                                    // Pause and Quit wait for the marker: either they win, or the order is dispatched.
                                    let order = recorder.paper_dispatch(self.id, attempt, at)?;
                                    drop(f);
                                    drop(recorder);
                                    let reply = venue.submit(&order);
                                    recorder = lock(store)?;
                                    match reply {
                                        Some(fill) => {
                                            let result = recorder.paper_finish_attempt(
                                                self.id,
                                                attempt,
                                                Some(fill),
                                            );
                                            conflict(&mut recorder, self.id, attempt, result)?
                                        }
                                        // Unknown, not zero: the reservation and block remain.
                                        None => recorder.paper_interrupt(
                                            self.id,
                                            attempt,
                                            "no_outcome",
                                        )?,
                                    }
                                } else {
                                    drop(f);
                                    recorder.paper_finish_attempt(self.id, attempt, None)?;
                                }
                            }
                        }
                    }
                } else {
                    recorder.paper_market_progress(
                        self.id,
                        &candidate.market_id,
                        "evidence_mismatch",
                        true,
                    )?;
                }
            }
        }
        if input.virtual_ms
            >= self
                .start
                .markets
                .last()
                .map_or(0, |m| m.close_ms.max(0) as u64)
        {
            self.open = false;
        }
        if fence.lock().unwrap().input_stopped {
            self.open = false;
        }
        let state = if !self.open {
            "stopped"
        } else if fence.lock().unwrap().paused {
            "paused"
        } else if self.participating {
            "active"
        } else {
            "waiting"
        };
        if !self.open {
            recorder.paper_finalize_markets(self.id)?;
        }
        recorder.paper_complete_input(
            self.id,
            input.ordinal,
            &canonical,
            at,
            self.evaluator.snapshot().evaluated_through_ms as i64,
            state,
        )?;
        Ok(())
    }
}

impl Input {
    pub(crate) fn revokes(&self, current: &InputContext) -> bool {
        if self.context.run_id != current.run_id
            || self.context.generation != current.generation
            || self.context.book_token != current.book_token
            || self.context.metadata_token != current.metadata_token
        {
            return false;
        }
        match &self.payload {
            Payload::Source(sources::Envelope {
                event:
                    sources::SourceEvent::Connection {
                        provider: sources::Provider::Kalshi,
                        token,
                        state,
                    },
                ..
            }) => *token == current.book_token && *state != sources::Connection::Open,
            Payload::Source(sources::Envelope {
                event:
                    sources::SourceEvent::Metadata {
                        token,
                        verified: false,
                        ..
                    },
                ..
            }) => *token == current.metadata_token,
            _ => false,
        }
    }
    pub(crate) fn canonical(&self) -> Result<String, Failure> {
        use serde_json::json;
        let invalid = || Failure::new("range");
        let text = |s: &String| {
            if s.len() <= 128 && s.capacity() <= 128 {
                Ok(())
            } else {
                Err(invalid())
            }
        };
        if self.ordinal <= 0 || self.virtual_ms > i64::MAX as u64 {
            return Err(invalid());
        }
        let payload = match &self.payload {
            Payload::Advance => json!(["advance"]),
            Payload::Settlement(entry) => {
                if entry.close_ms > entry.available_ms || entry.available_ms > self.virtual_ms {
                    return Err(invalid());
                }
                text(&entry.source_market_id)?;
                text(&entry.source_contract_id)?;
                text(&entry.outcome_id)?;
                json!(["settlement", entry])
            }
            Payload::Book(book) => {
                if book.accepted_ms > self.virtual_ms
                    || book.valid_until_ms <= book.accepted_ms
                    || book.source_generation != self.context.generation
                    || book.available_millionths > i64::MAX as u64
                    || !(0..=100_000_000).contains(&book.ask.millionths())
                {
                    return Err(invalid());
                }
                text(&book.source_market_id)?;
                text(&book.source_contract_id)?;
                json!([
                    "book",
                    book.source_market_id,
                    book.source_contract_id,
                    book.side,
                    book.binding_id,
                    book.source_generation.to_string(),
                    book.value_transaction.to_string(),
                    book.observation_number.to_string(),
                    book.ask.to_string(),
                    book.available_millionths.to_string(),
                    book.accepted_ms.to_string(),
                    book.valid_until_ms.to_string()
                ])
            }
            Payload::Source(envelope) => {
                let clock = envelope.clock;
                if clock.at_ms != self.virtual_ms
                    || clock.receipt_ms > clock.at_ms
                    || clock.completed_frontier_ms > clock.at_ms
                    || clock.at_ms > clock.received_through_ms
                    || clock.received_through_ms > i64::MAX as u64
                {
                    return Err(invalid());
                }
                if envelope
                    .provider_identity
                    .as_ref()
                    .is_some_and(|s| s.len() > 128 || s.capacity() > 128)
                    || envelope
                        .provider_time_utc
                        .as_ref()
                        .is_some_and(|s| s.len() > 128 || s.capacity() > 128)
                {
                    return Err(invalid());
                }
                let event = match &envelope.event {
                    sources::SourceEvent::Connection {
                        provider, state, ..
                    } => json!(["connection", format!("{provider:?}"), format!("{state:?}")]),
                    sources::SourceEvent::Subscription { stream, state, .. } => {
                        json!(["subscription", format!("{stream:?}"), format!("{state:?}")])
                    }
                    sources::SourceEvent::Metadata {
                        market_id,
                        verified,
                        ..
                    } => {
                        text(market_id)?;
                        json!(["metadata", market_id, verified])
                    }
                    sources::SourceEvent::Boundary { market_id } => {
                        text(market_id)?;
                        json!(["boundary", market_id])
                    }
                    sources::SourceEvent::Advance => json!(["advance"]),
                    sources::SourceEvent::Update {
                        stream,
                        updates,
                        coverage,
                        ..
                    } => {
                        if updates.len() > 128
                            || coverage.len() > 128
                            || updates.capacity() > 128
                            || coverage.capacity() > 128
                        {
                            return Err(Failure::new("resource_limit"));
                        }
                        let updates = updates
                            .iter()
                            .map(|update| {
                                text(&update.binding.input_id)?;
                                let scope = match &update.binding.scope {
                                    strategy::BindingScope::Underlying => json!(["underlying"]),
                                    strategy::BindingScope::Market { market_id } => {
                                        text(market_id)?;
                                        json!(["market", market_id])
                                    }
                                    strategy::BindingScope::Contract {
                                        market_id,
                                        contract_id,
                                        side,
                                    } => {
                                        text(market_id)?;
                                        text(contract_id)?;
                                        json!(["contract", market_id, contract_id, side])
                                    }
                                };
                                let value = match update.value {
                                    None => json!(null),
                                    Some(sources::ValueUpdate::Initialize(value)) => {
                                        json!(["initialize", value.to_string()])
                                    }
                                    Some(sources::ValueUpdate::Observation { number, value }) => {
                                        json!([
                                            "observation",
                                            number.to_string(),
                                            value.to_string()
                                        ])
                                    }
                                };
                                Ok(json!([
                                    update.binding.binding_id,
                                    update.binding.input_id,
                                    scope,
                                    format!("{:?}", update.unit),
                                    value,
                                    update.valid_until_ms.map(|n| n.to_string())
                                ]))
                            })
                            .collect::<Result<Vec<_>, Failure>>()?;
                        let coverage = coverage
                            .iter()
                            .map(|c| {
                                json!([c.binding_id, c.start_ms.to_string(), c.end_ms.to_string()])
                            })
                            .collect::<Vec<_>>();
                        json!(["update", format!("{stream:?}"), updates, coverage])
                    }
                    _ => return Err(Failure::new("unsupported_input")),
                };
                json!([
                    "source",
                    envelope.clock.at_ms.to_string(),
                    envelope.clock.receipt_ms.to_string(),
                    envelope.clock.received_through_ms.to_string(),
                    envelope.clock.completed_frontier_ms.to_string(),
                    envelope.provider_identity,
                    envelope.provider_time_utc,
                    event
                ])
            }
        };
        let encoded = json!([
            "paper_input_v1",
            self.context.run_id.to_string(),
            self.context.generation.to_string(),
            self.ordinal.to_string(),
            self.virtual_ms.to_string(),
            payload
        ])
        .to_string();
        if encoded.len() > 65536 {
            return Err(Failure::new("resource_limit"));
        }
        Ok(encoded)
    }
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct MarketReceipt {
    pub run_id: String,
    pub strategy_version_id: String,
    pub paper_market_id: String,
    pub source_market_id: String,
    pub source_contract_id: String,
    pub series_id: String,
    pub open_utc_ms: String,
    pub close_utc_ms: String,
    pub close_day: String,
    pub reason: String,
    pub primary_witness: Option<serde_json::Value>,
    pub purchase_check: Option<serde_json::Value>,
    pub more_evidence: bool,
    pub requested_quantity: String,
    /// Objective-level: an unresolved attempt's state, else `full`/`partial` from confirmed
    /// fills, else the latest attempt's terminal state. Amounts total all confirmed fills;
    /// price, available quantity and time describe the latest attempt.
    pub attempt_state: Option<String>,
    pub attempt_count: String,
    pub side: Option<String>,
    pub price: Option<String>,
    pub available_millionths: Option<String>,
    pub attempt_virtual_ms: Option<String>,
    pub filled_quantity: Option<String>,
    pub cost: Option<String>,
    pub fee: Option<String>,
    pub debit: Option<String>,
    pub payout: Option<String>,
    pub settled_net: Option<String>,
    pub winning_side: Option<String>,
    pub settlement_virtual_ms: Option<String>,
    pub settlement_recorded_at: Option<String>,
    pub recovery: Option<Recovery>,
}
#[derive(Clone, Debug, Serialize)]
pub struct HistoryPage {
    pub markets: Vec<MarketReceipt>,
    pub next_cursor: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct EvidenceRecord {
    pub evidence_id: String,
    pub first_virtual_ms: String,
    pub last_virtual_ms: String,
    pub repetitions: String,
    pub witness: serde_json::Value,
}
#[derive(Clone, Debug, Serialize)]
pub struct EvidencePage {
    pub evidence: Vec<EvidenceRecord>,
    pub next_cursor: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioMarket {
    pub series_id: String,
    pub source_market_id: String,
    pub source_contract_id: String,
    pub open_ms: i64,
    pub close_ms: i64,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioObservation {
    pub at_ms: u64,
    pub source_market_id: String,
    pub yes_ask: String,
    pub no_ask: String,
    pub available_millionths: String,
    pub valid_until_ms: u64,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub format: u8,
    pub revision: String,
    pub origin: String,
    pub markets: Vec<ScenarioMarket>,
    pub observations: Vec<ScenarioObservation>,
    pub settlements: Vec<SettlementManifestEntry>,
}
impl Scenario {
    pub fn parse(json: &str) -> Result<Self, Failure> {
        if json.len() > 16384 {
            return Err(Failure::new("resource_limit"));
        }
        let scenario: Self =
            serde_json::from_str(json).map_err(|_| Failure::new("invalid_request"))?;
        if scenario.format != 1
            || scenario.origin != "synthetic_controlled"
            || scenario.observations.len() > 16
        {
            return Err(Failure::new("unsupported_input"));
        }
        let start = scenario.start(
            "00000000-0000-4000-8000-000000000001".into(),
            1,
            Settings {
                quantity: 1,
                minimum_price: Decimal::parse("0").unwrap(),
                maximum_price: Decimal::parse("100").unwrap(),
                starting_balance: DOLLAR,
            },
            899000,
        )?;
        start.validate()?;
        let mut last = 0;
        for observation in &scenario.observations {
            if observation.at_ms < last
                || observation.valid_until_ms <= observation.at_ms
                || !scenario
                    .markets
                    .iter()
                    .any(|m| m.source_market_id == observation.source_market_id)
            {
                return Err(Failure::new("range"));
            }
            charge(
                1,
                Decimal::parse(&observation.yes_ask).map_err(|_| Failure::new("range"))?,
            )?;
            charge(
                1,
                Decimal::parse(&observation.no_ask).map_err(|_| Failure::new("range"))?,
            )?;
            observation
                .available_millionths
                .parse::<i64>()
                .ok()
                .filter(|n| *n >= 0)
                .ok_or_else(|| Failure::new("range"))?;
            last = observation.at_ms;
        }
        Ok(scenario)
    }
    pub fn start(
        &self,
        uuid: String,
        version: i64,
        settings: Settings,
        go_utc_ms: i64,
    ) -> Result<Start, Failure> {
        let start = Start {
            start_uuid: uuid,
            strategy_version_id: version,
            settings,
            scenario_revision: self.revision.clone(),
            markets: self
                .markets
                .iter()
                .map(|m| strategy::MarketDescriptor {
                    series_id: m.series_id.clone(),
                    market_id: m.source_market_id.clone(),
                    contract_id: m.source_contract_id.clone(),
                    open_ms: m.open_ms,
                    close_ms: m.close_ms,
                })
                .collect(),
            settlement_manifest: self.settlements.clone(),
            go_utc_ms,
        };
        start.validate()?;
        Ok(start)
    }
}

#[derive(Clone, Debug)]
pub enum Request {
    Start {
        start_uuid: String,
        version_id: i64,
        settings: Settings,
    },
    Pause {
        run_id: i64,
    },
    Resume {
        run_id: i64,
        admitted_generation: Option<u64>,
    },
    StopAfterMarket {
        run_id: i64,
    },
    Snapshot,
    History {
        run_id: Option<i64>,
        before_paper_market_id: Option<i64>,
    },
    Market {
        run_id: i64,
        paper_market_id: i64,
    },
    Evidence {
        run_id: i64,
        paper_market_id: i64,
        before_evidence_id: Option<i64>,
    },
    CheckSettlement {
        run_id: i64,
    },
}
impl Request {
    pub fn is_read(&self) -> bool {
        matches!(
            self,
            Self::Snapshot | Self::History { .. } | Self::Market { .. } | Self::Evidence { .. }
        )
    }
}
pub fn decode_request(value: &serde_json::Value) -> Result<Request, Failure> {
    let invalid = || Failure::new("invalid_request");
    let object = value.as_object().ok_or_else(invalid)?;
    let operation = object
        .get("operation")
        .and_then(|v| v.as_str())
        .ok_or_else(invalid)?;
    let allowed: &[&str] = match operation {
        "start" => &["operation", "start_uuid", "version_id", "settings"],
        "pause" | "resume" | "stop_after_market" | "check_settlement" => &["operation", "run_id"],
        "snapshot" => &["operation"],
        "history" => &["operation", "run_id", "before_paper_market_id"],
        "market" => &["operation", "run_id", "paper_market_id"],
        "evidence" => &[
            "operation",
            "run_id",
            "paper_market_id",
            "before_evidence_id",
        ],
        _ => return Err(invalid()),
    };
    if object.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err(invalid());
    }
    let text = |key: &str| -> Result<&str, Failure> {
        let text = object
            .get(key)
            .and_then(|v| v.as_str())
            .ok_or_else(invalid)?;
        if text.len() > 128 {
            return Err(Failure::new("resource_limit"));
        }
        Ok(text)
    };
    let id = |key: &str| -> Result<i64, Failure> { positive_id(text(key)?) };
    let optional = |key: &str| -> Result<Option<i64>, Failure> {
        if object.contains_key(key) {
            id(key).map(Some)
        } else {
            Ok(None)
        }
    };
    Ok(match operation {
        "start" => {
            let start_uuid = text("start_uuid")?;
            if !valid_uuid(start_uuid) {
                return Err(invalid());
            }
            let settings = object
                .get("settings")
                .and_then(|v| v.as_object())
                .ok_or_else(invalid)?;
            if settings.len() != 4
                || settings.keys().any(|k| {
                    ![
                        "quantity",
                        "minimum_price",
                        "maximum_price",
                        "starting_balance",
                    ]
                    .contains(&k.as_str())
                })
            {
                return Err(invalid());
            }
            let field = |key| {
                settings
                    .get(key)
                    .and_then(|v| v.as_str())
                    .filter(|s| s.len() <= 32)
                    .ok_or_else(invalid)
            };
            let settings = Settings {
                quantity: u64::try_from(positive_id(field("quantity")?)?).map_err(|_| invalid())?,
                minimum_price: Decimal::parse(field("minimum_price")?)
                    .map_err(|_| Failure::new("range"))?,
                maximum_price: Decimal::parse(field("maximum_price")?)
                    .map_err(|_| Failure::new("range"))?,
                starting_balance: parse_dollars(field("starting_balance")?)?,
            };
            settings.validate()?;
            Request::Start {
                start_uuid: start_uuid.into(),
                version_id: id("version_id")?,
                settings,
            }
        }
        "pause" => Request::Pause {
            run_id: id("run_id")?,
        },
        "resume" => Request::Resume {
            run_id: id("run_id")?,
            admitted_generation: None,
        },
        "stop_after_market" => Request::StopAfterMarket {
            run_id: id("run_id")?,
        },
        "check_settlement" => Request::CheckSettlement {
            run_id: id("run_id")?,
        },
        "snapshot" => Request::Snapshot,
        "history" => Request::History {
            run_id: optional("run_id")?,
            before_paper_market_id: optional("before_paper_market_id")?,
        },
        "market" => Request::Market {
            run_id: id("run_id")?,
            paper_market_id: id("paper_market_id")?,
        },
        "evidence" => Request::Evidence {
            run_id: id("run_id")?,
            paper_market_id: id("paper_market_id")?,
            before_evidence_id: optional("before_evidence_id")?,
        },
        _ => unreachable!(),
    })
}
pub fn positive_id(text: &str) -> Result<i64, Failure> {
    if text.is_empty()
        || text.len() > 19
        || text.starts_with('0')
        || !text.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(Failure::new("invalid_request"));
    }
    text.parse()
        .ok()
        .filter(|n| *n > 0)
        .ok_or_else(|| Failure::new("range"))
}
pub fn parse_dollars(text: &str) -> Result<i64, Failure> {
    let mut parts = text.split('.');
    let whole = parts.next().unwrap_or("");
    let fraction = parts.next();
    if parts.next().is_some()
        || whole.is_empty()
        || whole.len() > 7
        || whole.len() > 1 && whole.starts_with('0')
        || !whole.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(Failure::new("range"));
    }
    let fraction = fraction.unwrap_or("");
    if fraction.len() > 8 || text.ends_with('.') || !fraction.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Failure::new("range"));
    }
    let whole = whole.parse::<i64>().map_err(|_| Failure::new("range"))?;
    let tail = if fraction.is_empty() {
        0
    } else {
        fraction.parse::<i64>().map_err(|_| Failure::new("range"))?
            * 10_i64.pow(8 - fraction.len() as u32)
    };
    whole
        .checked_mul(DOLLAR)
        .and_then(|n| n.checked_add(tail))
        .ok_or_else(|| Failure::new("range"))
}
fn valid_uuid(text: &str) -> bool {
    text.len() == 36
        && text.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
            }
        })
}

pub fn bounded_response(value: &impl Serialize, limit: usize) -> Result<(), Failure> {
    struct Counter {
        bytes: usize,
        limit: usize,
    }
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.bytes = self
                .bytes
                .checked_add(bytes.len())
                .filter(|n| *n <= self.limit)
                .ok_or_else(|| std::io::Error::other("response bound"))?;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(Counter { bytes: 0, limit }, value)
        .map_err(|_| Failure::new("resource_limit"))
}

pub(crate) fn contract_cents(units: i64) -> String {
    let fraction = format!("{:06}", units % 1_000_000);
    let fraction = fraction.trim_end_matches('0');
    format!(
        "{}{}{}",
        units / 1_000_000,
        if fraction.is_empty() { "" } else { "." },
        fraction
    )
}
