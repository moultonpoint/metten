//! Feed interruptions through the public paper interface (issue #9): source loss and reconnect,
//! stale results, bursts, silence and clock jumps. Time comes only from these inputs.
use metten_engine::paper::*;
use metten_engine::strategy::Decimal;
use metten_engine::{Engine, StrategyRequest, StrategyResponse};
use metten_engine::{sources as src, strategy as rules};
use rules::Side::{No, Yes};
use std::path::Path;
fn price(s: &str) -> Decimal {
    Decimal::parse(s).unwrap()
}
#[path = "support/paper.rs"]
mod fixture;
use fixture::{Driver, saved, scenario};

const CURRENT: &str = "KXBTC15M-PAPER-CURRENT";
const NEXT: &str = "KXBTC15M-PAPER-NEXT";

fn attempts(path: &Path) -> Vec<(String, String, i64, i64)> {
    rusqlite::Connection::open(path)
        .unwrap()
        .prepare(
            "SELECT state,side,quantity,filled_quantity FROM paper_attempts ORDER BY attempt_id",
        )
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}
/// Every recorded checkpoint latch in the run: (market, side, virtual ms).
fn latches(engine: &Engine, run: i64) -> Vec<(String, String, String)> {
    let mut found = Vec::new();
    for market in engine.paper_history(Some(run), None).unwrap().markets {
        let id = market.paper_market_id.parse().unwrap();
        let mut cursor = None;
        loop {
            let page = engine.paper_evidence(run, id, cursor).unwrap();
            for record in page.evidence {
                if record.witness["kind"] == "checkpoint_latch" {
                    found.push((
                        market.source_market_id.clone(),
                        record.witness["side"].as_str().unwrap().to_string(),
                        record.witness["evaluated_at_ms"]
                            .as_str()
                            .unwrap()
                            .to_string(),
                    ));
                }
            }
            cursor = page.next_cursor.map(|c| c.parse().unwrap());
            if cursor.is_none() {
                break;
            }
        }
    }
    found
}
fn participated(engine: &Engine, run: i64) -> Vec<String> {
    engine
        .paper_history(Some(run), None)
        .unwrap()
        .markets
        .into_iter()
        .map(|m| m.source_market_id)
        .collect()
}
fn envelope(at: u64, frontier: u64, event: src::SourceEvent) -> Payload {
    Payload::Source(src::Envelope {
        clock: src::Clock {
            at_ms: at,
            receipt_ms: at,
            received_through_ms: at,
            completed_frontier_ms: frontier,
        },
        provider_identity: None,
        provider_time_utc: None,
        event,
    })
}
/// The next input for this driver, without submitting it.
fn next(driver: &Driver, at: u64, payload: Payload) -> Input {
    Input {
        context: driver.context,
        ordinal: driver.ordinal + 1,
        virtual_ms: at,
        payload,
    }
}
fn frontier(driver: &Driver) -> u64 {
    driver.summary.last_evaluated_through_ms.parse().unwrap()
}
fn kalshi(driver: &mut Driver, at: u64, state: src::Connection) {
    driver.source(
        at,
        src::SourceEvent::Connection {
            provider: src::Provider::Kalshi,
            token: driver.context.book_token,
            state,
        },
    );
}
fn verify(driver: &mut Driver, at: u64, market: &str) {
    driver.source(
        at,
        src::SourceEvent::Metadata {
            token: driver.context.metadata_token,
            market_id: market.into(),
            verified: true,
        },
    );
}
/// YES/NO asks for one market whose bindings are `yes_id` and `yes_id + 1`.
#[allow(clippy::too_many_arguments)]
fn update(
    market: &str,
    contract: &str,
    yes_id: u32,
    yes: &str,
    no: &str,
    number: Option<u64>,
    valid_until: u64,
) -> Vec<src::BindingUpdate> {
    [(yes_id, Yes, yes), (yes_id + 1, No, no)]
        .map(|(id, side, value)| src::BindingUpdate {
            binding: rules::BindingDescriptor {
                binding_id: id,
                input_id: "selected_side_ask".into(),
                scope: rules::BindingScope::Contract {
                    market_id: market.into(),
                    contract_id: contract.into(),
                    side,
                },
            },
            unit: rules::RuleUnit::ContractCents,
            value: Some(match number {
                Some(number) => src::ValueUpdate::Observation {
                    number,
                    value: price(value),
                },
                None => src::ValueUpdate::Initialize(price(value)),
            }),
            valid_until_ms: Some(valid_until),
        })
        .to_vec()
}
fn coverage(binding_id: u32) -> rules::CoverageFact {
    rules::CoverageFact {
        binding_id,
        start_ms: 0,
        end_ms: 61000,
    }
}
fn send(driver: &mut Driver, at: u64, updates: Vec<src::BindingUpdate>) {
    driver.source(
        at,
        src::SourceEvent::Update {
            stream: src::Stream::Book,
            token: driver.context.book_token,
            updates,
            coverage: vec![],
        },
    );
}
/// Kalshi loss at `lost`, then a new connection, fresh asks and reverified metadata at `back`.
fn reconnect(driver: &mut Driver, lost: u64, back: u64, market: &str, contract: &str, yes_id: u32) {
    kalshi(driver, lost, src::Connection::Disconnected);
    kalshi(driver, back, src::Connection::Open);
    send(
        driver,
        back,
        update(market, contract, yes_id, "50", "70", None, 2000000),
    );
    verify(driver, back, market);
}
/// Two consecutive windows that both open after Go, so a run can enter the first one.
fn two_windows(engine: &Engine) -> Start {
    let mut start = scenario(saved(engine));
    start.markets = ["A", "B"]
        .iter()
        .enumerate()
        .map(|(index, name)| rules::MarketDescriptor {
            series_id: "btc15m".into(),
            market_id: format!("KXBTC15M-PAPER-{name}"),
            contract_id: format!("paper-{}", name.to_lowercase()),
            open_ms: 1000 + 900000 * index as i64,
            close_ms: 901000 + 900000 * index as i64,
        })
        .collect();
    start.settlement_manifest.clear();
    start
}
fn driver_for(engine: &Engine, start: Start) -> Driver<'_> {
    let reply = engine.paper_start(start).unwrap();
    Driver {
        engine,
        context: reply.context.unwrap(),
        ordinal: 0,
        summary: reply.summary,
    }
}
fn enter_a(driver: &mut Driver) {
    kalshi(driver, 0, src::Connection::Open);
    verify(driver, 0, "KXBTC15M-PAPER-A");
    verify(driver, 0, "KXBTC15M-PAPER-B");
    let mut initial = update("KXBTC15M-PAPER-A", "paper-a", 1, "35", "70", None, 2000000);
    initial.extend(update(
        "KXBTC15M-PAPER-B",
        "paper-b",
        3,
        "50",
        "70",
        None,
        2000000,
    ));
    send(driver, 0, initial);
    driver.source(
        1000,
        src::SourceEvent::Boundary {
            market_id: "KXBTC15M-PAPER-A".into(),
        },
    );
}
fn book_a(driver: &Driver, number: u64, at: u64) -> Book {
    Book {
        source_market_id: "KXBTC15M-PAPER-A".into(),
        source_contract_id: "paper-a".into(),
        binding_id: 2,
        observation_number: number,
        accepted_ms: at,
        valid_until_ms: 2000000,
        ..driver.side_book(No, "25", number, at, 10_000_000)
    }
}

