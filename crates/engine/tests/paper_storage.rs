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
        4
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
    db.execute_batch("PRAGMA user_version=5").unwrap();
    drop(db);
    assert!(Engine::open(&path).err().unwrap().contains("newer Metten"));
}
