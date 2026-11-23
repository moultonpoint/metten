//! Uncertain purchase recovery through the public engine, real SQLite and the workspace's paper
//! venue. A probe around the venue observes sends independently of engine accounting and can
//! lose replies, hide records or exit the process around acceptance.
use metten_engine::Engine;
use metten_engine::paper::*;
use metten_engine::strategy::Decimal;
use metten_engine::{StrategyRequest, StrategyResponse};
use metten_engine::{sources as src, strategy as rules};
use rusqlite::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock, Weak, mpsc};
fn price(s: &str) -> Decimal {
    Decimal::parse(s).unwrap()
}
#[path = "support/paper.rs"]
mod fixture;
use fixture::{Driver, saved, scenario};

#[derive(Default)]
struct Plan {
    lose_replies: bool,
    hide_records: bool,
    exit_before_accept: bool,
    exit_after_accept: bool,
    contradict_replies: bool,
    malformed_replies: bool,
    /// Another writer holds the workspace write lock for 600 ms as the venue records the order.
    hold_write_lock: bool,
}
#[derive(Clone, Default)]
struct Probe {
    plan: Arc<Mutex<Plan>>,
    sends: Arc<Mutex<Vec<Order>>>,
    lookups: Arc<Mutex<usize>>,
    /// When set, each send and lookup proves another reader can use the recorder meanwhile.
    engine: Arc<OnceLock<Weak<Engine>>>,
}
struct Observed {
    inner: Box<dyn Venue>,
    probe: Probe,
    db: Connection,
}
impl Venue for Observed {
    fn submit(&mut self, order: &Order) -> Option<Fill> {
        // Nothing is sent before the attempt's durable dispatch marker.
        let state: String = self.db.query_row("SELECT state FROM paper_attempts WHERE 'metten-paper-'||run_id||'-'||paper_market_id||'-'||proposal_ordinal=?1", [&order.client_order_id], |r| r.get(0)).unwrap();
        assert_eq!(state, "dispatched");
        self.probe.sends.lock().unwrap().push(order.clone());
        self.probe.readable();
        let plan = self.probe.plan.lock().unwrap();
        if plan.exit_before_accept {
            std::process::exit(86);
        }
        let holder = plan.hold_write_lock.then(|| {
            let writer = Connection::open(self.db.path().unwrap()).unwrap();
            writer.execute_batch("BEGIN IMMEDIATE").unwrap();
            let release = std::time::Instant::now() + std::time::Duration::from_millis(600);
            std::thread::spawn(move || {
                std::thread::sleep(release.saturating_duration_since(std::time::Instant::now()));
                writer.execute_batch("ROLLBACK").unwrap();
            })
        });
        let reply = self.inner.submit(order);
        if let Some(holder) = holder {
            holder.join().unwrap();
        }
        if plan.exit_after_accept {
            std::process::exit(86);
        }
        if plan.malformed_replies {
            let charge = Charge {
                cost: 0,
                fee: 0,
                debit: 0,
            };
            return Some(Fill::Partial {
                quantity: 0,
                charge,
            });
        }
        if plan.contradict_replies {
            return Some(Fill::Zero {
                available_millionths: order.available_millionths,
            });
        }
        if plan.lose_replies { None } else { reply }
    }
    fn lookup(&mut self, id: &str) -> Option<(Order, Fill)> {
        *self.probe.lookups.lock().unwrap() += 1;
        self.probe.readable();
        if self.probe.plan.lock().unwrap().hide_records {
            return None;
        }
        self.inner.lookup(id)
    }
}
impl Probe {
    fn open(&self, path: &Path) -> Engine {
        let probe = self.clone();
        let db_path = path.to_path_buf();
        Engine::open_with_venue(path, move |inner| {
            Box::new(Observed {
                inner,
                probe,
                db: Connection::open(db_path).unwrap(),
            })
        })
        .unwrap()
    }
    fn plan(&self, change: impl FnOnce(&mut Plan)) {
        change(&mut self.plan.lock().unwrap());
    }
    /// When an engine is registered, proves another reader can use the recorder meanwhile.
    fn readable(&self) {
        if let Some(engine) = self.engine.get().and_then(Weak::upgrade) {
            let (done, read) = mpsc::channel();
            std::thread::spawn(move || done.send(engine.paper_history(None, None).is_ok()));
            assert_eq!(
                read.recv_timeout(std::time::Duration::from_secs(5)),
                Ok(true),
                "recorder lock held during venue I/O"
            );
        }
    }
    fn sends(&self) -> usize {
        self.sends.lock().unwrap().len()
    }
}