#[test]
fn revocation_rejected_before_it_is_applied_keeps_purchases_blocked_until_resume() {
    for code in ["range", "stale_input"] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("paper.sqlite");
        let engine = Engine::open(&path).unwrap();
        let mut driver = Driver::start(&engine);
        driver.prepare();
        let mut loss = next(
            &driver,
            61000,
            envelope(
                61000,
                frontier(&driver),
                src::SourceEvent::Connection {
                    provider: src::Provider::Kalshi,
                    token: driver.context.book_token,
                    state: src::Connection::Disconnected,
                },
            ),
        );
        if code == "range" {
            // A clock behind the run's frontier is refused before the loss reaches source state.
            loss.virtual_ms = 60000;
            if let Payload::Source(envelope) = &mut loss.payload {
                envelope.clock = src::Clock {
                    at_ms: 60000,
                    receipt_ms: 60000,
                    received_through_ms: 60000,
                    completed_frontier_ms: 0,
                };
            }
        } else {
            loss.ordinal = 1;
        }
        assert_eq!(engine.paper_input(loss).unwrap_err().code, code);
        assert!(engine.paper_effective().0);
        // Source state never saw the loss, so only the admitted block stops this qualifying book.
        driver.input(61000, Payload::Book(driver.book(10_000_000)));
        assert!(attempts(&path).is_empty());
        assert_eq!(driver.summary.control_state, "paused");
        assert_eq!(
            engine.paper_history(None, None).unwrap().markets[0].reason,
            "purchases_paused"
        );
        engine
            .paper_control(driver.context.run_id, Control::Resume)
            .unwrap();
        assert!(!engine.paper_effective().0);
        driver.observe(62000, 2, "80", "25");
        driver.input(
            62000,
            Payload::Book(driver.side_book(No, "25", 2, 62000, 10_000_000)),
        );
        assert_eq!(attempts(&path), [("full".into(), "no".into(), 10, 10)]);
    }
}

