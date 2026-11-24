use metten_engine::{Engine, StrategyRequest, StrategyResponse};
use rusqlite::Connection;
const STRATEGY: &str = include_str!("../../../examples/learning.json");

fn version(engine: &Engine) -> String {
    match engine
        .strategies(StrategyRequest::Save {
            json: STRATEGY.into(),
            reviewed_json: STRATEGY.into(),
        })
        .unwrap()
    {
        StrategyResponse::Saved { version } => version.id,
        _ => panic!("expected saved version"),
    }
}

#[test]
fn schema_three_migration_preserves_original_rows_and_is_transactional() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("paper.sqlite");
    // Build the exact published schema-3 workspace before the paper migration.
    let db = Connection::open(&path).unwrap();
    db.execute_batch("CREATE TABLE session_events(id INTEGER PRIMARY KEY,status TEXT NOT NULL,tick INTEGER NOT NULL,reason TEXT NOT NULL,recorded_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))) STRICT;
    INSERT INTO session_events VALUES(1,'stopped',42,'preserved','old');
    CREATE TABLE strategy_versions(id INTEGER PRIMARY KEY,json TEXT NOT NULL,recorded_at TEXT NOT NULL,format_revision INTEGER,meaning_id TEXT,explanation TEXT,display_name TEXT) STRICT;
    INSERT INTO strategy_versions VALUES(1,'original','old',NULL,NULL,NULL,NULL);
    PRAGMA user_version=3;").unwrap();
    drop(db);
    let engine = Engine::open(&path).unwrap();
    assert!(
        engine
            .snapshot()
            .history
            .iter()
            .any(|r| r.reason == "preserved" && r.tick == 42)
    );
    let db = Connection::open(&path).unwrap();
    assert_eq!(
        db.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        7
    );
    assert_eq!(
        db.query_row("SELECT json FROM strategy_versions WHERE id=1", [], |r| {
            r.get::<_, String>(0)
        })
        .unwrap(),
        "original"
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM paper_runs", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| r
            .get::<_, i64>(
            0
        ))
        .unwrap(),
        0
    );
}

