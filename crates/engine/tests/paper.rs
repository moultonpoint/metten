use metten_engine::paper::*;
use metten_engine::strategy::Decimal;
fn price(s: &str) -> Decimal {
    Decimal::parse(s).unwrap()
}

#[test]
fn exact_full_zero_and_settlement_economics() {
    let c = charge(10, price("25")).unwrap();
    assert_eq!(
        c,
        Charge {
            cost: 250_000_000,
            fee: 13_130_000,
            debit: 263_130_000
        }
    );
    assert_eq!(100 * DOLLAR - c.debit, 9_736_870_000);
    assert_eq!(payout(10, true).unwrap() - c.debit, 736_870_000);
    assert_eq!(payout(10, false).unwrap() - c.debit, -263_130_000);
    // 9 × $0.25 = $2.25 plus 0.07 × 9 × 0.25 × 0.75 = $0.118125, rounded up to $2.3682.
    assert_eq!(
        simulate(10, price("25"), 9_999_999).unwrap(),
        Fill::Partial {
            quantity: 9,
            charge: Charge {
                cost: 225_000_000,
                fee: 11_820_000,
                debit: 236_820_000
            }
        }
    );
    assert_eq!(
        simulate(10, price("25"), 999_999).unwrap(),
        Fill::Zero {
            available_millionths: 999_999
        }
    );
    assert_eq!(
        simulate(10, price("25"), 10_000_000).unwrap(),
        Fill::Full {
            quantity: 10,
            charge: c
        }
    );
}

#[test]
fn recorded_simulation_model_keeps_its_own_depth_description() {
    assert!(simulation_depth(SIMULATION_MODEL).contains("cancels the rest"));
    assert!(simulation_depth("controlled_best_ask_full_or_zero_v1").contains("no partial fills"));
}

#[test]
fn combined_rounding_preserves_fractional_cent_cost() {
    let c = charge(1, price("25.000001")).unwrap();
    assert_eq!(c.cost, 25_000_001);
    assert_eq!(c.debit, 26_320_000);
    assert_eq!(c.fee, 1_319_999);
    assert_eq!(charge(1, price("0")).unwrap().debit, 0);
    assert_eq!(charge(1, price("100")).unwrap().debit, DOLLAR);
}

#[test]
fn whole_objective_is_funded_and_prices_are_inclusive() {
    let s = Settings {
        quantity: 10,
        minimum_price: price("25"),
        maximum_price: price("30"),
        starting_balance: 100 * DOLLAR,
    };
    let c = charge(10, price("25")).unwrap();
    assert_eq!(funded_charge(&s, price("25"), c.debit, 0), Ok(c));
    assert!(funded_charge(&s, price("30"), 100 * DOLLAR, 0).is_ok());
    assert_eq!(
        funded_charge(&s, price("25"), c.debit - 1, 0),
        Err(PaperError::InsufficientFunds)
    );
    assert_eq!(
        funded_charge(&s, price("25"), c.debit, 1),
        Err(PaperError::InsufficientFunds)
    );
    assert_eq!(
        funded_charge(&s, price("24.999999"), 100 * DOLLAR, 0),
        Err(PaperError::Price)
    );
    assert_eq!(charge(u64::MAX, price("25")), Err(PaperError::Range));
    assert_eq!(charge(1, price("100.000001")), Err(PaperError::Range));
    assert_eq!(charge(1, price("-0.000001")), Err(PaperError::Range));
    assert!(
        Settings {
            quantity: 0,
            ..s.clone()
        }
        .validate()
        .is_err()
    );
    assert!(
        Settings {
            starting_balance: 0,
            ..s.clone()
        }
        .validate()
        .is_err()
    );
    assert!(
        Settings {
            minimum_price: price("31"),
            ..s
        }
        .validate()
        .is_err()
    );
}

use metten_engine::{Engine, StrategyRequest, StrategyResponse};
use metten_engine::{sources as src, strategy as rules};
#[path = "support/paper.rs"]
mod fixture;
use fixture::{Driver, saved, scenario};