#[test]
fn applying_a_different_loss_never_clears_a_refused_disconnect() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("paper.sqlite");
    let engine = Engine::open(&path).unwrap();
    let mut driver = Driver::start(&engine);
    driver.prepare();
    // A disconnect is admitted, then refused before it reaches source state.
    let mut refused = next(
        &driver,
        61000,
        envelope(
            61000,
            frontier(&driver),
            src::SourceEvent::Connection {
                provider: src::Provider::Kalshi,
                token: driver.context.book_token,
                state: src::Connection::Disconnected,
            },
        ),
    );
    refused.ordinal = 1;
    assert_eq!(engine.paper_input(refused).unwrap_err().code, "stale_input");
    // A different loss is admitted and applied; only its own admission is removed.
    driver.source(
        61100,
        src::SourceEvent::Metadata {
            token: driver.context.metadata_token,
            market_id: NEXT.into(),
            verified: false,
        },
    );
    assert!(engine.paper_effective().0);
    // Reverified metadata and a qualifying fresh book still cannot buy.
    verify(&mut driver, 61200, NEXT);
    driver.reinitialize(61300, "80", "25");
    driver.input(
        61300,
        Payload::Book(driver.side_book(No, "25", 1, 61300, 10_000_000)),
    );
    assert!(attempts(&path).is_empty());
    assert_eq!(driver.summary.control_state, "paused");
    engine
        .paper_control(driver.context.run_id, Control::Resume)
        .unwrap();
    driver.observe(62000, 2, "80", "25");
    driver.input(
        62000,
        Payload::Book(driver.side_book(No, "25", 2, 62000, 10_000_000)),
    );
    assert_eq!(attempts(&path), [("full".into(), "no".into(), 10, 10)]);
}

#[test]
fn a_level_checkpoint_true_in_the_fresh_post_reconnect_snapshot_latches_at_snapshot_time() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::open(dir.path().join("paper.sqlite")).unwrap();
    let mut driver = Driver::start(&engine);
    let run = driver.context.run_id;
    kalshi(&mut driver, 0, src::Connection::Open);
    verify(&mut driver, 0, CURRENT);
    verify(&mut driver, 0, NEXT);
    send(
        &mut driver,
        0,
        update(NEXT, "paper-next", 3, "50", "70", None, 1000000),
    );
    driver.source(
        1000,
        src::SourceEvent::Boundary {
            market_id: NEXT.into(),
        },
    );
    // Unlatched when the feed drops; nothing is assumed for the gap.
    kalshi(&mut driver, 20000, src::Connection::Disconnected);
    kalshi(&mut driver, 30000, src::Connection::Open);
    assert!(latches(&engine, run).is_empty());
    // Known limit: YES 35 in the fresh snapshot is treated as observed now.
    send(
        &mut driver,
        30000,
        update(NEXT, "paper-next", 3, "35", "70", None, 1000000),
    );
    verify(&mut driver, 30000, NEXT);
    assert_eq!(
        latches(&engine, run),
        [(NEXT.into(), "yes".into(), "30000".into())]
    );
}