#[test]
fn numeric_market_rows_keep_exact_source_identity_and_enforce_run_ownership() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("paper.sqlite");
    let engine = Engine::open(&path).unwrap();
    let strategy_id = version(&engine);
    let db = Connection::open(&path).unwrap();
    db.execute_batch("PRAGMA foreign_keys=ON").unwrap();
    for id in 1..=2 {
        db.execute("INSERT INTO paper_runs(run_id,start_uuid,strategy_version_id,format_revision,meaning_id,quantity,minimum_price,maximum_price,starting_balance,fee_model,simulation_model,scenario_revision,settlement_manifest,go_utc_ms,control_state,cash) VALUES(?1,?2,?3,2,'metten_rules_2',10,0,100000000,10000000000,'fee','finite','v1','[]',1000000,'stopped',10000000000)",rusqlite::params![id, format!("00000000-0000-4000-8000-{id:012}"),strategy_id]).unwrap();
        db.execute("INSERT INTO paper_markets(run_id,source_market_id,source_contract_id,series_id,open_utc_ms,close_utc_ms,close_day,reason,summary) VALUES(?1,'KXBTC15M-PAPER-NEXT','contract-next','btc15m',1001000,1901000,0,'prepared','{}')",[id]).unwrap();
    }
    let rows:Vec<(i64,i64,String)> = db.prepare("SELECT paper_market_id,run_id,source_market_id FROM paper_markets ORDER BY paper_market_id").unwrap().query_map([],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap().map(Result::unwrap).collect();
    assert_ne!(rows[0].0, rows[1].0);
    assert_eq!(rows[0].2, "KXBTC15M-PAPER-NEXT");
    assert!(
        db.execute("UPDATE paper_runs SET quantity=9 WHERE run_id=1", [])
            .is_err()
    );
    assert!(
        db.execute("UPDATE paper_markets SET source_market_id='renamed'", [])
            .is_err()
    );
    assert!(db.execute("INSERT INTO paper_attempts(run_id,paper_market_id,proposal_ordinal,side,quantity,price,available_millionths,evidence_identity,intent_virtual_ms,reserved,state) VALUES(2,?1,1,'no',10,25000000,10000000,'{}',61000,263130000,'intent')",[rows[0].0]).is_err());
    db.execute("INSERT INTO paper_attempts(run_id,paper_market_id,proposal_ordinal,side,quantity,price,available_millionths,evidence_identity,intent_virtual_ms,reserved,dispatched_virtual_ms,state,outcome_id,filled_quantity,cost,fee) VALUES(1,?1,1,'no',10,25000000,10000000,'{}',61000,263130000,61000,'full','outcome-1',10,250000000,13130000)",[rows[0].0]).unwrap();
    assert!(db.execute("INSERT INTO paper_attempts(run_id,paper_market_id,proposal_ordinal,side,quantity,price,available_millionths,evidence_identity,intent_virtual_ms,reserved,dispatched_virtual_ms,state) VALUES(1,?1,2,'no',10,25000000,10000000,'{}',62000,263130000,62000,'dispatched')",[rows[0].0]).is_err());
    assert!(
        db.execute("UPDATE paper_attempts SET price=26000000", [])
            .is_err()
    );
    assert!(
        db.execute(
            "UPDATE paper_attempts SET state='zero',filled_quantity=0,cost=0,fee=0",
            []
        )
        .is_err()
    );
    db.execute("INSERT INTO paper_evidence(run_id,paper_market_id,evidence_key,first_virtual_ms,last_virtual_ms,first_transaction,last_transaction,first_witness,latest_witness) SELECT 1,?1,'reason-'||value,0,0,0,0,'{}','{}' FROM json_each(?2)",rusqlite::params![rows[0].0, serde_json::to_string(&(0..4096).collect::<Vec<_>>()).unwrap()]).unwrap();
    assert!(db.execute("INSERT INTO paper_evidence(run_id,paper_market_id,evidence_key,first_virtual_ms,last_virtual_ms,first_transaction,last_transaction,first_witness,latest_witness) VALUES(1,?1,'one-too-many',0,0,0,0,'{}','{}')",[rows[0].0]).is_err());
    db.execute("INSERT INTO paper_evidence(run_id,paper_market_id,evidence_key,first_virtual_ms,last_virtual_ms,first_transaction,last_transaction,first_witness,latest_witness) VALUES(1,?1,'reason-0',0,1,0,1,'{}','{}') ON CONFLICT(paper_market_id,evidence_key) DO UPDATE SET last_virtual_ms=excluded.last_virtual_ms,last_transaction=excluded.last_transaction,repetitions=repetitions+1",[rows[0].0]).unwrap();
    assert_eq!(db.query_row("SELECT repetitions FROM paper_evidence WHERE paper_market_id=?1 AND evidence_key='reason-0'",[rows[0].0],|r|r.get::<_,i64>(0)).unwrap(),2);
    // After a confirmed 4-of-10 NO fill, only one NO attempt for the 6 remaining may be open.
    db.execute("INSERT INTO paper_attempts(run_id,paper_market_id,proposal_ordinal,side,quantity,price,available_millionths,evidence_identity,intent_virtual_ms,reserved,dispatched_virtual_ms,state,outcome_id,filled_quantity,cost,fee) VALUES(2,?1,1,'no',10,25000000,4000000,'{}',61000,263130000,61000,'partial','outcome-2',4,100000000,5250000)",[rows[1].0]).unwrap();
    let retry = |ordinal: i64, side: &str, quantity: i64| {
        db.execute("INSERT INTO paper_attempts(run_id,paper_market_id,proposal_ordinal,side,quantity,price,available_millionths,evidence_identity,intent_virtual_ms,reserved,state) VALUES(2,?1,?2,?3,?4,25000000,6000000,'{}',62000,0,'intent')",rusqlite::params![rows[1].0, ordinal, side, quantity]).map_err(|e| e.to_string())
    };
    let refused = |e: String| e.contains("unfilled locked objective");
    assert!(retry(2, "yes", 6).is_err_and(refused), "side lock");
    assert!(retry(2, "no", 10).is_err_and(refused), "remainder only");
    retry(2, "no", 6).unwrap();
    assert!(
        retry(3, "no", 6).is_err_and(refused),
        "one unresolved attempt"
    );
}