#[test]
fn checkpoint_latches_survive_witness_cap_and_stopped_reopen() {
    for preceding in [11, 31] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("paper.sqlite");
        let engine = Engine::open(&path).unwrap();
        let mut rules: serde_json::Value =
            serde_json::from_str(include_str!("../../../examples/learning.json")).unwrap();
        let checkpoint = rules["checkpoints"][0].clone();
        let checkpoints = rules["checkpoints"].as_array_mut().unwrap();
        for index in 0..preceding {
            let mut earlier = checkpoint.clone();
            earlier["id"] = serde_json::json!(format!("earlier_{index}"));
            checkpoints.insert(index, earlier);
        }
        let json = rules.to_string();
        let StrategyResponse::Saved { version } = engine
            .strategies(StrategyRequest::Save {
                json: json.clone(),
                reviewed_json: json,
            })
            .unwrap()
        else {
            panic!("save failed")
        };
        let reply = engine
            .paper_start(scenario(version.id.parse().unwrap()))
            .unwrap();
        let mut driver = Driver {
            engine: &engine,
            context: reply.context.unwrap(),
            ordinal: 0,
            summary: reply.summary,
        };
        driver.prepare();
        let run = driver.context.run_id;
        driver.input(61000, Payload::Book(driver.book(10_000_000)));
        let market = engine.paper_history(Some(run), None).unwrap().markets[0]
            .paper_market_id
            .parse()
            .unwrap();
        engine.stop_and_save().unwrap();
        drop(engine);
        let engine = Engine::open(&path).unwrap();
        let mut cursor = None;
        let mut latches = Vec::new();
        loop {
            let page = engine.paper_evidence(run, market, cursor).unwrap();
            for record in page.evidence {
                if record.witness["kind"] == "checkpoint_latch" {
                    assert_eq!(record.first_virtual_ms, "1000");
                    assert_eq!(record.last_virtual_ms, "1000");
                    assert_eq!(record.repetitions, "1");
                    latches.push(record.witness);
                }
            }
            cursor = page.next_cursor.map(|id| id.parse().unwrap());
            if cursor.is_none() {
                break;
            }
        }
        assert_eq!(latches.len(), preceding + 1);
        let original = latches
            .iter()
            .find(|latch| latch["checkpoint_id"] == "early_side_checkpoint")
            .unwrap();
        assert_eq!(original["side"], "yes");
        assert_eq!(original["evaluated_at_ms"], "1000");
        assert_eq!(original["transaction"], "2");
        let receipt = engine.paper_market(run, market).unwrap();
        assert_eq!(receipt.filled_quantity.as_deref(), Some("10"));
        assert_eq!(receipt.side.as_deref(), Some("no"));
    }
}

#[test]
fn public_engine_learning_strategy_records_full_purchase_then_exact_settlement() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("paper.sqlite");
    let engine = Engine::open(&path).unwrap();
    let mut driver = Driver::start(&engine);
    driver.prepare();
    let input = driver.input(61000, Payload::Book(driver.book(10_000_000)));
    assert_eq!(driver.summary.cash, "9736870000");
    assert!(driver.summary.pending_holdings);
    assert_eq!(driver.summary.reserved, "0");
    let revision = driver.summary.revision.clone();
    assert_eq!(
        engine.paper_input(input).unwrap().summary.revision,
        revision
    );
    let db = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        db.query_row(
            "SELECT side,filled_quantity,cost,fee FROM paper_attempts",
            [],
            |r| Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, i64>(3)?
            ))
        )
        .unwrap(),
        ("no".into(), 10, 250000000, 13130000)
    );
    let entry = scenario(1).settlement_manifest.remove(0);
    driver.input(901000, Payload::Settlement(entry));
    assert_eq!(driver.summary.cash, "10736870000");
    assert!(!driver.summary.pending_holdings);
    assert_eq!(driver.summary.control_state, "stopped");
    assert_eq!(
        db.query_row("SELECT payout,net FROM paper_settlements", [], |r| Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, i64>(1)?
        )))
        .unwrap(),
        (1000000000, 736870000)
    );
}

#[test]
fn public_engine_stopped_reopen_settles_saved_manifest_without_source_replay() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("paper.sqlite");
    let run;
    let ordinal;
    let evaluated;
    {
        let engine = Engine::open(&path).unwrap();
        let mut driver = Driver::start(&engine);
        driver.prepare();
        driver.input(61000, Payload::Book(driver.book(10_000_000)));
        run = driver.context.run_id;
        ordinal = driver.summary.completed_ordinal.clone();
        evaluated = driver.summary.last_evaluated_through_ms.clone();
        engine.stop_and_save().unwrap();
    }
    let engine = Engine::open(&path).unwrap();
    assert_eq!(engine.paper_summary(run).unwrap().cash, "9736870000");
    let reply = engine.paper_check_settlement(run).unwrap();
    assert_eq!(reply.summary.cash, "10736870000");
    assert_eq!(reply.summary.virtual_frontier_ms, "901000");
    assert_eq!(reply.summary.last_evaluated_through_ms, evaluated);
    assert_eq!(reply.summary.completed_ordinal, ordinal);
    assert_eq!(reply.summary.control_state, "stopped");
    assert!(reply.context.is_none());
    assert_eq!(
        engine.paper_check_settlement(run).unwrap().summary,
        reply.summary
    );
}

fn attempts(path: &std::path::Path) -> Vec<(String, String, i64, i64, i64, i64)> {
    let db = rusqlite::Connection::open(path).unwrap();
    db.prepare("SELECT state,side,quantity,filled_quantity,cost,fee FROM paper_attempts ORDER BY attempt_id")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}
fn latest_receipt(driver: &Driver) -> MarketReceipt {
    driver
        .engine
        .paper_history(Some(driver.context.run_id), None)
        .unwrap()
        .markets
        .remove(0)
}