#[test]
fn checkpoints_reset_when_the_next_market_is_entered_at_its_opening() {
    // Control: the same window-B book buys once B latches its own checkpoint.
    for latch_in_b in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("paper.sqlite");
        let engine = Engine::open(&path).unwrap();
        let mut driver = driver_for(&engine, two_windows(&engine));
        let run = driver.context.run_id;
        // A latches YES at its opening; B is prepared with YES 50, which never qualifies.
        enter_a(&mut driver);
        driver.source(
            901000,
            src::SourceEvent::Boundary {
                market_id: "KXBTC15M-PAPER-B".into(),
            },
        );
        let b = "KXBTC15M-PAPER-B";
        if latch_in_b {
            send(
                &mut driver,
                902000,
                update(b, "paper-b", 3, "35", "70", Some(1), 2000000),
            );
        }
        // In B's purchase window NO 25 buys only with a YES latch in B.
        let number = if latch_in_b { 2 } else { 1 };
        send(
            &mut driver,
            962000,
            update(
                "KXBTC15M-PAPER-B",
                "paper-b",
                3,
                "80",
                "25",
                Some(number),
                2000000,
            ),
        );
        driver.input(
            962000,
            Payload::Book(Book {
                source_market_id: "KXBTC15M-PAPER-B".into(),
                source_contract_id: "paper-b".into(),
                valid_until_ms: 2000000,
                ..driver.side_book(No, "25", number, 962000, 10_000_000)
            }),
        );
        let mut entered = participated(&engine, run);
        entered.sort();
        assert_eq!(entered, ["KXBTC15M-PAPER-A", "KXBTC15M-PAPER-B"]);
        let mut found = latches(&engine, run);
        found.sort();
        let mut expected = vec![(
            "KXBTC15M-PAPER-A".to_string(),
            "yes".to_string(),
            "1000".to_string(),
        )];
        if latch_in_b {
            expected.push((b.into(), "yes".into(), "902000".into()));
        }
        assert_eq!(found, expected);
        assert_eq!(attempts(&path).len(), usize::from(latch_in_b));
    }
}

#[test]
fn mid_window_reconnect_keeps_the_latch_and_continues_only_on_fresh_qualifying_evidence() {
    for qualifies in [true, false] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("paper.sqlite");
        let engine = Engine::open(&path).unwrap();
        let mut driver = Driver::start(&engine);
        let run = driver.context.run_id;
        driver.enter();
        let before = latches(&engine, run);
        assert_eq!(before, [(NEXT.into(), "yes".into(), "1000".into())]);
        let old_generation = driver.context.generation;
        // A qualifying NO ask observed before the loss, and its book.
        driver.observe(30000, 1, "80", "25");
        let pre_loss = driver.side_book(No, "25", 1, 30000, 10_000_000);
        kalshi(&mut driver, 40000, src::Connection::Disconnected);
        // Applying the loss lifts the admitted block; source state now blocks on its own.
        assert!(!engine.paper_effective().0);
        assert_ne!(driver.summary.control_state, "paused");
        kalshi(&mut driver, 50000, src::Connection::Open);
        assert_ne!(driver.context.generation, old_generation);
        // The pre-loss book is late, not fatal; nothing from before the loss can buy.
        let late = next(&driver, 50000, Payload::Book(pre_loss.clone()));
        assert_eq!(engine.paper_input(late).unwrap_err().code, "stale_result");
        // Fresh new-generation asks and their book, before metadata is reverified, cannot buy.
        let no = if qualifies { "25" } else { "50" };
        send(
            &mut driver,
            61000,
            update(NEXT, "paper-next", 3, "80", no, None, 1000000),
        );
        driver.input(
            61000,
            // Re-initialization keeps the highest observation number (1, never spent).
            Payload::Book(driver.side_book(No, no, 1, 61000, 10_000_000)),
        );
        assert!(attempts(&path).is_empty());
        if qualifies {
            assert_eq!(
                engine.paper_history(Some(run), None).unwrap().markets[0].reason,
                "objective_blocked"
            );
        }
        // Reverified metadata completes the fresh evidence: no Resume is needed.
        verify(&mut driver, 61500, NEXT);
        assert_eq!(latches(&engine, run), before);
        assert!(!engine.paper_effective().0);
        if qualifies {
            // The remembered YES side still selects its opposite.
            assert_eq!(attempts(&path), [("full".into(), "no".into(), 10, 10)]);
        } else {
            assert!(attempts(&path).is_empty());
        }
    }
}

#[test]
fn reconnect_across_a_market_boundary_never_enters_the_late_window() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("paper.sqlite");
    let engine = Engine::open(&path).unwrap();
    let mut driver = driver_for(&engine, two_windows(&engine));
    let run = driver.context.run_id;
    enter_a(&mut driver);
    kalshi(&mut driver, 850000, src::Connection::Disconnected);
    // Back after B opened at 901000: its opening was missed.
    kalshi(&mut driver, 902000, src::Connection::Open);
    send(
        &mut driver,
        902000,
        update("KXBTC15M-PAPER-B", "paper-b", 3, "35", "70", None, 2000000),
    );
    verify(&mut driver, 902000, "KXBTC15M-PAPER-B");
    driver.source(
        902000,
        src::SourceEvent::Boundary {
            market_id: "KXBTC15M-PAPER-B".into(),
        },
    );
    // In B's purchase window a qualifying opposite-side ask and its book arrive.
    send(
        &mut driver,
        962000,
        update(
            "KXBTC15M-PAPER-B",
            "paper-b",
            3,
            "80",
            "25",
            Some(1),
            2000000,
        ),
    );
    driver.input(
        962000,
        Payload::Book(Book {
            source_market_id: "KXBTC15M-PAPER-B".into(),
            source_contract_id: "paper-b".into(),
            valid_until_ms: 2000000,
            ..driver.side_book(No, "25", 1, 962000, 10_000_000)
        }),
    );
    assert!(attempts(&path).is_empty());
    assert_eq!(participated(&engine, run), ["KXBTC15M-PAPER-A"]);
    assert_eq!(
        latches(&engine, run),
        [("KXBTC15M-PAPER-A".into(), "yes".into(), "1000".into())]
    );
}