fn receipt(engine: &Engine, run: i64) -> MarketReceipt {
    engine
        .paper_history(Some(run), None)
        .unwrap()
        .markets
        .remove(0)
}
fn count(path: &Path, sql: &str) -> i64 {
    Connection::open(path)
        .unwrap()
        .query_row(sql, [], |r| r.get(0))
        .unwrap()
}
fn attempts(path: &Path) -> Vec<(String, i64, i64)> {
    Connection::open(path)
        .unwrap()
        .prepare("SELECT state,quantity,filled_quantity FROM paper_attempts ORDER BY attempt_id")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}
/// Confirms 4 of 10 NO, then loses the reply to the 6-contract retry with records hidden.
fn partial_then_uncertain_retry(driver: &mut Driver, probe: &Probe) {
    driver.prepare();
    driver.input(61000, Payload::Book(driver.book(4_000_000)));
    assert_eq!(driver.summary.cash, "9894750000");
    probe.plan(|p| {
        p.lose_replies = true;
        p.hide_records = true;
    });
    driver.observe(62000, 2, "80", "25");
    driver.input(
        62000,
        Payload::Book(driver.side_book(rules::Side::No, "25", 2, 62000, 3_500_000)),
    );
}

#[test]
fn lost_reply_is_checked_then_reconciled_without_a_second_send() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("paper.sqlite");
    let probe = Probe::default();
    probe.plan(|p| {
        p.lose_replies = true;
        p.hide_records = true;
    });
    let engine = Arc::new(probe.open(&path));
    probe.engine.set(Arc::downgrade(&engine)).unwrap();
    let mut driver = Driver::start(&engine);
    let run = driver.context.run_id;
    driver.prepare();
    driver.input(61000, Payload::Book(driver.book(10_000_000)));
    // Unknown is not zero: no debit, the whole reservation stays, the venue holds the order.
    assert_eq!(driver.summary.cash, "10000000000");
    assert_eq!(driver.summary.reserved, "263130000");
    let checking = driver.summary.recovery.clone().unwrap();
    assert_eq!(checking.state, "dispatched");
    assert_eq!(checking.interrupted.as_deref(), Some("no_outcome"));
    assert_eq!(checking.requested_quantity, "10");
    assert_eq!(checking.confirmed_quantity, "0");
    assert_eq!(count(&path, "SELECT count(*) FROM paper_venue_orders"), 1);
    let market = receipt(&engine, run);
    assert_eq!(market.attempt_state.as_deref(), Some("dispatched"));
    assert_eq!(market.reason, "checking_purchase");
    // Monitoring continues; a missing record and newer eligible evidence still send nothing.
    driver.input(61500, Payload::Advance);
    driver.observe(62000, 2, "80", "25");
    driver.input(
        62000,
        Payload::Book(driver.side_book(rules::Side::No, "25", 2, 62000, 100_000_000)),
    );
    assert_eq!(
        driver.summary.recovery.as_ref().unwrap().state,
        "dispatched"
    );
    assert_eq!(driver.summary.reserved, "263130000");
    assert_eq!(probe.sends(), 1);
    assert_eq!(attempts(&path).len(), 1);
    assert!(*probe.lookups.lock().unwrap() >= 2);
    probe.plan(|p| p.hide_records = false);
    driver.input(62500, Payload::Advance);
    assert_eq!(driver.summary.cash, "9736870000");
    assert_eq!(driver.summary.reserved, "0");
    let recovered = driver.summary.recovery.clone().unwrap();
    assert_eq!(recovered.state, "full");
    assert_eq!(recovered.filled_quantity, "10");
    assert_eq!(recovered.interrupted.as_deref(), Some("no_outcome"));
    let market = receipt(&engine, run);
    assert_eq!(market.attempt_state.as_deref(), Some("full"));
    assert_eq!(market.reason, "confirmed_full_fill_pending_settlement");
    assert_eq!(market.recovery.as_ref().unwrap().state, "full");
    // Later checks and reopen reuse the identical record: no change, no send.
    driver.input(63000, Payload::Advance);
    assert_eq!(driver.summary.cash, "9736870000");
    driver.input(
        901000,
        Payload::Settlement(scenario(1).settlement_manifest.remove(0)),
    );
    assert_eq!(driver.summary.cash, "10736870000");
    drop(engine);
    let engine = probe.open(&path);
    assert_eq!(engine.paper_summary(run).unwrap().cash, "10736870000");
    assert_eq!(probe.sends(), 1);
    assert_eq!(count(&path, "SELECT count(*) FROM paper_venue_orders"), 1);
}