#[test]
fn public_engine_partial_fills_complete_only_the_original_locked_objective() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("paper.sqlite");
    let engine = Engine::open(&path).unwrap();
    let mut driver = Driver::start(&engine);
    driver.prepare();
    let no = rules::Side::No;
    // 4 of 10 fill: $1.00 cost, 0.07×4×0.25×0.75 = $0.0525 fee.
    driver.input(61000, Payload::Book(driver.book(4_000_000)));
    assert_eq!(driver.summary.cash, "9894750000");
    assert_eq!(driver.summary.reserved, "0");
    let receipt = latest_receipt(&driver);
    assert_eq!(receipt.attempt_state.as_deref(), Some("partial"));
    assert_eq!(receipt.filled_quantity.as_deref(), Some("4"));
    assert_eq!(receipt.reason, "confirmed_partial_fill_pending_settlement");
    // The same observation restated with more depth is unchanged data: no second IOC.
    driver.input(
        61500,
        Payload::Book(driver.side_book(no, "25", 1, 61500, 100_000_000)),
    );
    driver.input(61600, Payload::Advance);
    // Re-initializing the same ask changes the value transaction, not the observation.
    let spent = driver.context.last_value_transaction;
    driver.reinitialize(61700, "80", "25");
    assert_ne!(driver.context.last_value_transaction, spent);
    driver.input(
        61700,
        Payload::Book(driver.side_book(no, "25", 1, 61700, 100_000_000)),
    );
    assert_eq!(attempts(&path).len(), 1);
    // The retained check prices the 6-contract remainder: $1.50 + $0.07875 → $1.5788.
    assert_eq!(
        latest_receipt(&driver).purchase_check.unwrap()["required_debit"],
        "157880000"
    );
    // A new observation funds a retry of exactly the 6 unfilled contracts; 3 fill.
    driver.observe(62000, 2, "80", "25");
    driver.input(
        62000,
        Payload::Book(driver.side_book(no, "25", 2, 62000, 3_500_000)),
    );
    // Only YES qualifies now; the side is locked to NO after a fill, so nothing is sent.
    driver.observe(63000, 3, "20", "35");
    driver.input(
        63000,
        Payload::Book(driver.side_book(rules::Side::Yes, "20", 3, 63000, 100_000_000)),
    );
    // NO qualifies again; the retry asks for the remaining 3 and completes the objective.
    driver.observe(64000, 4, "80", "25");
    driver.input(
        64000,
        Payload::Book(driver.side_book(no, "25", 4, 64000, 100_000_000)),
    );
    // Complete: later eligible liquidity never becomes a second objective.
    driver.observe(65000, 5, "80", "25");
    driver.input(
        65000,
        Payload::Book(driver.side_book(no, "25", 5, 65000, 100_000_000)),
    );
    assert_eq!(
        attempts(&path),
        vec![
            ("partial".into(), "no".into(), 10, 4, 100_000_000, 5_250_000),
            ("partial".into(), "no".into(), 6, 3, 75_000_000, 3_940_000),
            ("full".into(), "no".into(), 3, 3, 75_000_000, 3_940_000),
        ]
    );
    assert_eq!(driver.summary.cash, "9736870000");
    let receipt = latest_receipt(&driver);
    assert_eq!(receipt.attempt_state.as_deref(), Some("full"));
    assert_eq!(receipt.attempt_count, "3");
    assert_eq!(receipt.side.as_deref(), Some("no"));
    assert_eq!(receipt.filled_quantity.as_deref(), Some("10"));
    assert_eq!(receipt.cost.as_deref(), Some("250000000"));
    assert_eq!(receipt.fee.as_deref(), Some("13130000"));
    assert_eq!(receipt.debit.as_deref(), Some("263130000"));
    let entry = scenario(1).settlement_manifest.remove(0);
    driver.input(901000, Payload::Settlement(entry));
    // NO wins: 10 × $1 payout on the combined holding.
    assert_eq!(driver.summary.cash, "10736870000");
    assert_eq!(
        latest_receipt(&driver).settled_net.as_deref(),
        Some("736870000")
    );
    drop(engine);
    // Reopen reconciles cash, reservations and the combined settlement.
    let engine = Engine::open(&path).unwrap();
    assert_eq!(engine.paper_latest().unwrap().unwrap().cash, "10736870000");
}

#[test]
fn public_engine_reconnect_never_regrants_a_spent_book() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("paper.sqlite");
    let engine = Engine::open(&path).unwrap();
    let mut driver = Driver::start(&engine);
    driver.prepare();
    driver.input(61000, Payload::Book(driver.book(4_000_000)));
    let generation = driver.context.generation;
    for (at, state) in [
        (61100, src::Connection::Disconnected),
        (61200, src::Connection::Open),
    ] {
        driver.source(
            at,
            src::SourceEvent::Connection {
                provider: src::Provider::Kalshi,
                token: driver.context.book_token,
                state,
            },
        );
    }
    driver.source(
        61200,
        src::SourceEvent::Metadata {
            token: driver.context.metadata_token,
            market_id: "KXBTC15M-PAPER-NEXT".into(),
            verified: true,
        },
    );
    // A new source generation re-initializes the same ask; no observation is newer.
    driver.reinitialize(61300, "80", "25");
    assert_ne!(driver.context.generation, generation);
    engine
        .paper_control(driver.context.run_id, Control::Resume)
        .unwrap();
    driver.input(
        61400,
        Payload::Book(driver.side_book(rules::Side::No, "25", 1, 61400, 4_000_000)),
    );
    assert_eq!(attempts(&path).len(), 1);
    // The next genuine observation, numbered above the spent book, may fund a retry; numbers
    // beyond i64 (here 2^63) must compare exactly.
    let number = 1 << 63;
    driver.observe(62000, number, "80", "25");
    driver.input(
        62000,
        Payload::Book(driver.side_book(rules::Side::No, "25", number, 62000, 100_000_000)),
    );
    assert_eq!(attempts(&path).len(), 2);
}