#[test]
fn failed_schema_four_migration_rolls_back_and_newer_workspace_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("paper.sqlite");
    let engine = Engine::open(&path).unwrap();
    version(&engine);
    drop(engine);
    let db = Connection::open(&path).unwrap();
    db.execute_batch(
        "DROP TABLE paper_settlements; DROP TABLE paper_attempts;
        DROP TABLE paper_markets; DROP TABLE paper_runs; PRAGMA user_version=3",
    )
    .unwrap();
    drop(db);
    assert!(Engine::open(&path).is_err());
    let db = Connection::open(&path).unwrap();
    assert_eq!(
        db.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        3
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM strategy_versions", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(db.query_row("SELECT count(*) FROM sqlite_schema WHERE name IN ('paper_runs','paper_markets','paper_attempts','paper_settlements')", [], |r|r.get::<_,i64>(0)).unwrap(),0);
    db.execute_batch("PRAGMA user_version=8").unwrap();
    drop(db);
    assert!(Engine::open(&path).err().unwrap().contains("newer Metten"));
}

// The published schema-4 attempts table, before finite IOC retries.
const SCHEMA_FOUR_ATTEMPTS: &str = "
CREATE TABLE paper_attempts (
    attempt_id INTEGER PRIMARY KEY,
    run_id INTEGER NOT NULL,
    paper_market_id INTEGER NOT NULL,
    proposal_ordinal INTEGER NOT NULL CHECK(proposal_ordinal>0),
    side TEXT NOT NULL CHECK(side IN ('yes','no')),
    quantity INTEGER NOT NULL CHECK(quantity BETWEEN 1 AND 1000000),
    price INTEGER NOT NULL CHECK(price BETWEEN 0 AND 100000000),
    available_millionths INTEGER NOT NULL CHECK(available_millionths>=0),
    evidence_identity TEXT NOT NULL CHECK(length(CAST(evidence_identity AS BLOB))<=65536 AND json_valid(evidence_identity)),
    intent_virtual_ms INTEGER NOT NULL CHECK(intent_virtual_ms>=0),
    reserved INTEGER NOT NULL CHECK(reserved>=0),
    dispatched_virtual_ms INTEGER CHECK(dispatched_virtual_ms>=intent_virtual_ms),
    state TEXT NOT NULL CHECK(state IN ('intent','dispatched','full','zero','canceled','unknown')),
    outcome_id TEXT,
    filled_quantity INTEGER NOT NULL DEFAULT 0 CHECK(filled_quantity BETWEEN 0 AND quantity),
    cost INTEGER NOT NULL DEFAULT 0 CHECK(cost>=0),
    fee INTEGER NOT NULL DEFAULT 0 CHECK(fee>=0),
    recorded_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    FOREIGN KEY(run_id,paper_market_id) REFERENCES paper_markets(run_id,paper_market_id),
    UNIQUE(paper_market_id,proposal_ordinal),
    UNIQUE(run_id,paper_market_id,outcome_id),
    CHECK(state NOT IN ('full','zero') OR (dispatched_virtual_ms IS NOT NULL AND outcome_id IS NOT NULL)),
    CHECK(state!='full' OR filled_quantity=quantity),
    CHECK(state NOT IN ('zero','canceled') OR (filled_quantity=0 AND cost=0 AND fee=0))
) STRICT;
CREATE TRIGGER paper_attempt_fixed BEFORE UPDATE ON paper_attempts
WHEN NEW.run_id IS NOT OLD.run_id OR NEW.paper_market_id IS NOT OLD.paper_market_id
 OR NEW.proposal_ordinal IS NOT OLD.proposal_ordinal OR NEW.side IS NOT OLD.side
 OR NEW.quantity IS NOT OLD.quantity OR NEW.price IS NOT OLD.price
 OR NEW.available_millionths IS NOT OLD.available_millionths OR NEW.evidence_identity IS NOT OLD.evidence_identity
 OR NEW.intent_virtual_ms IS NOT OLD.intent_virtual_ms OR NEW.reserved IS NOT OLD.reserved
 OR (OLD.state!='intent' AND (NEW.state IS NOT OLD.state OR NEW.outcome_id IS NOT OLD.outcome_id
 OR NEW.dispatched_virtual_ms IS NOT OLD.dispatched_virtual_ms OR NEW.filled_quantity IS NOT OLD.filled_quantity
 OR NEW.cost IS NOT OLD.cost OR NEW.fee IS NOT OLD.fee))