#[test]
fn user_pause_stop_and_quit_win_over_source_recovery() {
    // Pause lands before the loss is applied; applying it must not lift the user's Pause.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("paper.sqlite");
    let engine = Engine::open(&path).unwrap();
    let mut driver = Driver::start(&engine);
    let run = driver.context.run_id;
    driver.enter();
    engine.paper_control(run, Control::Pause).unwrap();
    reconnect(&mut driver, 30000, 40000, NEXT, "paper-next", 3);
    driver.observe(61000, 1, "80", "25");
    driver.input(
        61000,
        Payload::Book(driver.side_book(No, "25", 1, 61000, 10_000_000)),
    );
    assert!(engine.paper_effective().0);
    assert_eq!(driver.summary.control_state, "paused");
    assert!(attempts(&path).is_empty());
    engine.paper_control(run, Control::Resume).unwrap();
    driver.observe(62000, 2, "80", "25");
    driver.input(
        62000,
        Payload::Book(driver.side_book(No, "25", 2, 62000, 10_000_000)),
    );
    assert_eq!(attempts(&path).len(), 1);

    // Stop-after-market survives recovery: the entered window may finish, the next is refused.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("paper.sqlite");
    let engine = Engine::open(&path).unwrap();
    let mut driver = driver_for(&engine, two_windows(&engine));
    let run = driver.context.run_id;
    enter_a(&mut driver);
    engine.paper_control(run, Control::StopAfterMarket).unwrap();
    reconnect(&mut driver, 30000, 40000, "KXBTC15M-PAPER-A", "paper-a", 1);
    assert!(engine.paper_effective().1);
    send(
        &mut driver,
        61000,
        update(
            "KXBTC15M-PAPER-A",
            "paper-a",
            1,
            "80",
            "25",
            Some(1),
            2000000,
        ),
    );
    driver.input(61000, Payload::Book(book_a(&driver, 1, 61000)));
    assert_eq!(attempts(&path).len(), 1);
    driver.source(
        901000,
        src::SourceEvent::Boundary {
            market_id: "KXBTC15M-PAPER-B".into(),
        },
    );
    assert_eq!(driver.summary.control_state, "stopped");
    assert_eq!(participated(&engine, run), ["KXBTC15M-PAPER-A"]);

    // Quit while the loss is being recovered: nothing more is admitted or sent.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("paper.sqlite");
    let engine = Engine::open(&path).unwrap();
    let mut driver = Driver::start(&engine);
    driver.enter();
    kalshi(&mut driver, 30000, src::Connection::Disconnected);
    engine.fence_paper_quit();
    let back = next(
        &driver,
        40000,
        envelope(
            40000,
            frontier(&driver),
            src::SourceEvent::Connection {
                provider: src::Provider::Kalshi,
                token: driver.context.book_token,
                state: src::Connection::Open,
            },
        ),
    );
    assert_eq!(
        engine.paper_input(back).unwrap_err().code,
        "stopped_context"
    );
    assert!(engine.paper_effective().0);
    assert!(attempts(&path).is_empty());
}