#[test]
fn brief_write_lock_during_venue_record_still_confirms_the_purchase() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("paper.sqlite");
    let probe = Probe::default();
    probe.plan(|p| p.hold_write_lock = true);
    let engine = probe.open(&path);
    let mut driver = Driver::start(&engine);
    let run = driver.context.run_id;
    driver.prepare();
    driver.input(61000, Payload::Book(driver.book(10_000_000)));
    assert_eq!(attempts(&path), [("full".into(), 10, 10)]);
    assert_eq!(driver.summary.cash, "9736870000");
    assert_eq!(driver.summary.reserved, "0");
    assert_eq!(driver.summary.recovery, None);
    driver.input(
        901000,
        Payload::Settlement(scenario(1).settlement_manifest.remove(0)),
    );
    assert_eq!(driver.summary.cash, "10736870000");
    assert_eq!(engine.paper_summary(run).unwrap().control_state, "stopped");
    let mut next = scenario(saved(&engine));
    next.start_uuid = "00000000-0000-4000-8000-000000000002".into();
    assert!(engine.paper_start(next).unwrap().context.is_some());
}

#[test]
fn settlement_waits_for_an_uncertain_retry_and_pays_the_final_holding_once() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("paper.sqlite");
    let probe = Probe::default();
    let engine = probe.open(&path);
    let mut driver = Driver::start(&engine);
    let run = driver.context.run_id;
    partial_then_uncertain_retry(&mut driver, &probe);
    // The confirmed partial holding stays visible while the 6-contract retry is checked.
    assert_eq!(driver.summary.cash, "9894750000");
    assert_eq!(driver.summary.reserved, "157880000");
    let checking = driver.summary.recovery.clone().unwrap();
    assert_eq!(checking.requested_quantity, "6");
    assert_eq!(checking.confirmed_quantity, "4");
    let market = receipt(&engine, run);
    assert_eq!(market.filled_quantity.as_deref(), Some("4"));
    assert_eq!(market.attempt_state.as_deref(), Some("dispatched"));
    driver.input(
        901000,
        Payload::Settlement(scenario(1).settlement_manifest.remove(0)),
    );
    assert_eq!(driver.summary.control_state, "unresolved");
    assert_eq!(driver.summary.cash, "9894750000");
    assert!(driver.summary.pending_holdings);
    assert_eq!(count(&path, "SELECT count(*) FROM paper_settlements"), 0);
    // The explicit check retries the lookup; a miss refuses settlement and changes nothing.
    let before = engine.paper_summary(run).unwrap();
    assert_eq!(
        engine.paper_check_settlement(run).unwrap_err().code,
        "unresolved"
    );
    assert_eq!(engine.paper_summary(run).unwrap(), before);
    probe.plan(|p| p.hide_records = false);
    // 3 of the 6 filled: $0.7894 more debit; NO wins on the combined 7: $7 payout.
    let settled = engine.paper_check_settlement(run).unwrap().summary;
    assert_eq!(settled.control_state, "stopped");
    assert_eq!(settled.cash, "10515810000");
    assert_eq!(settled.reserved, "0");
    assert_eq!(
        count(
            &path,
            "SELECT held_quantity*10+count(*) FROM paper_settlements"
        ),
        71
    );
    assert_eq!(engine.paper_check_settlement(run).unwrap().summary, settled);
    assert_eq!(probe.sends(), 2);
    drop(engine);
    let engine = probe.open(&path);
    assert_eq!(engine.paper_summary(run).unwrap().cash, "10515810000");
    assert_eq!(probe.sends(), 2);
}