BEGIN SELECT RAISE(ABORT,'Paper proposal and terminal facts are immutable'); END;
CREATE UNIQUE INDEX paper_one_dispatch ON paper_attempts(paper_market_id) WHERE dispatched_virtual_ms IS NOT NULL;
";

/// A real workspace holding one full and one zero attempt, rebuilt as schema 4.
fn schema_four_workspace(path: &std::path::Path) -> Vec<(i64, String, i64, i64, i64)> {
    let engine = Engine::open(path).unwrap();
    let strategy_id = version(&engine);
    drop(engine);
    let db = Connection::open(path).unwrap();
    db.execute_batch(
        "PRAGMA foreign_keys=ON; BEGIN;
        INSERT INTO paper_runs(run_id,start_uuid,strategy_version_id,format_revision,meaning_id,quantity,minimum_price,maximum_price,starting_balance,fee_model,simulation_model,scenario_revision,settlement_manifest,go_utc_ms,control_state,cash) VALUES(1,'00000000-0000-4000-8000-000000000001',STRATEGY_ID,2,'metten_rules_2',10,0,100000000,10000000000,'fee','controlled_best_ask_full_or_zero_v1','v1','[]',1000000,'stopped',10000000000),(2,'00000000-0000-4000-8000-000000000002',STRATEGY_ID,2,'metten_rules_2',101,0,100000000,10000000000,'fee','controlled_best_ask_full_or_zero_v1','v1','[]',1000000,'stopped',10000000000);
        INSERT INTO paper_markets(run_id,source_market_id,source_contract_id,series_id,open_utc_ms,close_utc_ms,close_day,participated,reason,summary) VALUES(1,'m1','c1','btc15m',0,900000,0,1,'confirmed_full_fill_pending_settlement','{}'),(2,'m2','c2','btc15m',0,900000,0,1,'insufficient_quantity','{}');
        DROP TABLE paper_venue_orders;
        ALTER TABLE paper_attempts RENAME TO current_attempts; DROP TRIGGER paper_attempt_fixed; DROP TRIGGER paper_attempt_objective;"
            .replace("STRATEGY_ID", &strategy_id)
            .as_str(),
    )
    .unwrap();
    db.execute_batch(SCHEMA_FOUR_ATTEMPTS).unwrap();
    db.execute_batch("INSERT INTO paper_attempts(run_id,paper_market_id,proposal_ordinal,side,quantity,price,available_millionths,evidence_identity,intent_virtual_ms,reserved,dispatched_virtual_ms,state,outcome_id,filled_quantity,cost,fee) SELECT 1,paper_market_id,1,'no',10,25000000,100000000,'{}',61000,263130000,61000,'full','paper-attempt-1',10,250000000,13130000 FROM paper_markets WHERE source_market_id='m1';
        INSERT INTO paper_attempts(run_id,paper_market_id,proposal_ordinal,side,quantity,price,available_millionths,evidence_identity,intent_virtual_ms,reserved,dispatched_virtual_ms,state,outcome_id) SELECT 2,paper_market_id,1,'no',101,25000000,100000000,'{}',61000,0,61000,'zero','paper-attempt-2' FROM paper_markets WHERE source_market_id='m2';
        UPDATE paper_runs SET cash=cash-263130000 WHERE run_id=1;
        DROP TABLE current_attempts; PRAGMA user_version=4; COMMIT;").unwrap();
    attempt_rows(&db)
}
fn attempt_rows(db: &Connection) -> Vec<(i64, String, i64, i64, i64)> {
    db.prepare(
        "SELECT attempt_id,state,filled_quantity,cost,fee FROM paper_attempts ORDER BY attempt_id",
    )
    .unwrap()
    .query_map([], |r| {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
    })
    .unwrap()
    .map(Result::unwrap)
    .collect()
}

#[test]
fn schema_four_attempts_migrate_unchanged_and_reconcile() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("paper.sqlite");
    let before = schema_four_workspace(&path);
    assert_eq!(before.len(), 2);
    let engine = Engine::open(&path).unwrap();
    // Opening validates cash, reservations and settlements against the migrated rows.
    assert_eq!(engine.paper_summary(1).unwrap().cash, "9736870000");
    assert!(engine.paper_summary(1).unwrap().pending_holdings);
    drop(engine);
    let db = Connection::open(&path).unwrap();
    assert_eq!(attempt_rows(&db), before);
    assert_eq!(
        db.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        7
    );
    assert_eq!(db.query_row("SELECT count(*) FROM sqlite_schema WHERE name IN ('paper_attempts_v4','paper_one_dispatch')", [], |r| r.get::<_, i64>(0)).unwrap(), 0);
    // The migrated table enforces the finite objective: nothing beyond the filled 10.
    assert!(db.execute("INSERT INTO paper_attempts(run_id,paper_market_id,proposal_ordinal,side,quantity,price,available_millionths,evidence_identity,intent_virtual_ms,reserved,state) SELECT 1,paper_market_id,2,'no',1,25000000,0,'{}',62000,0,'intent' FROM paper_markets WHERE source_market_id='m1'", []).is_err());
}