#[test]
fn stale_results_are_refused_without_consuming_the_ordinal_and_unknown_or_mixed_stop_the_run() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("paper.sqlite");
    let engine = Engine::open(&path).unwrap();
    let mut driver = Driver::start(&engine);
    driver.prepare();
    // CURRENT closed at 1000 and was retired; NEXT is current.
    let retired_update = src::SourceEvent::Update {
        stream: src::Stream::Book,
        token: driver.context.book_token,
        updates: update(CURRENT, "paper-current", 1, "35", "70", Some(1), 1000000),
        coverage: vec![],
    };
    let retired_metadata = src::SourceEvent::Metadata {
        token: driver.context.metadata_token,
        market_id: CURRENT.into(),
        verified: true,
    };
    // Coverage alone for a retired binding is late too.
    let retired_coverage = src::SourceEvent::Update {
        stream: src::Stream::Book,
        token: driver.context.book_token,
        updates: vec![],
        coverage: vec![coverage(1)],
    };
    for event in [retired_update, retired_metadata, retired_coverage] {
        let late = next(&driver, 61000, envelope(61000, 61000, event));
        let completed = driver.summary.completed_ordinal.clone();
        assert_eq!(engine.paper_input(late).unwrap_err().code, "stale_result");
        // The same ordinal then carries a valid input.
        driver.input(61000, Payload::Advance);
        assert_eq!(
            driver.summary.completed_ordinal,
            (completed.parse::<i64>().unwrap() + 1).to_string()
        );
    }
    // A book from before a reconnect is late, and the same ordinal then succeeds.
    let old = driver.book(10_000_000);
    reconnect(&mut driver, 61100, 61200, NEXT, "paper-next", 3);
    let late = next(&driver, 61300, Payload::Book(old.clone()));
    assert_eq!(engine.paper_input(late).unwrap_err().code, "stale_result");
    driver.input(61300, Payload::Advance);
    // A book claiming a newer generation keeps its existing non-fatal range refusal.
    let newer = Book {
        source_generation: driver.context.generation + 1,
        ..driver.book(10_000_000)
    };
    let newer = next(&driver, 61400, Payload::Book(newer));
    assert_eq!(engine.paper_input(newer).unwrap_err().code, "range");
    driver.input(61400, Payload::Advance);
    assert_eq!(driver.summary.control_state, "active");
    assert!(attempts(&path).is_empty());
    // A late unverified result for the retired market is stale, not a source loss.
    let late = src::SourceEvent::Metadata {
        token: driver.context.metadata_token,
        market_id: CURRENT.into(),
        verified: false,
    };
    let late = next(&driver, 61500, envelope(61500, 61400, late));
    assert_eq!(engine.paper_input(late).unwrap_err().code, "stale_result");
    assert!(!engine.paper_effective().0);
    driver.input(61500, Payload::Advance);
    assert_eq!(driver.summary.control_state, "active");

    // An unknown market, or a batch mixing a retired and a current or unknown binding (in values
    // or coverage), still stops the run.
    let unknown = |driver: &Driver| src::SourceEvent::Metadata {
        token: driver.context.metadata_token,
        market_id: "KXBTC15M-PAPER-UNKNOWN".into(),
        verified: true,
    };
    let unknown_loss = |driver: &Driver| src::SourceEvent::Metadata {
        token: driver.context.metadata_token,
        market_id: "KXBTC15M-PAPER-UNKNOWN".into(),
        verified: false,
    };
    let retired_values = |driver: &Driver, binding: u32| src::SourceEvent::Update {
        stream: src::Stream::Book,
        token: driver.context.book_token,
        updates: update(CURRENT, "paper-current", 1, "35", "70", Some(1), 1000000),
        coverage: vec![coverage(binding)],
    };
    let current_coverage = |driver: &Driver| retired_values(driver, 3);
    let unknown_coverage = |driver: &Driver| retired_values(driver, 99);
    // A descriptor naming the retired market on a current (3) or unknown (99) binding id.
    let mislabelled = |driver: &Driver, yes_id: u32| src::SourceEvent::Update {
        stream: src::Stream::Book,
        token: driver.context.book_token,
        updates: update(
            CURRENT,
            "paper-current",
            yes_id,
            "35",
            "70",
            Some(1),
            1000000,
        ),
        coverage: vec![],
    };
    let current_id = |driver: &Driver| mislabelled(driver, 3);
    let unknown_id = |driver: &Driver| mislabelled(driver, 99);
    let unknown_update = |driver: &Driver| src::SourceEvent::Update {
        stream: src::Stream::Book,
        token: driver.context.book_token,
        updates: update(
            "KXBTC15M-PAPER-UNKNOWN",
            "paper-unknown",
            7,
            "35",
            "70",
            Some(1),
            1000000,
        ),
        coverage: vec![],
    };
    let mixed = |driver: &Driver| {
        let mut updates = update(CURRENT, "paper-current", 1, "35", "70", Some(1), 1000000);
        updates.extend(update(NEXT, "paper-next", 3, "80", "25", Some(2), 1000000));
        src::SourceEvent::Update {
            stream: src::Stream::Book,
            token: driver.context.book_token,
            updates,
            coverage: vec![],
        }
    };
    let cases: [&dyn Fn(&Driver) -> src::SourceEvent; 8] = [
        &unknown,
        &unknown_loss,
        &unknown_update,
        &mixed,
        &current_coverage,
        &unknown_coverage,
        &current_id,
        &unknown_id,
    ];
    for event in cases {
        let dir = tempfile::tempdir().unwrap();
        let engine = Engine::open(dir.path().join("paper.sqlite")).unwrap();
        let mut driver = Driver::start(&engine);
        driver.prepare();
        let input = next(&driver, 61000, envelope(61000, 61000, event(&driver)));
        assert_eq!(
            engine.paper_input(input).unwrap_err().code,
            "unavailable_evidence"
        );
        let summary = engine.paper_summary(driver.context.run_id).unwrap();
        assert_eq!(summary.control_state, "stopped");
        assert_eq!(summary.error_code.as_deref(), Some("unavailable_evidence"));
    }
}