#[test]
fn public_engine_zero_fills_are_terminal_and_retries_end_with_eligibility() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("paper.sqlite");
    let engine = Engine::open(&path).unwrap();
    let mut driver = Driver::start(&engine);
    driver.prepare();
    let yes = rules::Side::Yes;
    // Under one contract available: a confirmed zero fill, no fee.
    driver.input(61000, Payload::Book(driver.book(999_999)));
    // Restating that unchanged observation with depth never grants new liquidity.
    driver.input(62000, Payload::Book(driver.book(100_000_000)));
    assert_eq!(driver.summary.cash, "10000000000");
    assert_eq!(latest_receipt(&driver).reason, "insufficient_quantity");
    // Before any fill the side may change once the prior attempt has finished.
    driver.observe(63000, 2, "20", "35");
    driver.input(
        63000,
        Payload::Book(driver.side_book(yes, "20", 2, 63000, 0)),
    );
    driver.observe(64000, 3, "20", "35");
    // 5 × $0.20 = $1.00 plus 0.07 × 5 × 0.20 × 0.80 = $0.056 fee.
    driver.input(
        64000,
        Payload::Book(driver.side_book(yes, "20", 3, 64000, 5_000_000)),
    );
    assert_eq!(driver.summary.cash, "9894400000");
    // After the purchase window ends, eligible-looking liquidity is never used.
    driver.observe(125000, 4, "20", "25");
    driver.input(
        125000,
        Payload::Book(driver.side_book(yes, "20", 4, 125000, 100_000_000)),
    );
    assert_eq!(
        attempts(&path),
        vec![
            ("zero".into(), "no".into(), 10, 0, 0, 0),
            ("zero".into(), "yes".into(), 10, 0, 0, 0),
            (
                "partial".into(),
                "yes".into(),
                10,
                5,
                100_000_000,
                5_600_000
            ),
        ]
    );
    // The purchased contracts are retained through settlement; NO wins, so YES pays nothing.
    let entry = scenario(1).settlement_manifest.remove(0);
    driver.input(901000, Payload::Settlement(entry));
    let receipt = latest_receipt(&driver);
    assert_eq!(receipt.attempt_state.as_deref(), Some("partial"));
    assert_eq!(receipt.filled_quantity.as_deref(), Some("5"));
    assert_eq!(receipt.settled_net.as_deref(), Some("-105600000"));
    assert_eq!(driver.summary.cash, "9894400000");
}

#[test]
fn public_engine_receipts_reject_old_gap_and_conflicting_inputs_without_reapplication() {
    for conflict in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("paper.sqlite");
        let engine = Engine::open(&path).unwrap();
        let mut driver = Driver::start(&engine);
        let older = driver.input(0, Payload::Advance);
        let latest = driver.input(1, Payload::Advance);
        assert_eq!(engine.paper_input(older).unwrap_err().code, "stale_input");
        let before = driver.summary.completed_ordinal.clone();
        let mut bad = latest.clone();
        if conflict {
            bad.virtual_ms = 2;
        } else {
            bad.ordinal += 2;
        }
        assert_eq!(
            engine.paper_input(bad).unwrap_err().code,
            if conflict {
                "input_conflict"
            } else {
                "input_gap"
            }
        );
        assert_eq!(
            engine
                .paper_summary(driver.context.run_id)
                .unwrap()
                .completed_ordinal,
            before
        );
        assert_eq!(
            engine.paper_input(latest).unwrap_err().code,
            "stopped_context"
        );
    }
}