#[test]
fn failed_schema_five_migration_keeps_schema_four_records() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("paper.sqlite");
    let before = schema_four_workspace(&path);
    let db = Connection::open(&path).unwrap();
    db.execute_batch("CREATE TABLE paper_attempts_v4(blocker INTEGER) STRICT")
        .unwrap();
    drop(db);
    assert!(Engine::open(&path).is_err());
    let db = Connection::open(&path).unwrap();
    assert_eq!(
        db.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        4
    );
    assert_eq!(attempt_rows(&db), before);
}

#[test]
fn attempts_per_market_are_bounded_like_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("paper.sqlite");
    let strategy_id = version(&Engine::open(&path).unwrap());
    let db = Connection::open(&path).unwrap();
    db.execute_batch("PRAGMA foreign_keys=ON").unwrap();
    db.execute("INSERT INTO paper_runs(run_id,start_uuid,strategy_version_id,format_revision,meaning_id,quantity,minimum_price,maximum_price,starting_balance,fee_model,simulation_model,scenario_revision,settlement_manifest,go_utc_ms,control_state,cash) VALUES(1,'00000000-0000-4000-8000-000000000001',?1,2,'metten_rules_2',10,0,100000000,10000000000,'fee','finite','v1','[]',1000000,'stopped',10000000000)",[strategy_id]).unwrap();
    db.execute("INSERT INTO paper_markets(run_id,source_market_id,source_contract_id,series_id,open_utc_ms,close_utc_ms,close_day,reason,summary) VALUES(1,'m','c','btc15m',0,900000,0,'prepared','{}')",[]).unwrap();
    let attempt = "INSERT INTO paper_attempts(run_id,paper_market_id,proposal_ordinal,side,quantity,price,available_millionths,evidence_identity,intent_virtual_ms,reserved,dispatched_virtual_ms,state,outcome_id) SELECT 1,1,value,'no',10,25000000,0,'{}',61000,0,61000,'zero','paper-attempt-'||value FROM json_each(?1)";
    let ordinals = |range: std::ops::RangeInclusive<i64>| {
        serde_json::to_string(&range.collect::<Vec<_>>()).unwrap()
    };
    // Each zero fill is terminal, so 4096 retries are individually valid.
    db.execute(attempt, [ordinals(1..=4096)]).unwrap();
    assert!(db.execute(attempt, [ordinals(4097..=4097)]).is_err());
}