#[test]
fn recovery_keeps_newer_pause_and_stop_and_resume_waits_for_the_check() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("paper.sqlite");
    let probe = Probe::default();
    let engine = probe.open(&path);
    let mut driver = Driver::start(&engine);
    let run = driver.context.run_id;
    partial_then_uncertain_retry(&mut driver, &probe);
    engine.paper_control(run, Control::Pause).unwrap();
    engine.paper_control(run, Control::StopAfterMarket).unwrap();
    assert_eq!(
        engine.paper_control(run, Control::Resume).unwrap_err().code,
        "checking_purchase"
    );
    assert!(engine.paper_effective().0);
    probe.plan(|p| {
        p.lose_replies = false;
        p.hide_records = false;
    });
    driver.input(62500, Payload::Advance);
    let recovered = driver.summary.recovery.clone().unwrap();
    assert_eq!(recovered.state, "partial");
    assert_eq!(recovered.confirmed_quantity, "7");
    // Recovery establishes facts only; the user's Pause and Stop still apply.
    let (paused, stop_after, _) = engine.paper_effective();
    assert!(paused && stop_after);
    driver.observe(63000, 3, "80", "25");
    driver.input(
        63000,
        Payload::Book(driver.side_book(rules::Side::No, "25", 3, 63000, 100_000_000)),
    );
    assert_eq!(probe.sends(), 2);
    engine.paper_control(run, Control::Resume).unwrap();
    driver.observe(64000, 4, "80", "25");
    driver.input(
        64000,
        Payload::Book(driver.side_book(rules::Side::No, "25", 4, 64000, 100_000_000)),
    );
    assert_eq!(
        attempts(&path),
        vec![
            ("partial".into(), 10, 4),
            ("partial".into(), 6, 3),
            ("full".into(), 3, 3)
        ]
    );
    assert_eq!(probe.sends(), 3);
    assert!(engine.paper_effective().1);
}

/// Child process body: one purchase with the configured exit, then an abrupt exit.
#[test]
fn recovery_child() {
    let Ok(point) = std::env::var("METTEN_RECOVERY_CHILD") else {
        return;
    };
    let path = std::env::var("METTEN_RECOVERY_PATH").unwrap();
    let probe = Probe::default();
    probe.plan(|p| match point.as_str() {
        "before_accept" => p.exit_before_accept = true,
        "after_accept" => p.exit_after_accept = true,
        _ => (),
    });
    let engine = probe.open(Path::new(&path));
    let mut driver = Driver::start(&engine);
    driver.prepare();
    driver.input(61000, Payload::Book(driver.book(10_000_000)));
    std::process::exit(86);
}

#[test]
fn abrupt_exits_reopen_stopped_from_venue_evidence_without_resending() {
    for (point, accepted, recovered) in [
        ("before_accept", 0, None),
        ("after_accept", 1, Some("restart")),
        ("after_outcome", 1, None),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("paper.sqlite");
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "recovery_child",
                "--exact",
                "--nocapture",
                "--test-threads",
                "1",
            ])
            .env("METTEN_RECOVERY_CHILD", point)
            .env("METTEN_RECOVERY_PATH", &path)
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(86), "{point}");
        let venue = "SELECT count(*) FROM paper_venue_orders";
        assert_eq!(count(&path, venue), accepted, "{point}");
        for _ in 0..2 {
            let probe = Probe::default();
            let engine = probe.open(&path);
            let summary = engine.paper_summary(1).unwrap();
            let recovery = summary.recovery.clone();
            if accepted == 0 {
                // A possibly sent order with no venue record stays unknown, never zero.
                assert_eq!(summary.control_state, "unresolved", "{point}");
                assert_eq!(summary.cash, "10000000000");
                assert_eq!(summary.reserved, "263130000");
                assert_eq!(recovery.unwrap().state, "dispatched");
                assert_eq!(
                    engine.paper_check_settlement(1).unwrap_err().code,
                    "unresolved"
                );
                let mut another = scenario(saved(&engine));
                another.start_uuid = "00000000-0000-4000-8000-000000000002".into();
                assert_eq!(engine.paper_start(another).unwrap_err().code, "run_active");
            } else {
                assert_eq!(summary.control_state, "stopped", "{point}");
                assert_eq!(summary.cash, "9736870000");
                assert_eq!(summary.reserved, "0");
                assert_eq!(
                    recovery.as_ref().and_then(|r| r.interrupted.as_deref()),
                    recovered
                );
            }
            // Crash recovery never resumes purchases or source playback.
            assert_eq!(
                engine.paper_control(1, Control::Resume).unwrap_err().code,
                "stale_run"
            );
            assert_eq!(probe.sends(), 0, "{point}");
            assert_eq!(count(&path, venue), accepted);
        }
        if accepted == 1 {
            let engine = Engine::open(&path).unwrap();
            assert_eq!(
                engine.paper_check_settlement(1).unwrap().summary.cash,
                "10736870000"
            );
            drop(engine);
            assert!(Engine::open(&path).is_ok());
        }
    }
}