#[test]
fn public_engine_pause_requires_new_book_and_stop_before_entry_does_not_participate() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("paper.sqlite");
    let engine = Engine::open(&path).unwrap();
    let mut driver = Driver::start(&engine);
    driver.prepare();
    engine
        .paper_control(driver.context.run_id, Control::Pause)
        .unwrap();
    driver.input(61000, Payload::Book(driver.book(10_000_000)));
    assert_eq!(driver.summary.cash, "10000000000");
    engine
        .paper_control(driver.context.run_id, Control::Resume)
        .unwrap();
    driver.input(62000, Payload::Advance);
    assert_eq!(driver.summary.cash, "10000000000");
    let mut fresh = driver.book(10_000_000);
    fresh.accepted_ms = 62001;
    driver.input(62001, Payload::Book(fresh));
    assert_eq!(driver.summary.cash, "9736870000");
    engine.stop_and_save().unwrap();
    drop(engine);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("stopped.sqlite");
    let engine = Engine::open(&path).unwrap();
    let driver = Driver::start(&engine);
    assert_eq!(
        engine
            .paper_control(driver.context.run_id, Control::StopAfterMarket)
            .unwrap()
            .summary
            .control_state,
        "stopped"
    );
    let db = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        db.query_row("SELECT sum(participated) FROM paper_markets", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn public_engine_coherence_mismatch_never_commits_an_intent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("paper.sqlite");
    let engine = Engine::open(&path).unwrap();
    let mut driver = Driver::start(&engine);
    driver.prepare();
    let mut mismatch = driver.book(10_000_000);
    mismatch.ask = price("35");
    driver.input(61000, Payload::Book(mismatch));
    let mut replacement = driver.book(10_000_000);
    replacement.value_transaction += 1;
    driver.input(61000, Payload::Book(replacement));
    let db = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        db.query_row("SELECT count(*) FROM paper_attempts", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    driver.input(61000, Payload::Book(driver.book(10_000_000)));
    assert_eq!(driver.summary.cash, "9736870000");
}

#[test]
fn public_engine_failed_outcome_commit_is_reconciled_from_the_venue_on_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("paper.sqlite");
    let run;
    let db = rusqlite::Connection::open(&path).unwrap();
    {
        let engine = Engine::open(&path).unwrap();
        let mut driver = Driver::start(&engine);
        driver.prepare();
        run = driver.context.run_id;
        db.execute_batch("CREATE TRIGGER fail_paper_outcome BEFORE UPDATE OF state ON paper_attempts WHEN NEW.state='full' BEGIN SELECT RAISE(ABORT,'controlled failure'); END").unwrap();
        let input = Input {
            context: driver.context,
            ordinal: driver.ordinal + 1,
            virtual_ms: 61000,
            payload: Payload::Book(driver.book(10_000_000)),
        };
        assert_eq!(
            engine.paper_input(input.clone()).unwrap_err().code,
            "storage"
        );
        assert_eq!(
            engine.paper_input(input).unwrap_err().code,
            "stopped_context"
        );
        // The venue accepted and filled the order; the engine has not recorded the outcome.
        assert_eq!(
            db.query_row("SELECT state FROM paper_attempts", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "dispatched"
        );
        assert_eq!(
            db.query_row("SELECT filled_quantity FROM paper_venue_orders", [], |r| {
                r.get::<_, i64>(0)
            })
            .unwrap(),
            10
        );
        let preserved = engine.paper_summary(run).unwrap();
        assert_eq!(preserved.cash, "10000000000");
        assert_eq!(preserved.reserved, "263130000");
        assert_eq!(preserved.control_state, "unresolved");
        assert_eq!(preserved.error_code.as_deref(), Some("storage"));
        let mut conflicting = scenario(driver.summary.strategy_version_id.parse().unwrap());
        conflicting.settings.quantity = 11;
        assert_eq!(
            engine.paper_start(conflicting).unwrap_err().code,
            "input_conflict"
        );
        assert_eq!(
            engine.paper_control(run, Control::Resume).unwrap_err().code,
            "stale_run"
        );
        assert!(engine.paper_effective().0);
        // While the outcome still cannot be recorded, the check fails closed and keeps the hold.
        assert_eq!(
            engine.paper_check_settlement(run).unwrap_err().code,
            "storage"
        );
        assert_eq!(engine.paper_summary(run).unwrap().reserved, "263130000");
    }
    db.execute_batch("DROP TRIGGER fail_paper_outcome").unwrap();
    let engine = Engine::open(&path).unwrap();
    let recovered = engine.paper_summary(run).unwrap();
    assert_eq!(recovered.control_state, "stopped");
    assert_eq!(recovered.error_code, None);
    assert_eq!(recovered.cash, "9736870000");
    assert_eq!(recovered.reserved, "0");
    let recovery = recovered.recovery.unwrap();
    assert_eq!(recovery.state, "full");
    assert_eq!(recovery.interrupted.as_deref(), Some("no_outcome"));
    // Recovery never resumes purchases; the holding settles once from the saved manifest.
    let another = || {
        let mut another = scenario(1);
        another.start_uuid = "00000000-0000-4000-8000-000000000002".into();
        another
    };
    assert_eq!(
        engine.paper_start(another()).unwrap_err().code,
        "run_active"
    );
    assert_eq!(
        engine.paper_check_settlement(run).unwrap().summary.cash,
        "10736870000"
    );
    assert!(engine.paper_start(another()).is_ok());
}

#[test]
fn public_engine_failed_receipt_boundaries_do_not_reapply_financial_effects() {
    for after_outcome in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("paper.sqlite");
        let run;
        {
            let engine = Engine::open(&path).unwrap();
            let mut driver = Driver::start(&engine);
            driver.prepare();
            run = driver.context.run_id;
            let db = rusqlite::Connection::open(&path).unwrap();
            db.execute_batch(if after_outcome {"CREATE TRIGGER fail_receipt BEFORE UPDATE OF completed_ordinal ON paper_runs WHEN NEW.completed_ordinal>OLD.completed_ordinal BEGIN SELECT RAISE(ABORT,'controlled B1 failure'); END"}else{"CREATE TRIGGER fail_receipt BEFORE UPDATE OF pending_ordinal ON paper_runs WHEN NEW.pending_ordinal IS NOT NULL BEGIN SELECT RAISE(ABORT,'controlled B0 failure'); END"}).unwrap();
            let input = Input {
                context: driver.context,
                ordinal: driver.ordinal + 1,
                virtual_ms: 61000,
                payload: Payload::Book(driver.book(10_000_000)),
            };
            assert_eq!(
                engine.paper_input(input.clone()).unwrap_err().code,
                "storage"
            );
            assert_eq!(
                engine.paper_input(input).unwrap_err().code,
                "stopped_context"
            );
            let summary = engine.paper_summary(run).unwrap();
            assert_eq!(summary.completed_ordinal, driver.summary.completed_ordinal);
            if after_outcome {
                assert_eq!(summary.cash, "9736870000");
                assert_eq!(
                    summary.pending_ordinal,
                    Some((driver.ordinal + 1).to_string())
                );
            } else {
                assert_eq!(summary.cash, "10000000000");
                assert!(summary.pending_ordinal.is_none());
            }
            assert_eq!(
                db.query_row("SELECT count(*) FROM paper_attempts", [], |r| r
                    .get::<_, i64>(0))
                    .unwrap(),
                if after_outcome { 1 } else { 0 }
            );
            db.execute_batch("DROP TRIGGER fail_receipt").unwrap();
        }
        let engine = Engine::open(&path).unwrap();
        let summary = engine.paper_check_settlement(run).unwrap().summary;
        assert_eq!(
            summary.cash,
            if after_outcome {
                "10736870000"
            } else {
                "10000000000"
            }
        );
    }
}