#[test]
fn burst_in_one_millisecond_latches_a_transient_crossing_and_keeps_its_side() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("paper.sqlite");
    let engine = Engine::open(&path).unwrap();
    let mut driver = Driver::start(&engine);
    let run = driver.context.run_id;
    kalshi(&mut driver, 0, src::Connection::Open);
    verify(&mut driver, 0, CURRENT);
    verify(&mut driver, 0, NEXT);
    send(
        &mut driver,
        0,
        update(NEXT, "paper-next", 3, "50", "70", None, 1000000),
    );
    driver.source(
        1000,
        src::SourceEvent::Boundary {
            market_id: NEXT.into(),
        },
    );
    assert!(latches(&engine, run).is_empty());
    // Three observations in the same millisecond; YES crosses below 40 only in the middle one.
    for (number, yes) in [(1, "50"), (2, "35"), (3, "50")] {
        driver.observe(5000, number, yes, "70");
    }
    assert_eq!(
        latches(&engine, run),
        [(NEXT.into(), "yes".into(), "5000".into())]
    );
    driver.observe(61000, 4, "80", "25");
    driver.input(
        61000,
        Payload::Book(driver.side_book(No, "25", 4, 61000, 10_000_000)),
    );
    assert_eq!(attempts(&path), [("full".into(), "no".into(), 10, 10)]);
}

#[test]
fn silence_expires_data_passes_the_deadline_and_closes_the_market_from_advance_alone() {
    // The quote is valid when accepted at 40000; the short lease ends at 50000, during silence.
    for (lease, expected) in [(50000, 0), (1000000, 1)] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("paper.sqlite");
        let engine = Engine::open(&path).unwrap();
        let mut driver = Driver::start(&engine);
        let run = driver.context.run_id;
        driver.enter();
        // The last quote: NO 25 would qualify for the opposite side once its window opens.
        send(
            &mut driver,
            40000,
            update(NEXT, "paper-next", 3, "80", "25", Some(1), lease),
        );
        let book = driver.side_book(No, "25", 1, 40000, 10_000_000);
        // Advance-only inputs carry time into the purchase window; no book has been sent yet.
        for at in [45000, 55000, 61000] {
            driver.input(at, Payload::Advance);
        }
        let receipt = engine
            .paper_history(Some(run), None)
            .unwrap()
            .markets
            .remove(0);
        let witness = receipt.primary_witness.unwrap();
        assert_eq!(witness["evaluated_at_ms"], "61000");
        assert_eq!(witness["observation_number"], "1");
        // The accepted NO 25 expired under Advance alone (unavailable, not the older NO 70's false).
        assert_eq!(
            witness["status"],
            if expected == 0 { "unavailable" } else { "true" }
        );
        assert!(attempts(&path).is_empty());
        driver.input(61500, Payload::Book(book));
        assert_eq!(attempts(&path).len(), expected);
        for at in [121000, 900999] {
            driver.input(at, Payload::Advance);
        }
        assert_eq!(driver.summary.control_state, "active");
        driver.input(901000, Payload::Advance);
        assert_eq!(driver.summary.control_state, "stopped");
        assert_eq!(attempts(&path).len(), expected);
        assert_eq!(participated(&engine, run), [NEXT]);
        if expected == 0 {
            assert_ne!(
                engine.paper_history(Some(run), None).unwrap().markets[0].reason,
                "purchases_paused"
            );
        }
    }
}