#[test]
fn unsent_intent_is_canceled_on_reopen_and_a_failed_intent_leaves_nothing() {
    for (trigger, attempted) in [
        ("BEFORE INSERT ON paper_attempts", 0),
        (
            "BEFORE UPDATE OF state ON paper_attempts WHEN NEW.state='dispatched'",
            1,
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("paper.sqlite");
        let probe = Probe::default();
        {
            let engine = probe.open(&path);
            let mut driver = Driver::start(&engine);
            driver.prepare();
            let db = Connection::open(&path).unwrap();
            db.execute_batch(&format!(
                "CREATE TRIGGER fail_step {trigger} BEGIN SELECT RAISE(ABORT,'controlled failure'); END"
            ))
            .unwrap();
            let input = Input {
                context: driver.context,
                ordinal: driver.ordinal + 1,
                virtual_ms: 61000,
                payload: Payload::Book(driver.book(10_000_000)),
            };
            assert_eq!(engine.paper_input(input).unwrap_err().code, "storage");
            assert_eq!(attempts(&path).len(), attempted);
            db.execute_batch("DROP TRIGGER fail_step").unwrap();
        }
        let engine = probe.open(&path);
        let summary = engine.paper_summary(1).unwrap();
        assert_eq!(summary.cash, "10000000000");
        assert_eq!(summary.reserved, "0");
        assert_eq!(summary.control_state, "stopped");
        if attempted == 1 {
            // Without its dispatch marker the intent provably never left: cancel and release.
            assert_eq!(attempts(&path), vec![("canceled".into(), 10, 0)]);
            let recovery = summary.recovery.unwrap();
            assert_eq!(recovery.state, "canceled");
            assert_eq!(recovery.interrupted.as_deref(), Some("restart"));
        }
        assert_eq!(probe.sends(), 0);
        let mut another = scenario(saved(&engine));
        another.start_uuid = "00000000-0000-4000-8000-000000000002".into();
        assert!(engine.paper_start(another).is_ok());
    }
}

#[test]
fn missing_unreadable_and_contradicting_venue_records_never_establish_an_outcome() {
    for case in [
        "plain",
        "missing",
        "unreadable",
        "charged_zero",
        "overflow",
        "result",
        "request",
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("paper.sqlite");
        let probe = Probe::default();
        {
            let engine = probe.open(&path);
            let mut driver = Driver::start(&engine);
            partial_then_uncertain_retry(&mut driver, &probe);
            // Quit checks the purchase; with no record it saves the run unresolved.
            engine.stop_and_save().unwrap();
            assert_eq!(engine.paper_summary(1).unwrap().control_state, "unresolved");
        }
        let db = Connection::open(&path).unwrap();
        db.execute_batch("DROP TRIGGER paper_venue_fixed; DROP TRIGGER paper_venue_kept; PRAGMA ignore_check_constraints=ON")
            .unwrap();
        let retry = "WHERE client_order_id LIKE '%-2'";
        match case {
            "missing" => db.execute_batch(&format!("DELETE FROM paper_venue_orders {retry}")),
            "unreadable" => db.execute_batch(&format!(
                "UPDATE paper_venue_orders SET side='maybe' {retry}"
            )),
            "charged_zero" => db.execute_batch(&format!(
                "UPDATE paper_venue_orders SET filled_quantity=0 {retry}"
            )),
            "overflow" => db.execute_batch(&format!(
                "UPDATE paper_venue_orders SET cost=9223372036854775807,fee=1 {retry}"
            )),
            "result" => db.execute_batch(&format!(
                "UPDATE paper_venue_orders SET filled_quantity=6,cost=150000000,fee=7880000 {retry}"
            )),
            "request" => db.execute_batch(&format!(
                "UPDATE paper_venue_orders SET price=24000000 {retry}"
            )),
            _ => Ok(()),
        }
        .unwrap();
        for _ in 0..2 {
            let engine = Probe::default().open(&path);
            let summary = engine.paper_summary(1).unwrap();
            let recovery = summary.recovery.clone().unwrap();
            if case == "plain" {
                assert_eq!(summary.control_state, "stopped");
                assert_eq!(recovery.state, "partial");
                assert_eq!(recovery.confirmed_quantity, "7");
                assert_eq!(summary.cash, "9815810000");
                continue;
            }
            // Established accounting stands; the retry stays reserved and purchases blocked.
            assert_eq!(summary.control_state, "unresolved", "{case}");
            assert_eq!(summary.cash, "9894750000");
            assert_eq!(summary.reserved, "157880000");
            assert_eq!(recovery.state, "dispatched");
            assert_eq!(recovery.confirmed_quantity, "4");
            assert_eq!(
                summary.error_code.as_deref(),
                ["charged_zero", "overflow", "result", "request"]
                    .contains(&case)
                    .then_some("report_conflict")
            );
            assert_eq!(
                engine.paper_check_settlement(1).unwrap_err().code,
                if summary.error_code.is_some() {
                    "report_conflict"
                } else {
                    "unresolved"
                }
            );
            // Restoring the record's matching fill must not lift the block.
            if case == "charged_zero" {
                db.execute_batch(&format!(
                    "UPDATE paper_venue_orders SET filled_quantity=3 {retry}"
                ))
                .unwrap();
            }
        }
    }
}

#[test]
fn contradictory_reply_blocks_purchases_even_after_a_matching_lookup() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("paper.sqlite");
    let probe = Probe::default();
    let run;
    {
        let engine = probe.open(&path);
        let mut driver = Driver::start(&engine);
        run = driver.context.run_id;
        driver.prepare();
        // The reply says nothing filled; the venue's own record says all 10 did.
        probe.plan(|p| p.contradict_replies = true);
        let input = Input {
            context: driver.context,
            ordinal: driver.ordinal + 1,
            virtual_ms: 61000,
            payload: Payload::Book(driver.book(10_000_000)),
        };
        assert_eq!(
            engine.paper_input(input).unwrap_err().code,
            "report_conflict"
        );
        assert_eq!(receipt(&engine, run).reason, "checking_purchase");
        // Quit finds the matching record but must not let it override the contradiction.
        engine.stop_and_save().unwrap();
    }
    for _ in 0..2 {
        let engine = probe.open(&path);
        let summary = engine.paper_summary(run).unwrap();
        assert_eq!(summary.control_state, "unresolved");
        assert_eq!(summary.error_code.as_deref(), Some("report_conflict"));
        assert_eq!(summary.cash, "10000000000");
        assert_eq!(summary.reserved, "263130000");
        assert_eq!(summary.recovery.unwrap().state, "dispatched");
        assert_eq!(
            engine.paper_check_settlement(run).unwrap_err().code,
            "report_conflict"
        );
    }
    assert_eq!(probe.sends(), 1);
}