// The published schema-5 attempt triggers, before dispatch markers.
const SCHEMA_FIVE_TRIGGERS: &str = "
CREATE TRIGGER paper_attempt_fixed BEFORE UPDATE ON paper_attempts
WHEN NEW.run_id IS NOT OLD.run_id OR NEW.paper_market_id IS NOT OLD.paper_market_id
 OR NEW.proposal_ordinal IS NOT OLD.proposal_ordinal OR NEW.side IS NOT OLD.side
 OR NEW.quantity IS NOT OLD.quantity OR NEW.price IS NOT OLD.price
 OR NEW.available_millionths IS NOT OLD.available_millionths OR NEW.evidence_identity IS NOT OLD.evidence_identity
 OR NEW.intent_virtual_ms IS NOT OLD.intent_virtual_ms OR NEW.reserved IS NOT OLD.reserved
 OR (OLD.state!='intent' AND (NEW.state IS NOT OLD.state OR NEW.outcome_id IS NOT OLD.outcome_id
 OR NEW.dispatched_virtual_ms IS NOT OLD.dispatched_virtual_ms OR NEW.filled_quantity IS NOT OLD.filled_quantity
 OR NEW.cost IS NOT OLD.cost OR NEW.fee IS NOT OLD.fee))
BEGIN SELECT RAISE(ABORT,'Paper proposal and terminal facts are immutable'); END;
CREATE TRIGGER paper_attempt_objective BEFORE INSERT ON paper_attempts
WHEN EXISTS(SELECT 1 FROM paper_attempts WHERE paper_market_id=NEW.paper_market_id AND state IN ('intent','dispatched','unknown'))
 OR EXISTS(SELECT 1 FROM paper_attempts WHERE paper_market_id=NEW.paper_market_id AND state IN ('full','partial') AND side!=NEW.side)
 OR NEW.quantity+coalesce((SELECT sum(filled_quantity) FROM paper_attempts WHERE paper_market_id=NEW.paper_market_id AND state IN ('full','partial')),0)
    IS NOT (SELECT quantity FROM paper_runs WHERE run_id=NEW.run_id)
 OR (SELECT count(*) FROM paper_attempts WHERE paper_market_id=NEW.paper_market_id)>=4096
BEGIN SELECT RAISE(ABORT,'Paper attempt must retry only the unfilled locked objective'); END;
";