#[test]
fn clock_jumps_never_replay_missed_purchase_ticks_or_enter_late() {
    // A stored qualifying book, then one jump: the decision is made only at the target time.
    for (target, expected) in [(90000, 1), (150000, 0)] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("paper.sqlite");
        let engine = Engine::open(&path).unwrap();
        let mut driver = Driver::start(&engine);
        driver.enter();
        driver.observe(30000, 1, "80", "25");
        driver.input(
            30000,
            Payload::Book(driver.side_book(No, "25", 1, 30000, 10_000_000)),
        );
        assert!(attempts(&path).is_empty());
        driver.input(target, Payload::Advance);
        assert_eq!(attempts(&path).len(), expected);
    }
    // A jump over the opening: the market is never entered, even with qualifying evidence.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("paper.sqlite");
    let engine = Engine::open(&path).unwrap();
    let mut driver = Driver::start(&engine);
    let run = driver.context.run_id;
    kalshi(&mut driver, 0, src::Connection::Open);
    verify(&mut driver, 0, CURRENT);
    verify(&mut driver, 0, NEXT);
    driver.update(0, true);
    driver.input(62000, Payload::Advance);
    driver.source(
        62000,
        src::SourceEvent::Boundary {
            market_id: NEXT.into(),
        },
    );
    driver.observe(62000, 1, "80", "25");
    driver.input(
        62000,
        Payload::Book(driver.side_book(No, "25", 1, 62000, 10_000_000)),
    );
    assert!(attempts(&path).is_empty());
    assert!(participated(&engine, run).is_empty());
    assert!(latches(&engine, run).is_empty());
}

#[test]
fn delayed_settlement_releases_pending_risk_but_keeps_the_market_close_day() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("workspace.sqlite");
    let engine = Engine::open(&path).unwrap();
    let mut start = two_windows(&engine);
    start.go_utc_ms = 20_779 * 86_400_000 - 901_001;
    start.settings.limits = Some(DailyLimits {
        maximum_loss: 100 * DOLLAR,
        profit_target: Some(DOLLAR),
        resume_next_day: false,
    });
    let final_result = SettlementManifestEntry {
        source_market_id: start.markets[0].market_id.clone(),
        source_contract_id: start.markets[0].contract_id.clone(),
        close_ms: 901000,
        outcome_id: "final-a".into(),
        winning_side: No,
        available_ms: 902000,
    };
    start.settlement_manifest.push(final_result.clone());
    let mut driver = driver_for(&engine, start);
    enter_a(&mut driver);
    send(
        &mut driver,
        61000,
        update(
            "KXBTC15M-PAPER-A",
            "paper-a",
            1,
            "80",
            "25",
            Some(1),
            2000000,
        ),
    );
    driver.input(61000, Payload::Book(book_a(&driver, 1, 61000)));
    driver.source(
        901000,
        src::SourceEvent::Boundary {
            market_id: "KXBTC15M-PAPER-B".into(),
        },
    );
    driver.input(901001, Payload::Advance);
    let daily = driver.summary.daily_risk.as_ref().unwrap();
    assert_eq!(daily.utc_day, "20779");
    assert_eq!(daily.pending_holdings, "263130000");
    let first = driver.input(902000, Payload::Settlement(final_result.clone()));
    // A replay with a new ordinal also has no financial effect.
    driver.input(first.virtual_ms, Payload::Settlement(final_result.clone()));
    let daily = driver.summary.daily_risk.as_ref().unwrap();
    assert_eq!(daily.settled_net, "0");
    assert_eq!(daily.pending_holdings, "0");
    assert_eq!(daily.stop_reason, None);
    let history = engine
        .paper_history(Some(driver.context.run_id), None)
        .unwrap();
    let market = history
        .markets
        .iter()
        .find(|m| m.source_market_id == final_result.source_market_id)
        .unwrap();
    assert_eq!(market.close_day, "20778");
    assert_eq!(market.settled_net.as_deref(), Some("736870000"));
    assert_eq!(driver.summary.control_state, "active");

    let mut conflict = final_result;
    conflict.winning_side = Yes;
    let failure = engine
        .paper_input(next(&driver, 902000, Payload::Settlement(conflict)))
        .unwrap_err();
    assert_eq!(failure.code, "report_conflict");
    let run = driver.context.run_id;
    drop(engine);
    let engine = Engine::open(&path).unwrap();
    assert_eq!(
        engine.paper_summary(run).unwrap().error_code.as_deref(),
        Some("report_conflict")
    );
}