#[test]
fn malformed_reply_stays_blocked_after_a_matching_zero_lookup() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("paper.sqlite");
    let probe = Probe::default();
    let run;
    {
        let engine = probe.open(&path);
        let mut driver = Driver::start(&engine);
        run = driver.context.run_id;
        driver.prepare();
        // Under one contract available: the venue records a zero fill, the reply claims "partial 0".
        probe.plan(|p| p.malformed_replies = true);
        let input = Input {
            context: driver.context,
            ordinal: driver.ordinal + 1,
            virtual_ms: 61000,
            payload: Payload::Book(driver.book(999_999)),
        };
        assert_eq!(
            engine.paper_input(input).unwrap_err().code,
            "report_conflict"
        );
        engine.stop_and_save().unwrap();
    }
    let engine = probe.open(&path);
    let summary = engine.paper_summary(run).unwrap();
    assert_eq!(summary.control_state, "unresolved");
    assert_eq!(summary.error_code.as_deref(), Some("report_conflict"));
    assert_ne!(summary.reserved, "0");
    assert_eq!(summary.recovery.unwrap().state, "dispatched");
}

#[test]
fn recovered_zero_fill_advances_its_receipt_revision() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("paper.sqlite");
    let probe = Probe::default();
    probe.plan(|p| {
        p.lose_replies = true;
        p.hide_records = true;
    });
    let run;
    {
        let engine = probe.open(&path);
        let mut driver = Driver::start(&engine);
        run = driver.context.run_id;
        driver.prepare();
        driver.input(61000, Payload::Book(driver.book(999_999)));
        engine.stop_and_save().unwrap();
    }
    let revision = "SELECT sum(evidence_revision) FROM paper_markets";
    let checking = count(&path, revision);
    probe.plan(|p| p.hide_records = false);
    let engine = probe.open(&path);
    let summary = engine.paper_summary(run).unwrap();
    assert_eq!(summary.recovery.unwrap().state, "zero");
    assert_eq!(summary.reserved, "0");
    // The receipt changed with no cash movement, so its own revision must advance for the journal.
    assert!(count(&path, revision) > checking);
    assert_eq!(receipt(&engine, run).attempt_state.as_deref(), Some("zero"));
}