#[test]
fn public_engine_failed_settlement_preserves_cash_frontier_and_can_retry_once() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("paper.sqlite");
    let run;
    {
        let engine = Engine::open(&path).unwrap();
        let mut driver = Driver::start(&engine);
        driver.prepare();
        driver.input(61000, Payload::Book(driver.book(10_000_000)));
        run = driver.context.run_id;
        engine.stop_and_save().unwrap();
    }
    let engine = Engine::open(&path).unwrap();
    let before = engine.paper_summary(run).unwrap();
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch("CREATE TRIGGER fail_settlement BEFORE INSERT ON paper_settlements BEGIN SELECT RAISE(ABORT,'controlled E failure'); END").unwrap();
    assert_eq!(
        engine.paper_check_settlement(run).unwrap_err().code,
        "storage"
    );
    assert_eq!(engine.paper_summary(run).unwrap(), before);
    db.execute_batch("DROP TRIGGER fail_settlement").unwrap();
    let settled = engine.paper_check_settlement(run).unwrap().summary;
    assert_eq!(settled.cash, "10736870000");
    assert_eq!(engine.paper_check_settlement(run).unwrap().summary, settled);
}

#[test]
fn public_engine_unaffordable_objective_is_skipped_without_resize() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("paper.sqlite");
    let engine = Engine::open(&path).unwrap();
    let mut start = scenario(saved(&engine));
    start.settings.starting_balance = DOLLAR;
    let reply = engine.paper_start(start).unwrap();
    let mut driver = Driver {
        engine: &engine,
        context: reply.context.unwrap(),
        ordinal: 0,
        summary: reply.summary,
    };
    driver.prepare();
    driver.input(61000, Payload::Book(driver.book(10_000_000)));
    assert_eq!(driver.summary.cash, DOLLAR.to_string());
    assert_eq!(driver.summary.pending_ordinal, None);
    let db = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        db.query_row("SELECT count(*) FROM paper_attempts", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        db.query_row(
            "SELECT reason FROM paper_markets WHERE participated=1",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "insufficient_funds"
    );
    assert_eq!(
        db.query_row("SELECT quantity FROM paper_runs", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        10
    );
}

#[test]
fn public_engine_history_retains_exact_rule_witnesses_and_read_only_pages() {
    let directory = tempfile::tempdir().unwrap();
    let engine = Engine::open(directory.path().join("paper.sqlite")).unwrap();
    let mut driver = Driver::start(&engine);
    driver.prepare();
    let run = driver.context.run_id;
    let page = engine.paper_history(Some(run), None).unwrap();
    assert_eq!(page.markets.len(), 1);
    let market = page.markets[0].paper_market_id.parse().unwrap();
    let receipt = engine.paper_market(run, market).unwrap();
    let witness = receipt.primary_witness.unwrap();
    assert_eq!(witness["observed"], "25");
    assert_eq!(witness["required"], "30");
    assert_eq!(witness["side"], "no");
    assert_eq!(witness["evaluated_at_ms"], "61000");
    let summary = engine.paper_summary(run).unwrap();
    let page = engine.paper_evidence(run, market, None).unwrap();
    assert!(!page.evidence.is_empty());
    assert!(page.evidence.len() <= 20);
    if let Some(cursor) = page.next_cursor {
        let next = engine
            .paper_evidence(run, market, Some(cursor.parse().unwrap()))
            .unwrap();
        assert!(
            next.evidence
                .iter()
                .all(|e| e.evidence_id.parse::<i64>().unwrap() < cursor.parse().unwrap())
        );
    }
    assert_eq!(summary, engine.paper_summary(run).unwrap());
    assert!(engine.paper_market(run + 1, market).is_err());
    assert!(engine.paper_evidence(run + 1, market, None).is_err());
}

#[test]
fn public_engine_start_uuid_rejects_conflicting_market_identity_and_reopen_checks_money() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("paper.sqlite");
    let engine = Engine::open(&path).unwrap();
    let version = saved(&engine);
    let start = scenario(version);
    let initial = engine.paper_start(start.clone()).unwrap();
    assert_eq!(
        engine.paper_start(start.clone()).unwrap().summary.run_id,
        initial.summary.run_id
    );
    let mut conflict = start;
    conflict.markets[0].market_id.push('x');
    assert_eq!(
        engine.paper_start(conflict).unwrap_err().code,
        "input_conflict"
    );
    engine.stop_and_save().unwrap();
    drop(engine);
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute("UPDATE paper_runs SET cash=cash+1", []).unwrap();
    drop(db);
    assert!(Engine::open(&path).is_err());
    let db = rusqlite::Connection::open(path).unwrap();
    assert_eq!(
        db.query_row("SELECT cash FROM paper_runs", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        100 * DOLLAR + 1
    );
}

#[test]
fn reopen_refuses_a_settlement_for_a_market_with_no_fills() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("paper.sqlite");
    let engine = Engine::open(&path).unwrap();
    engine.paper_start(scenario(saved(&engine))).unwrap();
    engine.stop_and_save().unwrap();
    drop(engine);
    let db = rusqlite::Connection::open(&path).unwrap();
    // Payout 0 leaves cash consistent, so only the settlement check can catch it.
    assert_eq!(db.execute("INSERT INTO paper_settlements(run_id,paper_market_id,outcome_id,outcome_revision,winning_side,effective_virtual_ms,held_quantity,payout,net,close_day) SELECT run_id,paper_market_id,'tampered','synthetic_final_v1','yes',0,1,0,0,close_day FROM paper_markets LIMIT 1", []).unwrap(), 1);
    drop(db);
    assert!(Engine::open(&path).is_err());
    let db = rusqlite::Connection::open(path).unwrap();
    assert_eq!(
        db.query_row("SELECT outcome_id FROM paper_settlements", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "tampered"
    );
}

#[test]
fn public_engine_price_refusal_keeps_the_first_specific_check_at_market_end() {
    let directory = tempfile::tempdir().unwrap();
    let engine = Engine::open(directory.path().join("paper.sqlite")).unwrap();
    let mut start = scenario(saved(&engine));
    start.settings.minimum_price = price("26");
    let reply = engine.paper_start(start).unwrap();
    let mut driver = Driver {
        engine: &engine,
        context: reply.context.unwrap(),
        ordinal: 0,
        summary: reply.summary,
    };
    driver.prepare();
    let run = driver.context.run_id;
    let page = engine.paper_history(Some(run), None).unwrap();
    assert_eq!(page.markets[0].reason, "price");
    assert_eq!(
        page.markets[0].purchase_check.as_ref().unwrap()["offered_price"],
        "25"
    );
    driver.input(901000, Payload::Advance);
    let receipt = engine
        .paper_history(Some(run), None)
        .unwrap()
        .markets
        .remove(0);
    assert_eq!(receipt.reason, "price");
    assert_eq!(receipt.purchase_check.unwrap()["minimum_price"], "26");
    assert!(receipt.primary_witness.unwrap().is_object());
    assert_eq!(receipt.attempt_state, None);
    assert_eq!(driver.summary.cash, "10000000000");
}

#[test]
fn public_engine_retry_checks_live_event_token_and_owned_input_capacity_before_receipt() {
    let directory = tempfile::tempdir().unwrap();
    let a = Engine::open(directory.path().join("a.sqlite")).unwrap();
    let b = Engine::open(directory.path().join("b.sqlite")).unwrap();
    let mut first = Driver::start(&a);
    let second = Driver::start(&b);
    let accepted = first.source(
        0,
        src::SourceEvent::Connection {
            provider: src::Provider::Kalshi,
            token: first.context.book_token,
            state: src::Connection::Open,
        },
    );
    let summary = first.summary.clone();
    let mut forged = accepted.clone();
    if let Payload::Source(envelope) = &mut forged.payload
        && let src::SourceEvent::Connection { token, .. } = &mut envelope.event
    {
        *token = second.context.book_token;
    }
    assert_eq!(a.paper_input(forged).unwrap_err().code, "stopped_context");
    assert_eq!(a.paper_input(accepted).unwrap().summary, summary);
    let mut spare = String::with_capacity(129);
    spare.push_str("KXBTC15M-PAPER-NEXT");
    let mut book = first.book(100_000_000);
    book.source_market_id = spare;
    assert!(
        a.paper_input(Input {
            context: first.context,
            ordinal: first.ordinal + 1,
            virtual_ms: 0,
            payload: Payload::Book(book)
        })
        .is_err()
    );
    assert_eq!(a.paper_summary(first.context.run_id).unwrap(), summary);
}

#[test]
fn public_engine_later_pause_survives_an_older_resume_completion() {
    use std::sync::Arc;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("paper.sqlite");
    let engine = Arc::new(Engine::open(&path).unwrap());
    let mut driver = Driver::start(&engine);
    driver.prepare();
    let run = driver.context.run_id;
    engine.paper_control(run, Control::Pause).unwrap();
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch("BEGIN IMMEDIATE").unwrap();
    let worker = engine.clone();
    let thread = std::thread::spawn(move || {
        loop {
            match worker.paper_control(run, Control::Resume) {
                Err(error) if error.code == "busy" => std::thread::yield_now(),
                reply => break reply,
            }
        }
    });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
    loop {
        match engine.paper_context(run) {
            Err(error) if error.code == "busy" => break,
            _ => assert!(std::time::Instant::now() < deadline),
        }
        std::thread::yield_now();
    }
    engine.fence_paper_control(run, Control::Pause).unwrap();
    db.execute_batch("ROLLBACK").unwrap();
    let reply = thread.join().unwrap().unwrap();
    assert!(reply.effective_paused);
    assert!(engine.paper_effective().0);
    let mut book = driver.book(100_000_000);
    book.accepted_ms = 62000;
    driver.input(62000, Payload::Book(book));
    assert_eq!(driver.summary.cash, "10000000000");
}

#[test]
fn historical_start_retry_does_not_replace_current_run_or_control_target() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::open(dir.path().join("paper.sqlite")).unwrap();
    let first = scenario(saved(&engine));
    let one = engine.paper_start(first.clone()).unwrap();
    engine
        .paper_control(one.context.unwrap().run_id, Control::StopAfterMarket)
        .unwrap();
    let mut second = first.clone();
    second.start_uuid = "00000000-0000-4000-8000-000000000002".into();
    let two = engine.paper_start(second).unwrap();
    let historical = engine.paper_start(first).unwrap();
    assert_eq!(historical.summary.run_id, one.summary.run_id);
    assert_eq!(
        engine.paper_latest().unwrap().unwrap().run_id,
        two.summary.run_id
    );
    assert!(
        engine
            .paper_control(two.context.unwrap().run_id, Control::Pause)
            .unwrap()
            .effective_paused
    );
}