/// A schema-5 workspace holding a confirmed 4-of-10 partial fill and the retry's intent for the
/// other 6, recorded before dispatch markers existed: it may or may not have been sent.
fn schema_five_workspace(path: &std::path::Path) -> Vec<(i64, String, i64, i64, i64)> {
    let strategy_id = version(&Engine::open(path).unwrap());
    let db = Connection::open(path).unwrap();
    db.execute_batch(
        &format!("PRAGMA foreign_keys=ON; BEGIN;
        DROP TABLE paper_venue_orders; DROP TRIGGER paper_attempt_fixed; DROP TRIGGER paper_attempt_objective;
        ALTER TABLE paper_attempts DROP COLUMN interrupted;
        {SCHEMA_FIVE_TRIGGERS}
        INSERT INTO paper_runs(run_id,start_uuid,strategy_version_id,format_revision,meaning_id,quantity,minimum_price,maximum_price,starting_balance,fee_model,simulation_model,scenario_revision,settlement_manifest,go_utc_ms,control_state,cash,reserved) VALUES(1,'00000000-0000-4000-8000-000000000001',{strategy_id},2,'metten_rules_2',10,0,100000000,10000000000,'fee','controlled_best_ask_finite_ioc_v2','v1','[]',1000000,'unresolved',9894750000,157880000);
        INSERT INTO paper_markets(run_id,source_market_id,source_contract_id,series_id,open_utc_ms,close_utc_ms,close_day,participated,reason,summary) VALUES(1,'m1','c1','btc15m',0,900000,0,1,'confirmed_partial_fill_pending_settlement','{{}}');
        INSERT INTO paper_attempts(run_id,paper_market_id,proposal_ordinal,side,quantity,price,available_millionths,evidence_identity,intent_virtual_ms,reserved,dispatched_virtual_ms,state,outcome_id,filled_quantity,cost,fee) VALUES(1,1,1,'no',10,25000000,4000000,'{{}}',61000,263130000,61000,'partial','paper-attempt-1',4,100000000,5250000);
        INSERT INTO paper_attempts(run_id,paper_market_id,proposal_ordinal,side,quantity,price,available_millionths,evidence_identity,intent_virtual_ms,reserved,state) VALUES(1,1,2,'no',6,25000000,3500000,'{{}}',62000,157880000,'intent');
        PRAGMA user_version=5; COMMIT;"),
    )
    .unwrap();
    attempt_rows(&db)
}

#[test]
fn schema_five_unmarked_intent_stays_uncertain_and_keeps_the_confirmed_partial() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("paper.sqlite");
    let before = schema_five_workspace(&path);
    for _ in 0..2 {
        let engine = Engine::open(&path).unwrap();
        let summary = engine.paper_summary(1).unwrap();
        // No marker can prove the old intent unsent, and the venue has no record of it.
        assert_eq!(summary.control_state, "unresolved");
        assert_eq!(summary.cash, "9894750000");
        assert_eq!(summary.reserved, "157880000");
        let recovery = summary.recovery.unwrap();
        assert_eq!(recovery.state, "unknown");
        assert_eq!(recovery.interrupted.as_deref(), Some("legacy"));
        assert_eq!(recovery.confirmed_quantity, "4");
        assert_eq!(
            engine.paper_check_settlement(1).unwrap_err().code,
            "unresolved"
        );
        let market = engine
            .paper_history(Some(1), None)
            .unwrap()
            .markets
            .remove(0);
        assert_eq!(market.filled_quantity.as_deref(), Some("4"));
        assert_eq!(market.reason, "checking_purchase");
    }
    let db = Connection::open(&path).unwrap();
    assert_eq!(
        db.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        7
    );
    let mut after = before.clone();
    after[1].1 = "unknown".into();
    assert_eq!(attempt_rows(&db), after);
    // Request fields and an unknown attempt's lack of outcome stay immutable.
    assert!(
        db.execute(
            "UPDATE paper_attempts SET quantity=5 WHERE attempt_id=2",
            []
        )
        .is_err()
    );
    assert!(
        db.execute(
            "UPDATE paper_attempts SET state='canceled' WHERE attempt_id=2",
            []
        )
        .is_err()
    );
    assert!(
        db.execute(
            "UPDATE paper_attempts SET interrupted='restart' WHERE attempt_id=2",
            []
        )
        .is_err()
    );
}

#[test]
fn failed_schema_six_migration_keeps_schema_five_records() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("paper.sqlite");
    let before = schema_five_workspace(&path);
    let db = Connection::open(&path).unwrap();
    db.execute_batch("CREATE TABLE paper_venue_orders(blocker INTEGER) STRICT")
        .unwrap();
    drop(db);
    assert!(Engine::open(&path).is_err());
    let db = Connection::open(&path).unwrap();
    assert_eq!(
        db.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        5
    );
    assert_eq!(attempt_rows(&db), before);
    assert!(
        db.prepare("SELECT interrupted FROM paper_attempts")
            .is_err()
    );
}