#[test]
fn failed_start_leaves_a_different_open_run_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::open(dir.path().join("paper.sqlite")).unwrap();
    let first = scenario(saved(&engine));
    let one = engine.paper_start(first.clone()).unwrap();
    engine
        .paper_control(one.context.unwrap().run_id, Control::StopAfterMarket)
        .unwrap();
    let mut second = first.clone();
    second.start_uuid = "00000000-0000-4000-8000-000000000002".into();
    let two = engine.paper_start(second.clone()).unwrap();
    let mut driver = Driver {
        engine: &engine,
        context: two.context.unwrap(),
        ordinal: 0,
        summary: two.summary,
    };
    let run = driver.context.run_id;
    let before = engine.paper_summary(run).unwrap();
    let mut changed = first.clone();
    changed.settings.quantity = 9;
    assert_eq!(
        engine.paper_start(changed).unwrap_err().code,
        "input_conflict"
    );
    let mut missing = first;
    missing.start_uuid = "00000000-0000-4000-8000-000000000003".into();
    missing.strategy_version_id = 999;
    assert!(engine.paper_start(missing).is_err());
    assert_eq!(engine.paper_summary(run).unwrap(), before);
    assert_eq!(before.error_code, None);
    driver.context = engine.paper_context(run).unwrap();
    driver.prepare();
    assert!(
        engine
            .paper_control(run, Control::Pause)
            .unwrap()
            .effective_paused
    );
    // A conflict against the open run's own Go id still stops that run.
    second.settings.quantity = 9;
    assert_eq!(
        engine.paper_start(second).unwrap_err().code,
        "input_conflict"
    );
    let stopped = engine.paper_summary(run).unwrap();
    assert_eq!(stopped.control_state, "stopped");
    assert_eq!(stopped.error_code.as_deref(), Some("input_conflict"));
}

#[test]
fn invalid_source_clock_is_rejected_before_pending_receipt_and_can_be_corrected() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::open(dir.path().join("paper.sqlite")).unwrap();
    let reply = engine.paper_start(scenario(saved(&engine))).unwrap();
    let context = reply.context.unwrap();
    let input = Input {
        context,
        ordinal: 1,
        virtual_ms: 0,
        payload: Payload::Source(src::Envelope {
            clock: src::Clock {
                at_ms: 1,
                receipt_ms: 0,
                received_through_ms: 1,
                completed_frontier_ms: 0,
            },
            provider_identity: None,
            provider_time_utc: None,
            event: src::SourceEvent::Advance,
        }),
    };
    assert_eq!(engine.paper_input(input).unwrap_err().code, "range");
    assert_eq!(
        engine
            .paper_summary(context.run_id)
            .unwrap()
            .pending_ordinal,
        None
    );
    let reply = engine
        .paper_input(Input {
            context,
            ordinal: 1,
            virtual_ms: 0,
            payload: Payload::Advance,
        })
        .unwrap();
    assert_eq!(reply.summary.completed_ordinal, "1");
    assert_eq!(reply.summary.pending_ordinal, None);
}

#[test]
fn same_process_failed_settlement_preserves_stopped_state_and_can_retry_once() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("paper.sqlite");
    let engine = Engine::open(&path).unwrap();
    let mut start = scenario(saved(&engine));
    start.settlement_manifest[0].available_ms = 1000000;
    let reply = engine.paper_start(start).unwrap();
    let mut driver = Driver {
        engine: &engine,
        context: reply.context.unwrap(),
        ordinal: 0,
        summary: reply.summary,
    };
    driver.prepare();
    driver.input(61000, Payload::Book(driver.book(10000000)));
    driver.input(901000, Payload::Advance);
    let run = driver.context.run_id;
    let before = engine.paper_summary(run).unwrap();
    assert_eq!(before.control_state, "stopped");
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch("CREATE TRIGGER fail_settlement BEFORE INSERT ON paper_settlements BEGIN SELECT RAISE(ABORT,'controlled E failure'); END").unwrap();
    assert_eq!(
        engine.paper_check_settlement(run).unwrap_err().code,
        "storage"
    );
    assert_eq!(engine.paper_summary(run).unwrap(), before);
    db.execute_batch("DROP TRIGGER fail_settlement").unwrap();
    let settled = engine.paper_check_settlement(run).unwrap().summary;
    assert_eq!(settled.cash, "10736870000");
    assert_eq!(settled.virtual_frontier_ms, "1000000");
    assert_eq!(
        settled.last_evaluated_through_ms,
        before.last_evaluated_through_ms
    );
    assert_eq!(engine.paper_check_settlement(run).unwrap().summary, settled);
}
