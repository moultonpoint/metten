mod risk;

use crate::{Record, Status, WriterSettings};
use rusqlite::{Connection, OptionalExtension, params};
use std::{fs::File, path::Path, time::Duration};

pub(crate) struct Recorder {
    connection: Connection,
    _lock: WorkspaceLock,
    #[cfg(test)]
    pub(crate) fail_next_paper_latest: std::cell::Cell<bool>,
}

struct WorkspaceLock(File);

impl Drop for WorkspaceLock {
    fn drop(&mut self) {
        // The connection drops first. Explicit unlock also releases the lock if a child still holds a duplicate descriptor.
        let _ = self.0.unlock();
    }
}

impl Recorder {
    pub(crate) fn open(path: &Path) -> Result<(Self, WriterSettings), String> {
        let lock = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path.with_extension("lock"))
            .map_err(|e| e.to_string())?;
        lock.try_lock().map_err(|_| {
            "This workspace is already open. Close the other Metten window and reopen.".to_string()
        })?;
        let lock = WorkspaceLock(lock);
        let connection = Connection::open(path).map_err(|e| e.to_string())?;
        connection
            .busy_timeout(Duration::from_millis(250))
            .map_err(|e| e.to_string())?;
        connection
            .execute_batch(
                "PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
            PRAGMA fullfsync=ON; PRAGMA checkpoint_fullfsync=ON; PRAGMA foreign_keys=ON;",
            )
            .map_err(|e| e.to_string())?;
        let foreign_keys: i64 = connection
            .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if foreign_keys != 1 {
            return Err("Required workspace relationship checks are unavailable.".into());
        }
        let settings = WriterSettings {
            journal_mode: connection
                .query_row("PRAGMA journal_mode", [], |r| r.get(0))
                .map_err(|e| e.to_string())?,
            synchronous: connection
                .query_row("PRAGMA synchronous", [], |r| r.get(0))
                .map_err(|e| e.to_string())?,
            fullfsync: connection
                .query_row("PRAGMA fullfsync", [], |r| r.get(0))
                .map_err(|e| e.to_string())?,
            checkpoint_fullfsync: connection
                .query_row("PRAGMA checkpoint_fullfsync", [], |r| r.get(0))
                .map_err(|e| e.to_string())?,
            sqlite_version: rusqlite::version().into(),
        };
        if settings.journal_mode != "wal"
            || settings.synchronous != 2
            || settings.fullfsync != 1
            || settings.checkpoint_fullfsync != 1
        {
            return Err("Required recording settings are unavailable on this workspace. Use a local Mac disk.".into());
        }
        let version: i64 = connection
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if version > 7 {
            return Err(
                "This workspace was created by a newer Metten. Open it with that version.".into(),
            );
        }
        connection
            .execute_batch(
                "BEGIN IMMEDIATE;
            CREATE TABLE IF NOT EXISTS session_events (
                id INTEGER PRIMARY KEY,
                status TEXT NOT NULL CHECK(status IN ('stopped','running','paused')),
                tick INTEGER NOT NULL CHECK(tick >= 0),
                reason TEXT NOT NULL,
                recorded_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
            ) STRICT;
            CREATE TABLE IF NOT EXISTS strategy_versions (
                id INTEGER PRIMARY KEY,
                json TEXT NOT NULL,
                recorded_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
            ) STRICT;
            CREATE TRIGGER IF NOT EXISTS strategy_no_update BEFORE UPDATE ON strategy_versions BEGIN SELECT RAISE(ABORT,'Strategy versions are immutable'); END;
            CREATE TRIGGER IF NOT EXISTS strategy_no_delete BEFORE DELETE ON strategy_versions BEGIN SELECT RAISE(ABORT,'Strategy versions are immutable'); END;
            ",
            )
            .map_err(|e| e.to_string())?;
        if version < 3 {
            connection.execute_batch("ALTER TABLE strategy_versions ADD COLUMN format_revision INTEGER;
                ALTER TABLE strategy_versions ADD COLUMN meaning_id TEXT;
                ALTER TABLE strategy_versions ADD COLUMN explanation TEXT;
                ALTER TABLE strategy_versions ADD COLUMN display_name TEXT;
                CREATE TRIGGER strategy_complete_insert BEFORE INSERT ON strategy_versions
                WHEN NEW.format_revision IS NOT 2 OR NEW.meaning_id IS NOT 'metten_rules_2'
                  OR NEW.explanation IS NULL OR NEW.display_name IS NULL
                BEGIN SELECT RAISE(ABORT,'Complete format-2 metadata required'); END;
                PRAGMA user_version=3;").map_err(|_| "Workspace migration failed; original records are preserved. Reopen with the current Metten.".to_string())?;
        }
        if version < 4 {
            connection.execute_batch(PAPER_SCHEMA).map_err(|_| {
                "Paper workspace migration failed; original records are preserved.".to_string()
            })?;
        }
        if version < 5 {
            // Schema-4 attempt rows satisfy every schema-5 constraint, so they copy unchanged.
            let copy = if version == 4 {
                "ALTER TABLE paper_attempts RENAME TO paper_attempts_v4;
                DROP INDEX paper_one_dispatch; DROP TRIGGER paper_attempt_fixed;"
            } else {
                ""
            };
            connection
                .execute_batch(copy)
                .and_then(|()| connection.execute_batch(PAPER_ATTEMPTS))
                .and_then(|()| {
                    connection.execute_batch(if version == 4 {
                        "INSERT INTO paper_attempts SELECT * FROM paper_attempts_v4;
                        DROP TABLE paper_attempts_v4;"
                    } else {
                        ""
                    })
                })
                .map_err(|_| {
                    "Paper workspace migration failed; original records are preserved.".to_string()
                })?;
        }
        if version < 6 {
            connection.execute_batch(PAPER_RECOVERY).map_err(|_| {
                "Paper workspace migration failed; original records are preserved.".to_string()
            })?;
        }
        if version < 7 {
            connection.execute_batch(risk::SCHEMA).map_err(|_| {
                "Daily risk migration failed; original records are preserved.".to_string()
            })?;
        }
        connection
            .execute_batch("COMMIT")
            .map_err(|_| "Workspace migration not confirmed. Reopen Metten.".to_string())?;
        Ok((
            Self {
                connection,
                _lock: lock,
                #[cfg(test)]
                fail_next_paper_latest: std::cell::Cell::new(false),
            },
            settings,
        ))
    }

    pub(crate) fn save_strategy(
        &mut self,
        rules: &crate::ValidatedStrategy,
    ) -> Result<crate::StrategySummary, crate::StrategyFailure> {
        let failed = |_| crate::StrategyFailure::new("storage");
        let tx = self.connection.transaction().map_err(failed)?;
        tx.execute("INSERT INTO strategy_versions(json,format_revision,meaning_id,explanation,display_name) VALUES (?1,2,'metten_rules_2',?2,?3)", params![rules.json(), rules.explanation(), rules.name()]).map_err(failed)?;
        let id = tx.last_insert_rowid();
        let recorded_at = tx
            .query_row(
                "SELECT recorded_at FROM strategy_versions WHERE id=?1",
                [id],
                |r| r.get(0),
            )
            .map_err(failed)?;
        let summary = crate::StrategySummary {
            id: id.to_string(),
            recorded_at,
            display_name: Some(rules.name().into()),
            format_revision: Some(2),
            status: crate::VersionStatus::Supported,
        };
        tx.commit().map_err(failed)?;
        Ok(summary)
    }

    pub(crate) fn strategy_versions(
        &self,
        before: Option<i64>,
    ) -> Result<crate::StrategyResponse, crate::StrategyFailure> {
        use crate::{StrategyFailure, StrategyResponse};
        let failed = |_| StrategyFailure::new("storage");
        let mut statement = self.connection.prepare("SELECT id, CASE WHEN length(CAST(recorded_at AS BLOB)) <= 40 THEN recorded_at ELSE 'Unknown recording time' END, format_revision, CASE WHEN length(CAST(meaning_id AS BLOB)) <= 128 THEN meaning_id END, CASE WHEN length(CAST(display_name AS BLOB)) <= 480 THEN display_name END, explanation IS NOT NULL FROM strategy_versions WHERE (?1 IS NULL OR id < ?1) ORDER BY id DESC LIMIT 51").map_err(failed)?;
        let mut rows = statement.query([before]).map_err(failed)?;
        let mut versions = Vec::new();
        let mut more = false;
        while let Some(row) = rows.next().map_err(failed)? {
            let summary = version_summary(row).map_err(failed)?;
            if versions.len() == 50 {
                more = true;
                break;
            }
            versions.push(summary);
            // Reserve the real cursor in the measured envelope even on a full final page.
            let page = StrategyResponse::Versions {
                next_cursor: versions.last().map(|v| v.id.clone()),
                versions: versions.clone(),
            };
            if serde_json::to_vec(&page)
                .map_err(|_| StrategyFailure::new("storage"))?
                .len()
                > crate::STRATEGY_PAGE_BYTES
            {
                versions.pop();
                more = true;
                break;
            }
        }
        let next_cursor = if more {
            versions.last().map(|v| v.id.clone())
        } else {
            None
        };
        Ok(StrategyResponse::Versions {
            versions,
            next_cursor,
        })
    }

    pub(crate) fn strategy_detail(
        &self,
        id: i64,
    ) -> Result<crate::StrategyResponse, crate::StrategyFailure> {
        use crate::{StrategyDetail, StrategyFailure, StrategyResponse, VersionStatus};
        let failed = |_| StrategyFailure::new("storage");
        let mut statement = self.connection.prepare("SELECT id, CASE WHEN length(CAST(recorded_at AS BLOB)) <= 40 THEN recorded_at ELSE 'Unknown recording time' END, format_revision, CASE WHEN length(CAST(meaning_id AS BLOB)) <= 128 THEN meaning_id END, CASE WHEN length(CAST(display_name AS BLOB)) <= 480 THEN display_name END, explanation IS NOT NULL, CASE WHEN length(CAST(json AS BLOB)) <= 1048576 THEN json END, CASE WHEN length(CAST(explanation AS BLOB)) <= 1048576 THEN explanation END FROM strategy_versions WHERE id=?1").map_err(failed)?;
        let mut rows = statement.query([id]).map_err(failed)?;
        let row = rows
            .next()
            .map_err(failed)?
            .ok_or_else(|| StrategyFailure::new("not_found"))?;
        let mut summary = version_summary(row).map_err(failed)?;
        let json: Option<String> = row.get(6).map_err(failed)?;
        let explanation: Option<String> = row.get(7).map_err(failed)?;
        let meaning_id: Option<String> = row.get(3).map_err(failed)?;
        let mut notice = match summary.status {
            VersionStatus::Supported => "Supported rules only; no trading is connected.",
            VersionStatus::LegacyReadOnly => {
                "Legacy read-only history. An absent explanation was never recorded; it is not reconstructed."
            }
            VersionStatus::Unsupported => {
                "This format or meaning is unsupported. Inspect or edit a copy; it cannot execute."
            }
            VersionStatus::InvalidRecord => {
                "Incomplete or inconsistent version record; it cannot execute."
            }
        };
        if summary.status == VersionStatus::Supported {
            let valid = json
                .as_deref()
                .and_then(|text| crate::ValidatedStrategy::parse(text).ok())
                .is_some_and(|rules| {
                    Some(rules.explanation()) == explanation.as_deref()
                        && Some(rules.name()) == summary.display_name.as_deref()
                });
            if !valid {
                summary.status = VersionStatus::InvalidRecord;
                notice = "Inconsistent version record; it cannot execute.";
            }
        }
        let mut detail = StrategyDetail {
            summary,
            json,
            explanation,
            meaning_id,
            notice,
        };
        let too_large = detail.json.is_none()
            || (row.get::<_, bool>(5).map_err(failed)? && detail.explanation.is_none())
            || serde_json::to_vec(&StrategyResponse::Detail {
                version: detail.clone(),
            })
            .map_err(|_| StrategyFailure::new("storage"))?
            .len()
                > crate::STRATEGY_DETAIL_BYTES;
        if too_large {
            detail.json = None;
            detail.explanation = None;
            detail.notice = "This version is too large to show completely. Original content is preserved in storage.";
        }
        Ok(StrategyResponse::Detail { version: detail })
    }

    pub(crate) fn latest(&self) -> Result<Vec<Record>, String> {
        let mut statement = self.connection.prepare("SELECT id,status,tick,reason,recorded_at FROM session_events ORDER BY id DESC LIMIT 100").map_err(|e| e.to_string())?;
        statement
            .query_map([], |row| {
                let status: String = row.get(1)?;
                let status = match status.as_str() {
                    "stopped" => Status::Stopped,
                    "running" => Status::Running,
                    "paused" => Status::Paused,
                    _ => return Err(rusqlite::Error::InvalidQuery),
                };
                Ok(Record {
                    id: row.get(0)?,
                    status,
                    tick: row.get(2)?,
                    reason: row.get(3)?,
                    recorded_at: row.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub(crate) fn append(
        &mut self,
        status: Status,
        tick: i64,
        reason: &str,
    ) -> Result<Vec<Record>, String> {
        let transaction = self.connection.transaction().map_err(|e| e.to_string())?;
        transaction
            .execute(
                "INSERT INTO session_events(status,tick,reason) VALUES (?1,?2,?3)",
                params![status.as_str(), tick, reason],
            )
            .map_err(|e| e.to_string())?;
        transaction.commit().map_err(|e| e.to_string())?;
        self.latest()
    }
}

fn version_summary(row: &rusqlite::Row<'_>) -> rusqlite::Result<crate::StrategySummary> {
    use crate::VersionStatus;
    let format_revision: Option<i64> = row.get(2)?;
    let meaning: Option<String> = row.get(3)?;
    let name: Option<String> = row.get(4)?;
    let has_explanation: bool = row.get(5)?;
    let name_valid = name.as_ref().is_some_and(|s| {
        !s.trim().is_empty() && s.chars().count() <= 120 && !s.chars().any(char::is_control)
    });
    let status = match (format_revision, meaning.as_deref()) {
        (None, None) => VersionStatus::LegacyReadOnly,
        (Some(2), Some("metten_rules_2")) if name_valid && has_explanation => {
            VersionStatus::Supported
        }
        (Some(2), Some("metten_rules_2")) | (None, _) => VersionStatus::InvalidRecord,
        _ => VersionStatus::Unsupported,
    };
    let time: String = row.get(1)?;
    Ok(crate::StrategySummary {
        id: row.get::<_, i64>(0)?.to_string(),
        recorded_at: if time.len() <= 40 && !time.chars().any(char::is_control) {
            time
        } else {
            "Unknown recording time".into()
        },
        format_revision,
        display_name: if name_valid { name } else { None },
        status,
    })
}

const PAPER_SCHEMA: &str = "
CREATE TABLE paper_runs (
    run_id INTEGER PRIMARY KEY,
    start_uuid TEXT NOT NULL UNIQUE CHECK(length(start_uuid)=36),
    strategy_version_id INTEGER NOT NULL REFERENCES strategy_versions(id),
    format_revision INTEGER NOT NULL CHECK(format_revision=2),
    meaning_id TEXT NOT NULL CHECK(meaning_id='metten_rules_2'),
    quantity INTEGER NOT NULL CHECK(quantity BETWEEN 1 AND 1000000),
    minimum_price INTEGER NOT NULL CHECK(minimum_price BETWEEN 0 AND 100000000),
    maximum_price INTEGER NOT NULL CHECK(maximum_price BETWEEN minimum_price AND 100000000),
    starting_balance INTEGER NOT NULL CHECK(starting_balance BETWEEN 1 AND 100000000000000),
    fee_model TEXT NOT NULL,
    simulation_model TEXT NOT NULL,
    scenario_revision TEXT NOT NULL CHECK(length(scenario_revision) BETWEEN 1 AND 128),
    settlement_manifest TEXT NOT NULL CHECK(length(CAST(settlement_manifest AS BLOB))<=16384 AND json_valid(settlement_manifest)),
    go_utc_ms INTEGER NOT NULL CHECK(go_utc_ms>=899000),
    control_state TEXT NOT NULL CHECK(control_state IN ('collecting','waiting','active','paused','stopped','unresolved')),
    stop_after_market INTEGER NOT NULL DEFAULT 0 CHECK(stop_after_market IN (0,1)),
    context_generation INTEGER NOT NULL DEFAULT 1 CHECK(context_generation>0),
    revision INTEGER NOT NULL DEFAULT 1 CHECK(revision>0),
    cash INTEGER NOT NULL CHECK(cash>=0),
    reserved INTEGER NOT NULL DEFAULT 0 CHECK(reserved BETWEEN 0 AND cash),
    completed_ordinal INTEGER NOT NULL DEFAULT 0 CHECK(completed_ordinal>=0),
    completed_receipt TEXT CHECK(length(CAST(completed_receipt AS BLOB))<=65536),
    completed_result TEXT,
    completed_revision INTEGER CHECK(completed_revision>0),
    pending_ordinal INTEGER CHECK(pending_ordinal=completed_ordinal+1),
    pending_receipt TEXT CHECK(length(CAST(pending_receipt AS BLOB))<=65536),
    virtual_frontier_ms INTEGER NOT NULL DEFAULT 0 CHECK(virtual_frontier_ms>=0),
    last_evaluated_through_ms INTEGER NOT NULL DEFAULT 0 CHECK(last_evaluated_through_ms>=0),
    error_code TEXT,
    CHECK((pending_ordinal IS NULL)=(pending_receipt IS NULL)),
    CHECK((completed_ordinal=0)=(completed_receipt IS NULL)),
    CHECK((completed_receipt IS NULL)=(completed_revision IS NULL)),
    CHECK((completed_receipt IS NULL)=(completed_result IS NULL))
) STRICT;
CREATE TRIGGER paper_run_fixed BEFORE UPDATE ON paper_runs
WHEN NEW.start_uuid IS NOT OLD.start_uuid OR NEW.strategy_version_id IS NOT OLD.strategy_version_id
 OR NEW.format_revision IS NOT OLD.format_revision OR NEW.meaning_id IS NOT OLD.meaning_id
 OR NEW.quantity IS NOT OLD.quantity OR NEW.minimum_price IS NOT OLD.minimum_price
 OR NEW.maximum_price IS NOT OLD.maximum_price OR NEW.starting_balance IS NOT OLD.starting_balance
 OR NEW.fee_model IS NOT OLD.fee_model OR NEW.simulation_model IS NOT OLD.simulation_model
 OR NEW.scenario_revision IS NOT OLD.scenario_revision OR NEW.settlement_manifest IS NOT OLD.settlement_manifest
 OR NEW.go_utc_ms IS NOT OLD.go_utc_ms
BEGIN SELECT RAISE(ABORT,'Paper run settings are immutable'); END;
CREATE TABLE paper_markets (
    paper_market_id INTEGER PRIMARY KEY,
    run_id INTEGER NOT NULL REFERENCES paper_runs(run_id),
    source_market_id TEXT NOT NULL CHECK(length(CAST(source_market_id AS BLOB)) BETWEEN 1 AND 128),
    source_contract_id TEXT NOT NULL CHECK(length(CAST(source_contract_id AS BLOB)) BETWEEN 1 AND 128),
    series_id TEXT NOT NULL CHECK(length(CAST(series_id AS BLOB)) BETWEEN 1 AND 128),
    open_utc_ms INTEGER NOT NULL CHECK(open_utc_ms>=0),
    close_utc_ms INTEGER NOT NULL CHECK(close_utc_ms=open_utc_ms+900000),
    close_day INTEGER NOT NULL CHECK(close_day=close_utc_ms/86400000),
    participated INTEGER NOT NULL DEFAULT 0 CHECK(participated IN (0,1)),
    qualified INTEGER NOT NULL DEFAULT 0 CHECK(qualified IN (0,1)),
    reason TEXT NOT NULL,
    evidence_revision INTEGER NOT NULL DEFAULT 0 CHECK(evidence_revision>=0),
    summary TEXT NOT NULL CHECK(length(CAST(summary AS BLOB))<=65536 AND json_valid(summary)),
    UNIQUE(run_id,source_market_id), UNIQUE(run_id,paper_market_id)
) STRICT;
CREATE TRIGGER paper_market_fixed BEFORE UPDATE ON paper_markets
WHEN NEW.run_id IS NOT OLD.run_id OR NEW.source_market_id IS NOT OLD.source_market_id
 OR NEW.source_contract_id IS NOT OLD.source_contract_id OR NEW.series_id IS NOT OLD.series_id
 OR NEW.open_utc_ms IS NOT OLD.open_utc_ms OR NEW.close_utc_ms IS NOT OLD.close_utc_ms OR NEW.close_day IS NOT OLD.close_day
BEGIN SELECT RAISE(ABORT,'Paper market identity is immutable'); END;
CREATE TABLE paper_settlements (
    settlement_id INTEGER PRIMARY KEY,
    run_id INTEGER NOT NULL,
    paper_market_id INTEGER NOT NULL UNIQUE,
    outcome_id TEXT NOT NULL CHECK(length(CAST(outcome_id AS BLOB)) BETWEEN 1 AND 128),
    outcome_revision TEXT NOT NULL,
    winning_side TEXT NOT NULL CHECK(winning_side IN ('yes','no')),
    effective_virtual_ms INTEGER NOT NULL CHECK(effective_virtual_ms>=0),
    recorded_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    held_quantity INTEGER NOT NULL CHECK(held_quantity BETWEEN 1 AND 1000000),
    payout INTEGER NOT NULL CHECK(payout BETWEEN 0 AND held_quantity*100000000),
    net INTEGER NOT NULL,
    close_day INTEGER NOT NULL,
    FOREIGN KEY(run_id,paper_market_id) REFERENCES paper_markets(run_id,paper_market_id)
) STRICT;
CREATE TRIGGER paper_settlement_fixed BEFORE UPDATE ON paper_settlements
BEGIN SELECT RAISE(ABORT,'Settlement facts are immutable'); END;
CREATE TABLE paper_evidence (
    evidence_id INTEGER PRIMARY KEY,
    run_id INTEGER NOT NULL,
    paper_market_id INTEGER NOT NULL,
    evidence_key TEXT NOT NULL CHECK(length(CAST(evidence_key AS BLOB)) BETWEEN 1 AND 2048),
    first_virtual_ms INTEGER NOT NULL CHECK(first_virtual_ms>=0),
    last_virtual_ms INTEGER NOT NULL CHECK(last_virtual_ms>=first_virtual_ms),
    first_transaction INTEGER NOT NULL CHECK(first_transaction>=0),
    last_transaction INTEGER NOT NULL CHECK(last_transaction>=first_transaction),
    repetitions INTEGER NOT NULL DEFAULT 1 CHECK(repetitions>0),
    first_witness TEXT NOT NULL CHECK(length(CAST(first_witness AS BLOB))<=2048 AND json_valid(first_witness)),
    latest_witness TEXT NOT NULL CHECK(length(CAST(latest_witness AS BLOB))<=2048 AND json_valid(latest_witness)),
    first_qualified_witness TEXT CHECK(length(CAST(first_qualified_witness AS BLOB))<=2048 AND json_valid(first_qualified_witness)),
    FOREIGN KEY(run_id,paper_market_id) REFERENCES paper_markets(run_id,paper_market_id),
    UNIQUE(paper_market_id,evidence_key)
) STRICT;
CREATE TRIGGER paper_evidence_limit BEFORE INSERT ON paper_evidence
WHEN (SELECT count(*) FROM paper_evidence WHERE paper_market_id=NEW.paper_market_id)>=4096
 AND NOT EXISTS(SELECT 1 FROM paper_evidence WHERE paper_market_id=NEW.paper_market_id AND evidence_key=NEW.evidence_key)
BEGIN SELECT RAISE(ABORT,'Paper evidence resource limit'); END;
PRAGMA user_version=4;
";

/// Attempts per market, matching the per-market evidence bound; reaching it fails closed.
const PAPER_ATTEMPT_LIMIT: i64 = 4096;
// Schema 5 replaces schema 4's one-dispatch-per-market attempts with a finite IOC objective:
// attempts retry only the unfilled quantity, one at a time, on the side of any confirmed fill.
const PAPER_ATTEMPTS: &str = "
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
    state TEXT NOT NULL CHECK(state IN ('intent','dispatched','full','partial','zero','canceled','unknown')),
    outcome_id TEXT,
    filled_quantity INTEGER NOT NULL DEFAULT 0 CHECK(filled_quantity BETWEEN 0 AND quantity),
    cost INTEGER NOT NULL DEFAULT 0 CHECK(cost>=0),
    fee INTEGER NOT NULL DEFAULT 0 CHECK(fee>=0),
    recorded_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    FOREIGN KEY(run_id,paper_market_id) REFERENCES paper_markets(run_id,paper_market_id),
    UNIQUE(paper_market_id,proposal_ordinal),
    UNIQUE(run_id,paper_market_id,outcome_id),
    CHECK(state NOT IN ('full','partial','zero') OR (dispatched_virtual_ms IS NOT NULL AND outcome_id IS NOT NULL)),
    CHECK(state!='full' OR filled_quantity=quantity),
    CHECK(state!='partial' OR filled_quantity BETWEEN 1 AND quantity-1),
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
CREATE TRIGGER paper_attempt_objective BEFORE INSERT ON paper_attempts
WHEN EXISTS(SELECT 1 FROM paper_attempts WHERE paper_market_id=NEW.paper_market_id AND state IN ('intent','dispatched','unknown'))
 OR EXISTS(SELECT 1 FROM paper_attempts WHERE paper_market_id=NEW.paper_market_id AND state IN ('full','partial') AND side!=NEW.side)
 OR NEW.quantity+coalesce((SELECT sum(filled_quantity) FROM paper_attempts WHERE paper_market_id=NEW.paper_market_id AND state IN ('full','partial')),0)
    IS NOT (SELECT quantity FROM paper_runs WHERE run_id=NEW.run_id)
 OR (SELECT count(*) FROM paper_attempts WHERE paper_market_id=NEW.paper_market_id)>=4096
BEGIN SELECT RAISE(ABORT,'Paper attempt must retry only the unfilled locked objective'); END;
PRAGMA user_version=5;
";

// Schema 6 adds recovery: an intent becomes `dispatched` (its pre-send marker) before any send,
// so an unmarked intent provably never left. Schema 5 had no marker, so its unconfirmed attempts
// may have been sent and become `unknown`. Only the venue's record of that exact order makes a
// possibly sent attempt terminal. The venue table is the controlled exchange's own durable record,
// written through a separate connection, never by engine accounting transactions.
const PAPER_RECOVERY: &str = "
ALTER TABLE paper_attempts ADD COLUMN interrupted TEXT CHECK(interrupted IN ('no_outcome','restart','legacy'));
DROP TRIGGER paper_attempt_fixed; DROP TRIGGER paper_attempt_objective;
UPDATE paper_attempts SET state='unknown',interrupted='legacy' WHERE state IN ('intent','dispatched','unknown');
UPDATE paper_markets SET reason='checking_purchase',evidence_revision=evidence_revision+1 WHERE paper_market_id IN (SELECT paper_market_id FROM paper_attempts WHERE interrupted='legacy');
CREATE TRIGGER paper_attempt_fixed BEFORE UPDATE ON paper_attempts
WHEN NEW.run_id IS NOT OLD.run_id OR NEW.paper_market_id IS NOT OLD.paper_market_id
 OR NEW.proposal_ordinal IS NOT OLD.proposal_ordinal OR NEW.side IS NOT OLD.side
 OR NEW.quantity IS NOT OLD.quantity OR NEW.price IS NOT OLD.price
 OR NEW.available_millionths IS NOT OLD.available_millionths OR NEW.evidence_identity IS NOT OLD.evidence_identity
 OR NEW.intent_virtual_ms IS NOT OLD.intent_virtual_ms OR NEW.reserved IS NOT OLD.reserved
 OR (OLD.interrupted IS NOT NULL AND NEW.interrupted IS NOT OLD.interrupted)
 OR (OLD.dispatched_virtual_ms IS NOT NULL AND NEW.dispatched_virtual_ms IS NOT OLD.dispatched_virtual_ms)
 OR (NEW.state IS NOT OLD.state AND NOT ((OLD.state='intent' AND NEW.state IN ('dispatched','canceled'))
    OR (OLD.state IN ('dispatched','unknown') AND NEW.state IN ('full','partial','zero'))))
 OR (NEW.state IN ('intent','dispatched','unknown','canceled') AND (NEW.outcome_id IS NOT NULL OR NEW.filled_quantity!=0 OR NEW.cost!=0 OR NEW.fee!=0))
 OR (NEW.state IN ('intent','canceled') AND NEW.dispatched_virtual_ms IS NOT NULL)
 OR (NEW.state='dispatched' AND NEW.dispatched_virtual_ms IS NULL)
 OR (OLD.state IN ('full','partial','zero','canceled') AND (NEW.outcome_id IS NOT OLD.outcome_id
    OR NEW.filled_quantity IS NOT OLD.filled_quantity OR NEW.cost IS NOT OLD.cost OR NEW.fee IS NOT OLD.fee))
BEGIN SELECT RAISE(ABORT,'Paper proposal and terminal facts are immutable'); END;
CREATE TRIGGER paper_attempt_objective BEFORE INSERT ON paper_attempts
WHEN EXISTS(SELECT 1 FROM paper_attempts WHERE run_id=NEW.run_id AND state IN ('intent','dispatched','unknown'))
 OR EXISTS(SELECT 1 FROM paper_attempts WHERE paper_market_id=NEW.paper_market_id AND state IN ('full','partial') AND side!=NEW.side)
 OR NEW.quantity+coalesce((SELECT sum(filled_quantity) FROM paper_attempts WHERE paper_market_id=NEW.paper_market_id AND state IN ('full','partial')),0)
    IS NOT (SELECT quantity FROM paper_runs WHERE run_id=NEW.run_id)
 OR (SELECT count(*) FROM paper_attempts WHERE paper_market_id=NEW.paper_market_id)>=4096
BEGIN SELECT RAISE(ABORT,'Paper attempt must retry only the unfilled locked objective'); END;
CREATE TABLE paper_venue_orders (
    client_order_id TEXT PRIMARY KEY CHECK(length(CAST(client_order_id AS BLOB)) BETWEEN 1 AND 128),
    source_market_id TEXT NOT NULL CHECK(length(CAST(source_market_id AS BLOB)) BETWEEN 1 AND 128),
    source_contract_id TEXT NOT NULL CHECK(length(CAST(source_contract_id AS BLOB)) BETWEEN 1 AND 128),
    side TEXT NOT NULL CHECK(side IN ('yes','no')),
    quantity INTEGER NOT NULL CHECK(quantity BETWEEN 1 AND 1000000),
    price INTEGER NOT NULL CHECK(price BETWEEN 0 AND 100000000),
    available_millionths INTEGER NOT NULL CHECK(available_millionths>=0),
    filled_quantity INTEGER NOT NULL CHECK(filled_quantity BETWEEN 0 AND quantity),
    cost INTEGER NOT NULL CHECK(cost>=0),
    fee INTEGER NOT NULL CHECK(fee>=0),
    accepted_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
) STRICT;
CREATE TRIGGER paper_venue_fixed BEFORE UPDATE ON paper_venue_orders BEGIN SELECT RAISE(ABORT,'Venue records are immutable'); END;
CREATE TRIGGER paper_venue_kept BEFORE DELETE ON paper_venue_orders BEGIN SELECT RAISE(ABORT,'Venue records are immutable'); END;
PRAGMA user_version=6;
";

/// The controlled paper exchange. It owns a connection separate from the recorder, executes each
/// IOC with `paper::simulate` and durably records the result under its client order id before
/// replying, so the outcome survives a lost reply or caller.
pub(crate) struct PaperVenue {
    connection: Connection,
}
impl PaperVenue {
    pub(crate) fn open(path: &Path) -> Result<Self, String> {
        let connection = Connection::open(path).map_err(|e| e.to_string())?;
        // Waits out a brief in-process write (such as a strategy save) instead of losing the
        // order record, yet stays under the 5 s Quit deadline.
        connection
            .busy_timeout(Duration::from_secs(2))
            .and_then(|()| {
                connection.execute_batch("PRAGMA synchronous=FULL; PRAGMA fullfsync=ON;")
            })
            .map_err(|e| e.to_string())?;
        Ok(Self { connection })
    }
}
fn side_text(side: crate::strategy::Side) -> &'static str {
    if side == crate::strategy::Side::Yes {
        "yes"
    } else {
        "no"
    }
}
fn side_value(text: &str) -> Option<crate::strategy::Side> {
    match text {
        "yes" => Some(crate::strategy::Side::Yes),
        "no" => Some(crate::strategy::Side::No),
        _ => None,
    }
}
impl crate::paper::Venue for PaperVenue {
    fn submit(&mut self, order: &crate::paper::Order) -> Option<crate::paper::Fill> {
        use crate::paper::Fill;
        let fill =
            crate::paper::simulate(order.quantity, order.price, order.available_millionths).ok()?;
        let (filled, cost, fee) = match fill {
            Fill::Zero { .. } => (0, 0, 0),
            Fill::Full { quantity, charge } | Fill::Partial { quantity, charge } => {
                (quantity, charge.cost, charge.fee)
            }
        };
        self.connection.execute("INSERT INTO paper_venue_orders(client_order_id,source_market_id,source_contract_id,side,quantity,price,available_millionths,filled_quantity,cost,fee) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",params![order.client_order_id,order.source_market_id,order.source_contract_id,side_text(order.side),order.quantity as i64,order.price.millionths() as i64,i64::try_from(order.available_millionths).ok()?,filled as i64,cost,fee]).ok()?;
        Some(fill)
    }
    fn lookup(&mut self, id: &str) -> Option<(crate::paper::Order, crate::paper::Fill)> {
        use crate::paper::{Charge, Fill, Order};
        let (market,contract,side,quantity,price,available,filled,cost,fee):(String,String,String,i64,i64,i64,i64,i64,i64)=self.connection.query_row("SELECT source_market_id,source_contract_id,side,quantity,price,available_millionths,filled_quantity,cost,fee FROM paper_venue_orders WHERE client_order_id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?))).ok()?;
        let order = Order {
            client_order_id: id.into(),
            source_market_id: market,
            source_contract_id: contract,
            side: side_value(&side)?,
            quantity: u64::try_from(quantity).ok()?,
            price: crate::strategy::Decimal::parse(&crate::paper::contract_cents(price)).ok()?,
            available_millionths: u64::try_from(available).ok()?,
        };
        // The venue's stored result, not a recomputation from the request.
        let charge = Charge {
            cost,
            fee,
            // Saturates so an impossible charge fails validation as a conflict.
            debit: cost.saturating_add(fee),
        };
        let fill = match u64::try_from(filled).ok()? {
            // A charged zero contradicts itself: reported as such, so it durably blocks.
            0 if cost != 0 || fee != 0 => Fill::Partial {
                quantity: 0,
                charge,
            },
            0 => Fill::Zero {
                available_millionths: order.available_millionths,
            },
            q if q == order.quantity => Fill::Full {
                quantity: q,
                charge,
            },
            q => Fill::Partial {
                quantity: q,
                charge,
            },
        };
        Some((order, fill))
    }
}

impl Recorder {
    pub(crate) fn paper_validate_balances(&self) -> Result<(), crate::paper::Failure> {
        let invalid: bool=self.connection.query_row("SELECT EXISTS(SELECT 1 FROM paper_runs r WHERE cash != starting_balance + coalesce((SELECT funds_adjustment FROM run_risk WHERE run_id=r.run_id),0) - coalesce((SELECT sum(cost+fee) FROM paper_attempts a WHERE a.run_id=r.run_id AND a.state IN ('full','partial')),0) + coalesce((SELECT sum(payout) FROM paper_settlements s WHERE s.run_id=r.run_id),0) OR reserved != coalesce((SELECT sum(reserved) FROM paper_attempts a WHERE a.run_id=r.run_id AND a.state IN ('intent','dispatched','unknown')),0)) OR EXISTS(SELECT 1 FROM paper_settlements s LEFT JOIN (SELECT run_id,paper_market_id,min(side) AS side,max(side) AS other_side,sum(filled_quantity) AS held,sum(cost) AS cost,sum(fee) AS fee FROM paper_attempts WHERE state IN ('full','partial') GROUP BY run_id,paper_market_id) a USING(run_id,paper_market_id) WHERE a.held IS NULL OR a.side!=a.other_side OR s.held_quantity!=a.held OR s.net!=s.payout-a.cost-a.fee OR s.payout!=CASE WHEN s.winning_side=a.side THEN a.held*100000000 ELSE 0 END)",[],|r|r.get(0)).map_err(paper_storage)?;
        let foreign_key_failure = self
            .connection
            .prepare("PRAGMA foreign_key_check")
            .map_err(paper_storage)?
            .exists([])
            .map_err(paper_storage)?;
        if invalid || foreign_key_failure {
            return Err(crate::paper::Failure::new("unresolved"));
        }
        Ok(())
    }

    pub(crate) fn paper_summary(
        &self,
        run_id: i64,
    ) -> Result<crate::paper::RunSummary, crate::paper::Failure> {
        self.connection.query_row("SELECT run_id,strategy_version_id,revision,control_state,cash,reserved,virtual_frontier_ms,last_evaluated_through_ms,completed_ordinal,pending_ordinal,error_code,quantity,minimum_price,maximum_price,starting_balance,fee_model,simulation_model,scenario_revision,go_utc_ms,(SELECT coalesce(sum(evidence_revision),0) FROM paper_markets m WHERE m.run_id=paper_runs.run_id)+(SELECT count(*) FROM paper_attempts a WHERE a.run_id=paper_runs.run_id)+(SELECT count(*) FROM paper_settlements s WHERE s.run_id=paper_runs.run_id),start_uuid,EXISTS(SELECT 1 FROM paper_attempts a LEFT JOIN paper_settlements s USING(run_id,paper_market_id) WHERE a.run_id=paper_runs.run_id AND a.state IN ('full','partial') AND s.settlement_id IS NULL) FROM paper_runs WHERE run_id=?1",[run_id],|r|Ok(crate::paper::RunSummary {
            run_id:r.get::<_,i64>(0)?.to_string(),strategy_version_id:r.get::<_,i64>(1)?.to_string(),revision:r.get::<_,i64>(2)?.to_string(),control_state:r.get(3)?,cash:r.get::<_,i64>(4)?.to_string(),reserved:r.get::<_,i64>(5)?.to_string(),virtual_frontier_ms:r.get::<_,i64>(6)?.to_string(),last_evaluated_through_ms:r.get::<_,i64>(7)?.to_string(),completed_ordinal:r.get::<_,i64>(8)?.to_string(),pending_ordinal:r.get::<_,Option<i64>>(9)?.map(|n|n.to_string()),error_code:r.get(10)?,quantity:r.get::<_,i64>(11)?.to_string(),minimum_price:crate::paper::contract_cents(r.get::<_,i64>(12)?),maximum_price:crate::paper::contract_cents(r.get::<_,i64>(13)?),starting_balance:r.get::<_,i64>(14)?.to_string(),fee_model:r.get(15)?,simulation_model:r.get(16)?,scenario_revision:r.get(17)?,go_utc_ms:r.get::<_,i64>(18)?.to_string(),history_revision:r.get::<_,i64>(19)?.to_string(),start_uuid:r.get(20)?,pending_holdings:r.get(21)?,recovery:None,daily_risk:None,current_risk_block:None
        })).map_err(paper_storage).and_then(|mut summary| {
            summary.recovery = self.paper_recovery(run_id, None)?;
            let utc = summary.go_utc_ms.parse::<i64>().ok().zip(summary.virtual_frontier_ms.parse::<i64>().ok())
                .and_then(|(go, at)| go.checked_add(at))
                .ok_or_else(|| crate::paper::Failure::new("range"))?;
            if summary.control_state == "active" {
                summary.current_risk_block = self.connection.query_row(
                "SELECT json_extract(summary,'$.current_risk_block') FROM paper_markets
                 WHERE run_id=?1 AND participated=1 AND open_utc_ms<=?2 AND close_utc_ms>?2",
                params![run_id, utc], |row| row.get::<_,Option<String>>(0),
                ).optional().map_err(paper_storage)?.flatten();
            }
            summary.daily_risk = self.daily_risk(run_id, utc.div_euclid(86_400_000))?;
            Ok(summary)
        })
    }

    /// The latest interrupted or unconfirmed attempt of a run (or one market): what is being
    /// checked, or how the check ended. Independent of any selected history item.
    pub(crate) fn paper_recovery(
        &self,
        run: i64,
        market: Option<i64>,
    ) -> Result<Option<crate::paper::Recovery>, crate::paper::Failure> {
        self.connection.query_row("SELECT a.paper_market_id,m.source_market_id,a.side,a.quantity,a.state,a.filled_quantity,(SELECT coalesce(sum(x.filled_quantity),0) FROM paper_attempts x WHERE x.paper_market_id=a.paper_market_id AND x.state IN ('full','partial')),a.interrupted FROM paper_attempts a JOIN paper_markets m USING(run_id,paper_market_id) WHERE a.run_id=?1 AND (?2 IS NULL OR a.paper_market_id=?2) AND (a.interrupted IS NOT NULL OR a.state IN ('intent','dispatched','unknown')) ORDER BY a.state IN ('intent','dispatched','unknown') DESC,a.attempt_id DESC LIMIT 1",params![run,market],|r|Ok(crate::paper::Recovery {
            paper_market_id:r.get::<_,i64>(0)?.to_string(),source_market_id:r.get(1)?,side:r.get(2)?,requested_quantity:r.get::<_,i64>(3)?.to_string(),state:r.get(4)?,filled_quantity:r.get::<_,i64>(5)?.to_string(),confirmed_quantity:r.get::<_,i64>(6)?.to_string(),interrupted:r.get(7)?
        })).optional().map_err(paper_storage)
    }

    pub(crate) fn paper_start(
        &mut self,
        start: &crate::paper::Start,
        prepared_id: i64,
        live: bool,
    ) -> Result<crate::paper::RunSummary, crate::paper::Failure> {
        use crate::paper::{FEE_MODEL, Failure, SIMULATION_MODEL};
        start.validate()?;
        let manifest =
            serde_json::to_string(&start.settlement_manifest).map_err(|_| Failure::new("range"))?;
        let existing=self.connection.query_row("SELECT run_id,strategy_version_id,quantity,minimum_price,maximum_price,starting_balance,scenario_revision,settlement_manifest FROM paper_runs WHERE start_uuid=?1",[&start.start_uuid],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,i64>(1)?,r.get::<_,i64>(2)?,r.get::<_,i64>(3)?,r.get::<_,i64>(4)?,r.get::<_,i64>(5)?,r.get::<_,String>(6)?,r.get::<_,String>(7)?)));
        match existing {
            Ok((id, version, quantity, min, max, balance, scenario, saved_manifest)) => {
                if version != start.strategy_version_id
                    || quantity != start.settings.quantity as i64
                    || i128::from(min) != start.settings.minimum_price.millionths()
                    || i128::from(max) != start.settings.maximum_price.millionths()
                    || balance != start.settings.starting_balance
                    || scenario != start.scenario_revision
                    || saved_manifest != manifest
                    || !risk::matches_settings(&self.connection, id, live, start.settings.limits)?
                {
                    return Err(Failure::new("input_conflict"));
                }
                let mut statement=self.connection.prepare("SELECT source_market_id,source_contract_id,series_id,open_utc_ms-(SELECT go_utc_ms FROM paper_runs WHERE run_id=?1),close_utc_ms-(SELECT go_utc_ms FROM paper_runs WHERE run_id=?1) FROM paper_markets WHERE run_id=?1 ORDER BY paper_market_id").map_err(paper_storage)?;
                let saved = statement
                    .query_map([id], |r| {
                        Ok((
                            r.get::<_, String>(0)?,
                            r.get::<_, String>(1)?,
                            r.get::<_, String>(2)?,
                            r.get::<_, i64>(3)?,
                            r.get::<_, i64>(4)?,
                        ))
                    })
                    .map_err(paper_storage)?
                    .collect::<rusqlite::Result<Vec<_>>>()
                    .map_err(paper_storage)?;
                if saved.len() != start.markets.len()
                    || saved.iter().zip(&start.markets).any(|(saved, market)| {
                        saved.0 != market.market_id
                            || saved.1 != market.contract_id
                            || saved.2 != market.series_id
                            || saved.3 != market.open_ms
                            || saved.4 != market.close_ms
                    })
                {
                    return Err(Failure::new("input_conflict"));
                }
                return self.paper_summary(id);
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => (),
            Err(e) => return Err(paper_storage(e)),
        }
        let version = self
            .strategy_detail(start.strategy_version_id)
            .map_err(|_| Failure::new("storage"))?;
        if !matches!(
            version,
            crate::StrategyResponse::Detail {
                version: crate::StrategyDetail {
                    summary: crate::StrategySummary {
                        status: crate::VersionStatus::Supported,
                        ..
                    },
                    ..
                }
            }
        ) {
            return Err(Failure::new("unsupported_input"));
        }
        let tx = self.connection.transaction().map_err(paper_storage)?;
        let blocked: bool = tx
            .query_row(PAPER_START_BLOCKED, [live], |r| r.get(0))
            .map_err(paper_storage)?;
        if blocked {
            return Err(Failure::new("run_active"));
        }
        tx.execute("INSERT INTO paper_runs(run_id,start_uuid,strategy_version_id,format_revision,meaning_id,quantity,minimum_price,maximum_price,starting_balance,fee_model,simulation_model,scenario_revision,settlement_manifest,go_utc_ms,control_state,cash) VALUES(?12,?1,?2,2,'metten_rules_2',?3,?4,?5,?6,?7,?8,?9,?10,?11,'collecting',?6)",params![start.start_uuid,start.strategy_version_id,start.settings.quantity as i64,start.settings.minimum_price.millionths() as i64,start.settings.maximum_price.millionths() as i64,start.settings.starting_balance,FEE_MODEL,SIMULATION_MODEL,start.scenario_revision,manifest,start.go_utc_ms,prepared_id]).map_err(paper_storage)?;
        let id = tx.last_insert_rowid();
        risk::save_settings(&tx, id, live, start.settings.limits, start.go_utc_ms)?;
        for market in &start.markets {
            let open = start
                .go_utc_ms
                .checked_add(market.open_ms)
                .ok_or_else(|| Failure::new("range"))?;
            let close = start
                .go_utc_ms
                .checked_add(market.close_ms)
                .ok_or_else(|| Failure::new("range"))?;
            tx.execute("INSERT INTO paper_markets(run_id,source_market_id,source_contract_id,series_id,open_utc_ms,close_utc_ms,close_day,reason,summary) VALUES(?1,?2,?3,?4,?5,?6,?7,'prepared','{}')",params![id,market.market_id,market.contract_id,market.series_id,open,close,close/86400000]).map_err(paper_storage)?;
        }
        tx.commit().map_err(paper_storage)?;
        self.paper_summary(id)
    }

    pub(crate) fn paper_begin_input(
        &mut self,
        run: i64,
        ordinal: i64,
        receipt: &str,
        at: i64,
    ) -> Result<Option<String>, crate::paper::Failure> {
        use crate::paper::Failure;
        if ordinal <= 0 || receipt.len() > 65536 {
            return Err(Failure::new("range"));
        }
        let tx = self.connection.transaction().map_err(paper_storage)?;
        let (state,last,saved,pending,revision,frontier):(String,i64,Option<String>,Option<i64>,Option<i64>,i64)=tx.query_row("SELECT control_state,completed_ordinal,completed_receipt,pending_ordinal,completed_revision,virtual_frontier_ms FROM paper_runs WHERE run_id=?1",[run],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).map_err(paper_storage)?;
        if state == "stopped" {
            return Err(Failure::new("stopped_context"));
        }
        if state == "unresolved" {
            return Err(Failure::new("unresolved"));
        }
        if pending.is_some() {
            return Err(Failure::new("busy"));
        }
        if ordinal < last {
            return Err(Failure::new("stale_input"));
        }
        if ordinal == last && saved.as_deref() == Some(receipt) {
            return Ok(revision.map(|n| n.to_string()));
        }
        let fence = if ordinal == last {
            Some("input_conflict")
        } else if last.checked_add(1) != Some(ordinal) {
            Some("input_gap")
        } else {
            None
        };
        if let Some(code) = fence {
            tx.execute("UPDATE paper_runs SET control_state='stopped',error_code=?2,revision=revision+1 WHERE run_id=?1",params![run,code]).map_err(paper_storage)?;
            tx.commit().map_err(paper_storage)?;
            return Err(Failure::new(code));
        }
        if at < frontier {
            return Err(Failure::new("range"));
        }
        tx.execute("UPDATE paper_runs SET pending_ordinal=?2,pending_receipt=?3,revision=revision+1 WHERE run_id=?1",params![run,ordinal,receipt]).map_err(paper_storage)?;
        tx.commit().map_err(paper_storage)?;
        Ok(None)
    }

    pub(crate) fn paper_complete_input(
        &mut self,
        run: i64,
        ordinal: i64,
        receipt: &str,
        frontier: i64,
        evaluated: i64,
        state: &str,
    ) -> Result<crate::paper::RunSummary, crate::paper::Failure> {
        let tx = self.connection.transaction().map_err(paper_storage)?;
        let changed=tx.execute("UPDATE paper_runs SET completed_ordinal=pending_ordinal,completed_receipt=pending_receipt,completed_result='complete',completed_revision=revision+1,pending_ordinal=NULL,pending_receipt=NULL,virtual_frontier_ms=?4,last_evaluated_through_ms=?5,control_state=CASE WHEN ?6='stopped' AND EXISTS(SELECT 1 FROM paper_attempts WHERE run_id=?1 AND state IN ('intent','dispatched','unknown')) THEN 'unresolved' ELSE ?6 END,revision=revision+1 WHERE run_id=?1 AND pending_ordinal=?2 AND pending_receipt=?3 AND virtual_frontier_ms<=?4 AND last_evaluated_through_ms<=?5 AND ?5<=?4",params![run,ordinal,receipt,frontier,evaluated,state]).map_err(paper_storage)?;
        if changed != 1 {
            return Err(crate::paper::Failure::new("unconfirmed"));
        }
        tx.commit().map_err(paper_storage)?;
        self.paper_summary(run)
    }

    pub(crate) fn paper_control(
        &mut self,
        run: i64,
        state: &str,
        stop_after: bool,
        error: Option<&str>,
    ) -> Result<crate::paper::RunSummary, crate::paper::Failure> {
        if !["waiting", "active", "paused", "stopped", "unresolved"].contains(&state) {
            return Err(crate::paper::Failure::new("invalid_request"));
        }
        let tx = self.connection.transaction().map_err(paper_storage)?;
        tx.execute("UPDATE paper_runs SET control_state=CASE WHEN control_state='unresolved' OR ?2='unresolved' OR (?2='stopped' AND EXISTS(SELECT 1 FROM paper_attempts WHERE run_id=?1 AND state IN ('intent','dispatched','unknown'))) THEN 'unresolved' ELSE ?2 END,stop_after_market=max(stop_after_market,?3),error_code=CASE WHEN ?4='report_conflict' THEN ?4 WHEN control_state='unresolved' OR EXISTS(SELECT 1 FROM paper_attempts WHERE run_id=?1 AND state IN ('intent','dispatched','unknown')) THEN coalesce(error_code,?4) ELSE coalesce(?4,error_code) END,revision=revision+1 WHERE run_id=?1",params![run,state,stop_after,error]).map_err(paper_storage)?;
        tx.execute(
            "UPDATE paper_markets SET summary=json_remove(summary,'$.current_risk_block')
             WHERE run_id=?1 AND json_extract(summary,'$.current_risk_block') IS NOT NULL",
            [run],
        )
        .map_err(paper_storage)?;
        tx.commit().map_err(paper_storage)?;
        self.paper_summary(run)
    }

    pub(crate) fn paper_stop_all(&mut self) -> Result<(), crate::paper::Failure> {
        let active: bool = self
            .connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM paper_runs WHERE control_state!='stopped' OR pending_ordinal IS NOT NULL)",
                [],
                |r| r.get(0),
            )
            .map_err(paper_storage)?;
        if !active {
            return Ok(());
        }
        // A pending input is closed as interrupted, never replayed: its committed effects stand
        // and any purchase it left unconfirmed is reconciled from venue evidence.
        self.connection.execute("UPDATE paper_runs SET control_state=CASE WHEN EXISTS(SELECT 1 FROM paper_attempts a WHERE a.run_id=paper_runs.run_id AND a.state IN ('intent','dispatched','unknown')) THEN 'unresolved' ELSE 'stopped' END,completed_ordinal=coalesce(pending_ordinal,completed_ordinal),completed_receipt=coalesce(pending_receipt,completed_receipt),completed_result=CASE WHEN pending_ordinal IS NULL THEN completed_result ELSE 'interrupted' END,completed_revision=CASE WHEN pending_ordinal IS NULL THEN completed_revision ELSE revision+1 END,pending_ordinal=NULL,pending_receipt=NULL,revision=revision+1 WHERE control_state!='stopped' OR pending_ordinal IS NOT NULL",[]).map_err(paper_storage)?;
        Ok(())
    }
}

fn paper_storage(_: rusqlite::Error) -> crate::paper::Failure {
    crate::paper::Failure::new("storage")
}

fn settlement_conflict(
    tx: rusqlite::Transaction<'_>,
    run: i64,
) -> Result<(), crate::paper::Failure> {
    let recorded = tx
        .execute(
            "UPDATE paper_runs SET control_state='unresolved',error_code='report_conflict',
         revision=revision+1 WHERE run_id=?1 AND EXISTS(SELECT 1 FROM run_risk WHERE run_id=?1)",
            [run],
        )
        .map_err(paper_storage)?;
    tx.commit().map_err(paper_storage)?;
    Err(crate::paper::Failure::new(if recorded == 0 {
        "input_conflict"
    } else {
        "report_conflict"
    }))
}

impl Recorder {
    pub(crate) fn paper_intent(
        &mut self,
        run: i64,
        candidate: &crate::strategy::Candidate,
        book: &crate::paper::Book,
        at: i64,
    ) -> Result<Option<i64>, crate::paper::Failure> {
        use crate::paper::{Failure, charge};
        let debit = charge(candidate.remaining_quantity, book.ask)?;
        let identity=serde_json::json!({"binding_id":book.binding_id,"generation":book.source_generation.to_string(),"value_transaction":book.value_transaction.to_string(),"observation_number":book.observation_number.to_string(),"accepted_ms":book.accepted_ms.to_string(),"valid_until_ms":book.valid_until_ms.to_string(),"market":book.source_market_id,"contract":book.source_contract_id}).to_string();
        let tx = self.connection.transaction().map_err(paper_storage)?;
        let (market,cash,reserved,quantity,min,max):(i64,i64,i64,i64,i64,i64)=tx.query_row("SELECT m.paper_market_id,r.cash,r.reserved,r.quantity,r.minimum_price,r.maximum_price FROM paper_markets m JOIN paper_runs r USING(run_id) WHERE m.run_id=?1 AND m.source_market_id=?2 AND m.source_contract_id=?3 AND m.participated=1",params![run,book.source_market_id,book.source_contract_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).map_err(paper_storage)?;
        let side = if book.side == crate::strategy::Side::Yes {
            "yes"
        } else {
            "no"
        };
        // An IOC consumes its book snapshot; only a strictly newer observation of that binding may
        // fund another attempt. SourceState never lowers a binding's number, even across
        // reconnects, so re-initialization is not new data. Canonical u64 strings compare by
        // length, then text, keeping the full range.
        let (confirmed,locked,unresolved,spent,attempts):(i64,Option<String>,bool,bool,i64)=tx.query_row("SELECT coalesce(sum(filled_quantity) FILTER (WHERE state IN ('full','partial')),0),max(side) FILTER (WHERE state IN ('full','partial')),EXISTS(SELECT 1 FROM paper_attempts WHERE run_id=?4 AND state IN ('intent','dispatched','unknown')),coalesce(max(dispatched_virtual_ms IS NOT NULL AND json_extract(evidence_identity,'$.binding_id')=?2 AND (length(json_extract(evidence_identity,'$.observation_number'))>length(?3) OR (length(json_extract(evidence_identity,'$.observation_number'))=length(?3) AND json_extract(evidence_identity,'$.observation_number')>=?3))),0),count(*) FROM paper_attempts WHERE paper_market_id=?1",params![market,book.binding_id,book.observation_number.to_string(),run],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).map_err(paper_storage)?;
        if unresolved {
            return Err(Failure::new("unresolved"));
        }
        if candidate.remaining_quantity as i64 != quantity - confirmed
            || locked.as_deref().is_some_and(|locked| locked != side)
        {
            return Err(Failure::new("input_conflict"));
        }
        if spent {
            return Ok(None);
        }
        if attempts >= PAPER_ATTEMPT_LIMIT {
            return Err(Failure::new("resource_limit"));
        }
        if book.ask.millionths() < i128::from(min) || book.ask.millionths() > i128::from(max) {
            return Err(Failure::new("price"));
        }
        let available = cash
            .checked_sub(reserved)
            .ok_or_else(|| Failure::new("range"))?;
        let ceiling = crate::strategy::Decimal::parse(&crate::paper::contract_cents(max))
            .map_err(|_| Failure::new("storage"))?;
        let reservation = risk::reserve(
            &tx,
            risk::PurchaseRisk {
                run,
                at,
                market,
                ask: book.ask,
                ceiling,
                quantity: candidate.remaining_quantity,
                first_fill: confirmed == 0,
                cash_after_reservations: available,
            },
        )?
        .unwrap_or(debit.debit);
        if available < reservation {
            return Err(Failure::new("insufficient_funds"));
        }
        let proposal:i64=tx.query_row("SELECT coalesce(max(proposal_ordinal),0)+1 FROM paper_attempts WHERE paper_market_id=?1",[market],|r|r.get(0)).map_err(paper_storage)?;
        tx.execute("INSERT INTO paper_attempts(run_id,paper_market_id,proposal_ordinal,side,quantity,price,available_millionths,evidence_identity,intent_virtual_ms,reserved,state) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,'intent')",params![run,market,proposal,side,candidate.remaining_quantity as i64,book.ask.millionths() as i64,i64::try_from(book.available_millionths).map_err(|_|Failure::new("range"))?,identity,at,reservation]).map_err(paper_storage)?;
        tx.execute("UPDATE paper_markets SET summary=json_remove(summary,'$.current_risk_block')
                    WHERE paper_market_id=?1 AND json_extract(summary,'$.current_risk_block') IS NOT NULL",
                   [market]).map_err(paper_storage)?;
        let attempt = tx.last_insert_rowid();
        tx.execute(
            "UPDATE paper_runs SET reserved=reserved+?2,revision=revision+1 WHERE run_id=?1",
            params![run, reservation],
        )
        .map_err(paper_storage)?;
        tx.commit().map_err(paper_storage)?;
        Ok(Some(attempt))
    }

    /// The pre-send marker: commits before the order may leave, so an intent without it was
    /// provably never sent. Returns the exact durable request to send.
    pub(crate) fn paper_dispatch(
        &mut self,
        run: i64,
        attempt: i64,
        at: i64,
    ) -> Result<crate::paper::Order, crate::paper::Failure> {
        let tx = self.connection.transaction().map_err(paper_storage)?;
        let changed=tx.execute("UPDATE paper_attempts SET state='dispatched',dispatched_virtual_ms=?3 WHERE run_id=?1 AND attempt_id=?2 AND state='intent'",params![run,attempt,at]).map_err(paper_storage)?;
        if changed != 1 {
            return Err(crate::paper::Failure::new("unconfirmed"));
        }
        tx.execute(
            "UPDATE paper_runs SET revision=revision+1 WHERE run_id=?1",
            [run],
        )
        .map_err(paper_storage)?;
        tx.commit().map_err(paper_storage)?;
        self.paper_order(attempt)
    }

    /// One attempt's request, rebuilt from its immutable durable fields; the client order id is
    /// its scoped identity (run, market, proposal ordinal).
    pub(crate) fn paper_order(
        &self,
        attempt: i64,
    ) -> Result<crate::paper::Order, crate::paper::Failure> {
        let (run,market,proposal,source,contract,side,quantity,price,available):(i64,i64,i64,String,String,String,i64,i64,i64)=self.connection.query_row("SELECT a.run_id,a.paper_market_id,a.proposal_ordinal,m.source_market_id,m.source_contract_id,a.side,a.quantity,a.price,a.available_millionths FROM paper_attempts a JOIN paper_markets m USING(run_id,paper_market_id) WHERE a.attempt_id=?1",[attempt],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?))).map_err(paper_storage)?;
        let invalid = || crate::paper::Failure::new("storage");
        Ok(crate::paper::Order {
            client_order_id: format!("metten-paper-{run}-{market}-{proposal}"),
            source_market_id: source,
            source_contract_id: contract,
            side: side_value(&side).ok_or_else(invalid)?,
            quantity: u64::try_from(quantity).map_err(|_| invalid())?,
            price: crate::strategy::Decimal::parse(&crate::paper::contract_cents(price))
                .map_err(|_| invalid())?,
            available_millionths: u64::try_from(available).map_err(|_| invalid())?,
        })
    }

    /// Unconfirmed attempts (run, attempt, state), oldest first.
    pub(crate) fn paper_unconfirmed(
        &self,
        run: Option<i64>,
    ) -> Result<Vec<(i64, i64, String)>, crate::paper::Failure> {
        let mut statement = self.connection.prepare("SELECT run_id,attempt_id,state FROM paper_attempts WHERE state IN ('intent','dispatched','unknown') AND (?1 IS NULL OR run_id=?1) ORDER BY attempt_id").map_err(paper_storage)?;
        statement
            .query_map([run], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .map_err(paper_storage)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(paper_storage)
    }

    /// Records once why an attempt's outcome is being checked; its market reports the check.
    pub(crate) fn paper_interrupt(
        &mut self,
        run: i64,
        attempt: i64,
        cause: &str,
    ) -> Result<(), crate::paper::Failure> {
        let tx = self.connection.transaction().map_err(paper_storage)?;
        let changed=tx.execute("UPDATE paper_attempts SET interrupted=?3 WHERE run_id=?1 AND attempt_id=?2 AND interrupted IS NULL AND state IN ('intent','dispatched','unknown')",params![run,attempt,cause]).map_err(paper_storage)?;
        if changed == 1 {
            tx.execute("UPDATE paper_markets SET reason='checking_purchase',evidence_revision=evidence_revision+1 WHERE paper_market_id=(SELECT paper_market_id FROM paper_attempts WHERE attempt_id=?1)",[attempt]).map_err(paper_storage)?;
            tx.execute(
                "UPDATE paper_runs SET revision=revision+1 WHERE run_id=?1",
                [run],
            )
            .map_err(paper_storage)?;
        }
        tx.commit().map_err(paper_storage)
    }

    /// Commits a terminal outcome with its debit, reservation release and recovery history in
    /// one transaction. `None` cancels an unsent intent; a fill needs a dispatched order. An
    /// identical repeated outcome is a no-op; a different one is refused.
    pub(crate) fn paper_finish_attempt(
        &mut self,
        run: i64,
        attempt: i64,
        outcome: Option<crate::paper::Fill>,
    ) -> Result<(), crate::paper::Failure> {
        use crate::paper::{Failure, Fill};
        let tx = self.connection.transaction().map_err(paper_storage)?;
        let (state,quantity,price,reservation,available):(String,i64,i64,i64,i64)=tx.query_row("SELECT state,quantity,price,reserved,available_millionths FROM paper_attempts WHERE run_id=?1 AND attempt_id=?2",params![run,attempt],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).map_err(paper_storage)?;
        let fillable = quantity.min(available / 1_000_000);
        let (next, filled, cost, fee, debit) = match outcome {
            None => ("canceled", 0, 0, 0, 0),
            Some(Fill::Zero {
                available_millionths,
            }) => {
                if available_millionths != available as u64 || fillable != 0 {
                    return Err(Failure::new("input_conflict"));
                }
                ("zero", 0, 0, 0, 0)
            }
            Some(
                Fill::Full {
                    quantity: q,
                    charge: c,
                }
                | Fill::Partial {
                    quantity: q,
                    charge: c,
                },
            ) => {
                let ask = crate::strategy::Decimal::parse(&crate::paper::contract_cents(price))
                    .map_err(|_| Failure::new("storage"))?;
                if q == 0
                    || q as i64 != fillable
                    || matches!(outcome, Some(Fill::Full { .. })) != (fillable == quantity)
                    || c != crate::paper::charge(q, ask)?
                    || c.debit > reservation
                {
                    return Err(Failure::new("input_conflict"));
                }
                (
                    if fillable == quantity {
                        "full"
                    } else {
                        "partial"
                    },
                    q as i64,
                    c.cost,
                    c.fee,
                    c.debit,
                )
            }
        };
        if !["intent", "dispatched", "unknown"].contains(&state.as_str()) {
            let same:bool=tx.query_row("SELECT state=?3 AND filled_quantity=?4 AND cost=?5 AND fee=?6 FROM paper_attempts WHERE run_id=?1 AND attempt_id=?2",params![run,attempt,next,filled,cost,fee],|r|r.get(0)).map_err(paper_storage)?;
            return if same {
                Ok(())
            } else {
                Err(Failure::new("input_conflict"))
            };
        }
        // Only an unsent intent may be canceled; only a possibly sent order may fill.
        if (state == "intent") != outcome.is_none() {
            return Err(Failure::new("input_conflict"));
        }
        // After contradictory reports no later report may settle the order.
        let conflicted: bool = tx
            .query_row(
                "SELECT coalesce(error_code='report_conflict',0) FROM paper_runs WHERE run_id=?1",
                [run],
                |r| r.get(0),
            )
            .map_err(paper_storage)?;
        if conflicted && outcome.is_some() {
            return Err(Failure::new("input_conflict"));
        }
        tx.execute("UPDATE paper_attempts SET state=?3,filled_quantity=?4,cost=?5,fee=?6,dispatched_virtual_ms=CASE WHEN ?7 THEN coalesce(dispatched_virtual_ms,intent_virtual_ms) END,outcome_id=?8 WHERE run_id=?1 AND attempt_id=?2",params![run,attempt,next,filled,cost,fee,outcome.is_some(),if outcome.is_some(){Some(format!("paper-attempt-{attempt}"))}else{None}]).map_err(paper_storage)?;
        // A stopped run whose last uncertain purchase is now confirmed is simply stopped, and the
        // failure that left it unresolved is no longer current.
        tx.execute("UPDATE paper_runs SET cash=cash-?2,reserved=reserved-?3,control_state=CASE WHEN control_state='unresolved' AND NOT EXISTS(SELECT 1 FROM paper_attempts WHERE run_id=?1 AND state IN ('intent','dispatched','unknown')) THEN 'stopped' ELSE control_state END,error_code=CASE WHEN control_state='unresolved' AND NOT EXISTS(SELECT 1 FROM paper_attempts WHERE run_id=?1 AND state IN ('intent','dispatched','unknown')) THEN NULL ELSE error_code END,revision=revision+1 WHERE run_id=?1",params![run,debit,reservation]).map_err(paper_storage)?;
        tx.execute("UPDATE paper_markets SET evidence_revision=evidence_revision+1,reason=CASE (SELECT coalesce(sum(a.filled_quantity),0) FROM paper_attempts a WHERE a.paper_market_id=paper_markets.paper_market_id AND a.state IN ('full','partial')) WHEN 0 THEN CASE WHEN EXISTS(SELECT 1 FROM paper_attempts a WHERE a.paper_market_id=paper_markets.paper_market_id AND a.state='zero') THEN 'insufficient_quantity' ELSE 'canceled_before_dispatch' END WHEN (SELECT quantity FROM paper_runs WHERE run_id=?1) THEN 'confirmed_full_fill_pending_settlement' ELSE 'confirmed_partial_fill_pending_settlement' END WHERE run_id=?1 AND paper_market_id=(SELECT paper_market_id FROM paper_attempts WHERE attempt_id=?2)",params![run,attempt]).map_err(paper_storage)?;
        tx.commit().map_err(paper_storage)?;
        Ok(())
    }

    pub(crate) fn paper_decision(
        &mut self,
        run: i64,
        source: &str,
        decision: &crate::strategy::Decision,
    ) -> Result<(), crate::paper::Failure> {
        use crate::paper::Failure;
        let tx = self.connection.transaction().map_err(paper_storage)?;
        let market: i64 = tx
            .query_row(
                "SELECT paper_market_id FROM paper_markets WHERE run_id=?1 AND source_market_id=?2",
                params![run, source],
                |r| r.get(0),
            )
            .map_err(paper_storage)?;
        if decision.witnesses.is_empty() || decision.witnesses.len() > 21 {
            return Err(Failure::new("resource_limit"));
        }
        let evidence_error = |error: rusqlite::Error| {
            if error.to_string().contains("Paper evidence resource limit") {
                Failure::new("resource_limit")
            } else {
                paper_storage(error)
            }
        };
        // Latches are immutable facts; optional comparison witnesses may be capped.
        for (id, checkpoint) in &decision.checkpoints {
            let key = serde_json::json!(["checkpoint_latch", id]).to_string();
            let witness = serde_json::json!({
                "kind": "checkpoint_latch", "checkpoint_id": id, "side": checkpoint.side,
                "evaluated_at_ms": checkpoint.at_ms.to_string(),
                "transaction": checkpoint.transaction.to_string(),
            })
            .to_string();
            let at = i64::try_from(checkpoint.at_ms).map_err(|_| Failure::new("range"))?;
            let transaction =
                i64::try_from(checkpoint.transaction).map_err(|_| Failure::new("range"))?;
            let recorded = tx.execute("INSERT INTO paper_evidence(run_id,paper_market_id,evidence_key,first_virtual_ms,last_virtual_ms,first_transaction,last_transaction,first_witness,latest_witness) VALUES(?1,?2,?3,?4,?4,?5,?5,?6,?6) ON CONFLICT(paper_market_id,evidence_key) DO UPDATE SET repetitions=repetitions WHERE first_witness=excluded.first_witness",params![run,market,key,at,transaction,witness]).map_err(evidence_error)?;
            if recorded != 1 {
                return Err(Failure::new("input_conflict"));
            }
        }
        for witness in &decision.witnesses {
            let encoded =
                serde_json::to_string(witness).map_err(|_| Failure::new("resource_limit"))?;
            if encoded.len() > 2048 {
                return Err(Failure::new("resource_limit"));
            }
            let key = serde_json::to_string(&(
                &witness.path,
                witness.side,
                witness.kind,
                witness.status,
                witness.interval_state,
                &witness.blocked_by_checkpoint,
            ))
            .map_err(|_| Failure::new("resource_limit"))?;
            let at = witness
                .evaluated_at_ms
                .parse::<i64>()
                .map_err(|_| Failure::new("range"))?;
            let transaction = witness
                .transaction
                .parse::<i64>()
                .map_err(|_| Failure::new("range"))?;
            let qualified = decision.leaves.iter().any(|leaf| {
                leaf.qualified_side.is_some() && witness.option_id.as_ref() == Some(&leaf.option_id)
            });
            tx.execute("INSERT INTO paper_evidence(run_id,paper_market_id,evidence_key,first_virtual_ms,last_virtual_ms,first_transaction,last_transaction,first_witness,latest_witness,first_qualified_witness) VALUES(?1,?2,?3,?4,?4,?5,?5,?6,?6,CASE WHEN ?7 THEN ?6 END) ON CONFLICT(paper_market_id,evidence_key) DO UPDATE SET last_virtual_ms=max(last_virtual_ms,excluded.last_virtual_ms),last_transaction=max(last_transaction,excluded.last_transaction),repetitions=repetitions+1,latest_witness=excluded.latest_witness,first_qualified_witness=coalesce(first_qualified_witness,excluded.first_qualified_witness)",params![run,market,key,at,transaction,encoded,qualified]).map_err(evidence_error)?;
        }
        let primary = &decision.witnesses[0];
        let leaf = decision
            .leaves
            .iter()
            .find(|leaf| primary.option_id.as_ref() == Some(&leaf.option_id));
        let (quantity,minimum,maximum,cash,confirmed,configured):(i64,i64,i64,i64,i64,bool)=tx.query_row(
            "SELECT quantity-coalesce(a.filled,0),minimum_price,maximum_price,cash-reserved,
             coalesce(a.filled,0),EXISTS(SELECT 1 FROM run_risk WHERE run_id=?1)
             FROM paper_runs LEFT JOIN (SELECT sum(filled_quantity) AS filled FROM paper_attempts
                 WHERE paper_market_id=?2 AND state IN ('full','partial')) a ON true WHERE run_id=?1",
            params![run,market],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?)),
        ).map_err(paper_storage)?;
        use crate::{paper, strategy::Decimal};
        let offered = leaf.and_then(|leaf| leaf.purchase_price);
        let required_debit = offered
            .filter(|_| quantity > 0)
            .map(|price| {
                if !configured {
                    return Ok(paper::charge(quantity as u64, price)?.debit);
                }
                let bound = if confirmed == 0 {
                    Decimal::parse(&paper::contract_cents(maximum))
                        .map_err(|_| Failure::new("storage"))?
                } else {
                    price
                };
                paper::objective_bound(quantity as u64, bound)
            })
            .transpose()?;
        let check=serde_json::json!({"evaluated_at_ms":decision.evaluated_through_ms.to_string(),"offered_price":offered.map(|price|price.to_string()),"minimum_price":paper::contract_cents(minimum),"maximum_price":paper::contract_cents(maximum),"available_funds":cash.to_string(),"required_debit":required_debit.map(|debit|debit.to_string())}).to_string();
        let witness = serde_json::to_string(primary).map_err(|_| Failure::new("resource_limit"))?;
        tx.execute("UPDATE paper_markets SET summary=json_set(summary,'$.primary_witness',json(?3),'$.purchase_check',json(?4),'$.more_evidence',json(?5)),evidence_revision=evidence_revision+1 WHERE run_id=?1 AND paper_market_id=?2",params![run,market,witness,check,if decision.more_evidence {"true"}else{"false"}]).map_err(paper_storage)?;
        tx.commit().map_err(paper_storage)
    }

    pub(crate) fn paper_market_progress(
        &mut self,
        run: i64,
        source: &str,
        reason: &str,
        qualified: bool,
    ) -> Result<(), crate::paper::Failure> {
        self.connection.execute(
            "UPDATE paper_markets SET summary=CASE
                 WHEN ?4 AND ?3 IN ('insufficient_funds','daily_budget','funds_unavailable','price',
                                    'evidence_mismatch','objective_blocked','unavailable_evidence','purchases_paused')
                      AND json_extract(prior.doc,'$.first_block') IS NULL
                 THEN json_set(prior.doc,'$.first_block',json_object('reason',?3,
                          'primary_witness',json_extract(prior.doc,'$.primary_witness'),
                          'purchase_check',json_extract(prior.doc,'$.purchase_check')))
                 ELSE prior.doc END,
             evidence_revision=evidence_revision+CASE WHEN participated=0 OR qualified<?4 THEN 1 ELSE 0 END,
             participated=1,qualified=max(qualified,?4),
             reason=CASE WHEN EXISTS(SELECT 1 FROM paper_attempts a
                       WHERE a.paper_market_id=paper_markets.paper_market_id
                       AND a.state IN ('full','partial','zero','dispatched','unknown')) THEN reason ELSE ?3 END
             FROM (SELECT CASE WHEN NOT ?4 OR ?3 IN ('price','evidence_mismatch','objective_blocked',
                                                         'unavailable_evidence','purchases_paused')
                               THEN json_remove(summary,'$.current_risk_block') ELSE summary END AS doc
                   FROM paper_markets WHERE run_id=?1 AND source_market_id=?2) AS prior
             WHERE run_id=?1 AND source_market_id=?2",
            params![run,source,reason,qualified],
        ).map_err(paper_storage)?;
        Ok(())
    }

    pub(crate) fn paper_finalize_markets(&mut self, run: i64) -> Result<(), crate::paper::Failure> {
        self.connection.execute("UPDATE paper_markets SET reason=json_extract(summary,'$.first_block.reason'),summary=json_set(summary,'$.primary_witness',json_extract(summary,'$.first_block.primary_witness'),'$.purchase_check',json_extract(summary,'$.first_block.purchase_check')),evidence_revision=evidence_revision+1 WHERE run_id=?1 AND participated=1 AND json_extract(summary,'$.first_block') IS NOT NULL AND NOT EXISTS(SELECT 1 FROM paper_attempts a WHERE a.paper_market_id=paper_markets.paper_market_id)",[run]).map_err(paper_storage)?;
        self.connection.execute("UPDATE paper_markets SET reason=coalesce((SELECT CASE WHEN json_extract(e.first_witness,'$.status')='unavailable' THEN 'unavailable' ELSE 'condition_false' END FROM paper_evidence e WHERE e.paper_market_id=paper_markets.paper_market_id AND json_extract(e.first_witness,'$.option_id') IS NOT NULL AND json_extract(e.first_witness,'$.interval_state')='within' AND json_extract(e.first_witness,'$.root_status') IN ('false','unavailable') ORDER BY CASE json_extract(e.first_witness,'$.root_status') WHEN 'unavailable' THEN 0 ELSE 1 END,e.first_virtual_ms,e.first_transaction,e.evidence_id LIMIT 1),reason),summary=json_set(summary,'$.primary_witness',coalesce((SELECT json(e.first_witness) FROM paper_evidence e WHERE e.paper_market_id=paper_markets.paper_market_id AND json_extract(e.first_witness,'$.option_id') IS NOT NULL AND json_extract(e.first_witness,'$.interval_state')='within' AND json_extract(e.first_witness,'$.root_status') IN ('false','unavailable') ORDER BY CASE json_extract(e.first_witness,'$.root_status') WHEN 'unavailable' THEN 0 ELSE 1 END,e.first_virtual_ms,e.first_transaction,e.evidence_id LIMIT 1),json_extract(summary,'$.primary_witness'))),evidence_revision=evidence_revision+1 WHERE run_id=?1 AND participated=1 AND qualified=0 AND NOT EXISTS(SELECT 1 FROM paper_attempts a WHERE a.paper_market_id=paper_markets.paper_market_id)",[run]).map_err(paper_storage)?;
        Ok(())
    }

    pub(crate) fn paper_settle(
        &mut self,
        run: i64,
        entry: &crate::paper::SettlementManifestEntry,
        at: i64,
    ) -> Result<(), crate::paper::Failure> {
        use crate::paper::{Failure, payout};
        if at < 0 || entry.available_ms < entry.close_ms {
            return Err(Failure::new("range"));
        }
        let tx = self.connection.transaction().map_err(paper_storage)?;
        let received: Option<i64> = tx
            .query_row(
                "SELECT json_extract(summary,'$.deferred_final_ms') FROM paper_markets
             WHERE run_id=?1 AND source_market_id=?2",
                params![run, entry.source_market_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(paper_storage)?
            .flatten();
        let at = at.max(received.unwrap_or(0));
        if (at as u64) < entry.available_ms {
            return Err(Failure::new("range"));
        }
        let manifest: String = tx
            .query_row(
                "SELECT settlement_manifest FROM paper_runs WHERE run_id=?1",
                [run],
                |r| r.get(0),
            )
            .map_err(paper_storage)?;
        let manifest: Vec<crate::paper::SettlementManifestEntry> =
            serde_json::from_str(&manifest).map_err(|_| Failure::new("storage"))?;
        if !manifest.contains(entry) {
            return settlement_conflict(tx, run);
        }
        let existing:Option<(String,String)>=tx.query_row("SELECT s.outcome_id,s.winning_side FROM paper_settlements s JOIN paper_markets m USING(run_id,paper_market_id) WHERE s.run_id=?1 AND m.source_market_id=?2",params![run,entry.source_market_id],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(paper_storage)?;
        let winner = if entry.winning_side == crate::strategy::Side::Yes {
            "yes"
        } else {
            "no"
        };
        if let Some((id, side)) = existing {
            return if id == entry.outcome_id && side == winner {
                Ok(())
            } else {
                settlement_conflict(tx, run)
            };
        }
        // The final holding is unknown while a purchase is checked; settle it once it is known.
        let checking:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM paper_attempts a JOIN paper_markets m USING(run_id,paper_market_id) WHERE m.run_id=?1 AND m.source_market_id=?2 AND a.state IN ('intent','dispatched','unknown'))",params![run,entry.source_market_id],|r|r.get(0)).map_err(paper_storage)?;
        if checking {
            tx.execute(
                "UPDATE paper_markets SET summary=json_set(summary,'$.deferred_final_ms',coalesce(json_extract(summary,'$.deferred_final_ms'),?3))
                 WHERE run_id=?1 AND source_market_id=?2",
                params![run, entry.source_market_id, at],
            )
            .map_err(paper_storage)?;
            return tx.commit().map_err(paper_storage);
        }
        tx.execute(
            "UPDATE paper_markets SET summary=json_remove(summary,'$.deferred_final_ms')
             WHERE run_id=?1 AND source_market_id=?2 AND json_extract(summary,'$.deferred_final_ms') IS NOT NULL",
            params![run,entry.source_market_id],
        ).map_err(paper_storage)?;
        let holding:Option<(i64,String,String,i64,i64,i64,i64)>=tx.query_row("SELECT m.paper_market_id,min(a.side),max(a.side),sum(a.filled_quantity),sum(a.cost),sum(a.fee),m.close_day FROM paper_markets m JOIN paper_attempts a USING(run_id,paper_market_id) JOIN paper_runs r USING(run_id) WHERE m.run_id=?1 AND m.source_market_id=?2 AND m.source_contract_id=?3 AND m.close_utc_ms-r.go_utc_ms=?4 AND a.state IN ('full','partial') GROUP BY m.paper_market_id",params![run,entry.source_market_id,entry.source_contract_id,i64::try_from(entry.close_ms).map_err(|_|Failure::new("range"))?],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?))).optional().map_err(paper_storage)?;
        let Some((market, side, other_side, quantity, cost, fee, day)) = holding else {
            return tx.commit().map_err(paper_storage);
        };
        if side != other_side {
            return Err(Failure::new("storage"));
        }
        let payment = payout(
            u64::try_from(quantity).map_err(|_| Failure::new("range"))?,
            side == winner,
        )?;
        let net = payment
            .checked_sub(cost)
            .and_then(|n| n.checked_sub(fee))
            .ok_or_else(|| Failure::new("range"))?;
        tx.execute("INSERT INTO paper_settlements(run_id,paper_market_id,outcome_id,outcome_revision,winning_side,effective_virtual_ms,held_quantity,payout,net,close_day) VALUES(?1,?2,?3,'synthetic_final_v1',?4,?5,?6,?7,?8,?9)",params![run,market,entry.outcome_id,winner,at,quantity,payment,net,day]).map_err(paper_storage)?;
        tx.execute("UPDATE paper_runs SET cash=cash+?2,virtual_frontier_ms=max(virtual_frontier_ms,?3),revision=revision+1 WHERE run_id=?1",params![run,payment,at]).map_err(paper_storage)?;
        risk::settled(&tx, run, day, at)?;
        tx.execute(
            "UPDATE paper_markets SET reason='settled' WHERE run_id=?1 AND paper_market_id=?2",
            params![run, market],
        )
        .map_err(paper_storage)?;
        tx.commit().map_err(paper_storage)?;
        Ok(())
    }

    pub(crate) fn paper_apply_deferred(
        &mut self,
        run: Option<i64>,
        at: i64,
    ) -> Result<(), crate::paper::Failure> {
        use crate::paper::{Failure, SettlementManifestEntry};
        let entries = {
            let mut statement = self.connection.prepare(
                "SELECT r.run_id,m.source_market_id,r.settlement_manifest,r.virtual_frontier_ms
                 FROM paper_markets m JOIN paper_runs r USING(run_id)
                 WHERE (?1 IS NULL OR r.run_id=?1) AND json_extract(m.summary,'$.deferred_final_ms') IS NOT NULL
                 AND NOT EXISTS(SELECT 1 FROM paper_attempts a WHERE a.paper_market_id=m.paper_market_id
                                AND a.state IN ('intent','dispatched','unknown'))",
            ).map_err(paper_storage)?;
            statement
                .query_map([run], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                })
                .map_err(paper_storage)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(paper_storage)?
        };
        for (run, market, manifest, frontier) in entries {
            let manifest = serde_json::from_str::<Vec<SettlementManifestEntry>>(&manifest)
                .map_err(|_| Failure::new("storage"))?;
            let entry = manifest
                .iter()
                .find(|entry| entry.source_market_id == market)
                .ok_or_else(|| Failure::new("storage"))?;
            self.paper_settle(run, entry, at.max(frontier))?;
        }
        Ok(())
    }

    /// Deduplicated confirmed fills for one market's objective: (quantity, locked side,
    /// any attempt finished, an attempt anywhere in the run unresolved).
    pub(crate) fn paper_objective(
        &self,
        run: i64,
        source: &str,
    ) -> Result<(u64, Option<crate::strategy::Side>, bool, bool), crate::paper::Failure> {
        let (confirmed,low,high,attempted,unresolved):(i64,Option<String>,Option<String>,bool,bool)=self.connection.query_row("SELECT coalesce(sum(a.filled_quantity) FILTER (WHERE a.state IN ('full','partial')),0),min(a.side) FILTER (WHERE a.state IN ('full','partial')),max(a.side) FILTER (WHERE a.state IN ('full','partial')),count(a.attempt_id)>0,EXISTS(SELECT 1 FROM paper_attempts x WHERE x.run_id=?1 AND x.state IN ('intent','dispatched','unknown')) FROM paper_markets m LEFT JOIN paper_attempts a USING(run_id,paper_market_id) WHERE m.run_id=?1 AND m.source_market_id=?2",params![run,source],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).map_err(paper_storage)?;
        if low != high {
            return Err(paper_storage(rusqlite::Error::InvalidQuery));
        }
        let side = match low.as_deref() {
            None => None,
            Some("yes") => Some(crate::strategy::Side::Yes),
            Some(_) => Some(crate::strategy::Side::No),
        };
        Ok((
            u64::try_from(confirmed).map_err(|_| crate::paper::Failure::new("storage"))?,
            side,
            attempted,
            unresolved,
        ))
    }

    pub(crate) fn paper_has_unconfirmed_purchase(
        &self,
        run: i64,
    ) -> Result<bool, crate::paper::Failure> {
        self.connection.query_row("SELECT EXISTS(SELECT 1 FROM paper_attempts WHERE run_id=?1 AND state IN ('intent','dispatched','unknown'))", [run], |row| row.get(0)).map_err(paper_storage)
    }

    pub(crate) fn paper_check_settlement(
        &mut self,
        run: i64,
    ) -> Result<crate::paper::RunSummary, crate::paper::Failure> {
        use crate::paper::{Failure, SettlementManifestEntry};
        let summary = self.paper_summary(run)?;
        if summary.control_state == "unresolved" || self.paper_has_unconfirmed_purchase(run)? {
            return Err(Failure::new("unresolved"));
        }
        if summary.control_state != "stopped" {
            return Err(Failure::new("run_active"));
        }
        let manifest: String = self
            .connection
            .query_row(
                "SELECT settlement_manifest FROM paper_runs WHERE run_id=?1",
                [run],
                |r| r.get(0),
            )
            .map_err(paper_storage)?;
        if manifest.len() > 16384 {
            return Err(Failure::new("resource_limit"));
        }
        let mut entries: Vec<SettlementManifestEntry> =
            serde_json::from_str(&manifest).map_err(|_| Failure::new("storage"))?;
        if entries.len() > 2 {
            return Err(Failure::new("resource_limit"));
        }
        entries.sort_by_key(|entry| entry.available_ms);
        let live_frontier = risk::settlement_frontier(&self.connection, run)?;
        for entry in entries {
            let pending:bool=self.connection.query_row("SELECT EXISTS(SELECT 1 FROM paper_markets m JOIN paper_attempts a USING(run_id,paper_market_id) LEFT JOIN paper_settlements s USING(run_id,paper_market_id) WHERE m.run_id=?1 AND m.source_market_id=?2 AND a.state IN ('full','partial') AND s.settlement_id IS NULL)",params![run,entry.source_market_id],|r|r.get(0)).map_err(paper_storage)?;
            if pending {
                let current = self
                    .paper_summary(run)?
                    .virtual_frontier_ms
                    .parse::<i64>()
                    .map_err(|_| Failure::new("storage"))?;
                let available =
                    i64::try_from(entry.available_ms).map_err(|_| Failure::new("range"))?;
                let at = live_frontier.unwrap_or(current.max(available));
                if at >= available {
                    self.paper_settle(run, &entry, at)?;
                }
            }
        }
        self.paper_summary(run)
    }
}

impl Recorder {
    pub(crate) fn paper_completion_revision(
        &self,
        run: i64,
    ) -> Result<Option<String>, crate::paper::Failure> {
        self.connection
            .query_row(
                "SELECT completed_revision FROM paper_runs WHERE run_id=?1",
                [run],
                |r| r.get::<_, Option<i64>>(0),
            )
            .map(|n| n.map(|n| n.to_string()))
            .map_err(paper_storage)
    }
}

impl Recorder {
    pub(crate) fn paper_latest(
        &self,
    ) -> Result<Option<crate::paper::RunSummary>, crate::paper::Failure> {
        #[cfg(test)]
        if self.fail_next_paper_latest.replace(false) {
            return Err(crate::paper::Failure::new("storage"));
        }
        let id: Option<i64> = self
            .connection
            .query_row("SELECT max(run_id) FROM paper_runs", [], |r| r.get(0))
            .map_err(paper_storage)?;
        id.map(|id| self.paper_summary(id)).transpose()
    }
    pub(crate) fn paper_history(
        &self,
        run: Option<i64>,
        before: Option<i64>,
        market: Option<i64>,
    ) -> Result<crate::paper::HistoryPage, crate::paper::Failure> {
        let mut statement=self.connection.prepare("SELECT m.run_id,m.paper_market_id,m.source_market_id,m.source_contract_id,m.series_id,m.open_utc_ms,m.close_utc_ms,m.close_day,m.reason,r.quantity,CASE WHEN a.state IN ('intent','dispatched','unknown') THEN a.state WHEN c.filled=r.quantity THEN 'full' WHEN c.filled>0 THEN 'partial' ELSE a.state END,coalesce(c.side,a.side),a.price,a.available_millionths,a.intent_virtual_ms,CASE WHEN a.attempt_id IS NOT NULL THEN coalesce(c.filled,0) END,CASE WHEN a.attempt_id IS NOT NULL THEN coalesce(c.cost,0) END,CASE WHEN a.attempt_id IS NOT NULL THEN coalesce(c.fee,0) END,CASE WHEN a.attempt_id IS NOT NULL THEN coalesce(c.cost+c.fee,0) END,s.payout,s.net,s.winning_side,s.effective_virtual_ms,s.recorded_at,r.strategy_version_id,json_extract(m.summary,'$.primary_witness'),coalesce(json_extract(m.summary,'$.more_evidence'),0),json_extract(m.summary,'$.purchase_check'),(SELECT count(*) FROM paper_attempts x WHERE x.paper_market_id=m.paper_market_id),m.participated,json_extract(m.summary,'$.risk_block') FROM paper_markets m JOIN paper_runs r USING(run_id) LEFT JOIN paper_attempts a ON a.attempt_id=(SELECT attempt_id FROM paper_attempts x WHERE x.paper_market_id=m.paper_market_id ORDER BY x.state IN ('intent','unknown','dispatched') DESC,attempt_id DESC LIMIT 1) LEFT JOIN (SELECT paper_market_id,max(side) AS side,sum(filled_quantity) AS filled,sum(cost) AS cost,sum(fee) AS fee FROM paper_attempts WHERE state IN ('full','partial') AND (?1 IS NULL OR run_id=?1) AND (?3 IS NULL OR paper_market_id=?3) GROUP BY paper_market_id) c ON c.paper_market_id=m.paper_market_id LEFT JOIN paper_settlements s USING(run_id,paper_market_id) WHERE (m.participated=1 OR json_extract(m.summary,'$.risk_block')='daily_entry_block') AND (?1 IS NULL OR m.run_id=?1) AND (?2 IS NULL OR m.paper_market_id<?2) AND (?3 IS NULL OR m.paper_market_id=?3) ORDER BY m.paper_market_id DESC LIMIT 51").map_err(paper_storage)?;
        let mut markets = statement
            .query_map(params![run, before, market], |r| {
                let amount = |i| r.get::<_, Option<i64>>(i).map(|n| n.map(|n| n.to_string()));
                Ok(crate::paper::MarketReceipt {
                    run_id: r.get::<_, i64>(0)?.to_string(),
                    paper_market_id: r.get::<_, i64>(1)?.to_string(),
                    source_market_id: r.get(2)?,
                    source_contract_id: r.get(3)?,
                    series_id: r.get(4)?,
                    open_utc_ms: r.get::<_, i64>(5)?.to_string(),
                    close_utc_ms: r.get::<_, i64>(6)?.to_string(),
                    close_day: r.get::<_, i64>(7)?.to_string(),
                    reason: r.get(8)?,
                    participated: r.get(29)?,
                    risk_block: r.get(30)?,
                    requested_quantity: r.get::<_, i64>(9)?.to_string(),
                    attempt_state: r.get(10)?,
                    side: r.get(11)?,
                    price: amount(12)?,
                    available_millionths: amount(13)?,
                    attempt_virtual_ms: amount(14)?,
                    filled_quantity: amount(15)?,
                    cost: amount(16)?,
                    fee: amount(17)?,
                    debit: amount(18)?,
                    payout: amount(19)?,
                    settled_net: amount(20)?,
                    winning_side: r.get(21)?,
                    settlement_virtual_ms: amount(22)?,
                    settlement_recorded_at: r.get(23)?,
                    strategy_version_id: r.get::<_, i64>(24)?.to_string(),
                    primary_witness: r
                        .get::<_, Option<String>>(25)?
                        .map(|s| {
                            serde_json::from_str(&s).map_err(|e| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    25,
                                    rusqlite::types::Type::Text,
                                    Box::new(e),
                                )
                            })
                        })
                        .transpose()?,
                    more_evidence: r.get(26)?,
                    purchase_check: r
                        .get::<_, Option<String>>(27)?
                        .map(|s| {
                            serde_json::from_str(&s).map_err(|e| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    27,
                                    rusqlite::types::Type::Text,
                                    Box::new(e),
                                )
                            })
                        })
                        .transpose()?,
                    attempt_count: r.get::<_, i64>(28)?.to_string(),
                    recovery: None,
                })
            })
            .map_err(paper_storage)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(paper_storage)?;
        for market in &mut markets {
            market.recovery = self.paper_recovery(
                market
                    .run_id
                    .parse()
                    .map_err(|_| crate::paper::Failure::new("storage"))?,
                Some(
                    market
                        .paper_market_id
                        .parse()
                        .map_err(|_| crate::paper::Failure::new("storage"))?,
                ),
            )?;
        }
        let next_cursor = if markets.len() > 50 {
            markets.pop();
            markets.last().map(|m| m.paper_market_id.clone())
        } else {
            None
        };
        Ok(crate::paper::HistoryPage {
            markets,
            next_cursor,
        })
    }
    pub(crate) fn paper_evidence(
        &self,
        run: i64,
        market: i64,
        before: Option<i64>,
    ) -> Result<crate::paper::EvidencePage, crate::paper::Failure> {
        let mut statement=self.connection.prepare("SELECT evidence_id,first_virtual_ms,last_virtual_ms,repetitions,latest_witness FROM paper_evidence WHERE run_id=?1 AND paper_market_id=?2 AND (?3 IS NULL OR evidence_id<?3) ORDER BY evidence_id DESC LIMIT 21").map_err(paper_storage)?;
        let mut evidence = statement
            .query_map(params![run, market, before], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, String>(4)?,
                ))
            })
            .map_err(paper_storage)?
            .map(|row| {
                let (id, first, last, count, witness) = row.map_err(paper_storage)?;
                Ok(crate::paper::EvidenceRecord {
                    evidence_id: id.to_string(),
                    first_virtual_ms: first.to_string(),
                    last_virtual_ms: last.to_string(),
                    repetitions: count.to_string(),
                    witness: serde_json::from_str(&witness)
                        .map_err(|_| crate::paper::Failure::new("storage"))?,
                })
            })
            .collect::<Result<Vec<_>, crate::paper::Failure>>()?;
        let next_cursor = if evidence.len() > 20 {
            evidence.pop();
            evidence.last().map(|e| e.evidence_id.clone())
        } else {
            None
        };
        Ok(crate::paper::EvidencePage {
            evidence,
            next_cursor,
        })
    }
}

const PAPER_START_BLOCKED: &str = "SELECT EXISTS(SELECT 1 FROM paper_runs WHERE control_state!='stopped')
    OR (?1 AND EXISTS(SELECT 1 FROM paper_runs r JOIN run_risk p USING(run_id)
                      WHERE p.scope=0 AND r.error_code='report_conflict'))
    OR EXISTS(SELECT 1 FROM paper_attempts a LEFT JOIN paper_settlements s USING(run_id,paper_market_id)
       LEFT JOIN run_risk r ON r.run_id=a.run_id
       WHERE a.state IN ('intent','dispatched','unknown')
       OR (a.state IN ('full','partial') AND s.settlement_id IS NULL
           AND coalesce(r.scope=0,0)=?1 AND NOT ?1))";

impl Recorder {
    pub(crate) fn paper_prepare_id(
        &self,
        uuid: &str,
        live: bool,
    ) -> Result<(i64, bool), crate::paper::Failure> {
        if let Some(id) = self
            .connection
            .query_row(
                "SELECT run_id FROM paper_runs WHERE start_uuid=?1",
                [uuid],
                |r| r.get::<_, i64>(0),
            )
            .optional()
            .map_err(paper_storage)?
        {
            return Ok((id, true));
        }
        if self
            .connection
            .query_row(PAPER_START_BLOCKED, [live], |r| r.get::<_, bool>(0))
            .map_err(paper_storage)?
        {
            return Err(crate::paper::Failure::new("run_active"));
        }
        let last: i64 = self
            .connection
            .query_row("SELECT coalesce(max(run_id),0) FROM paper_runs", [], |r| {
                r.get(0)
            })
            .map_err(paper_storage)?;
        Ok((
            last.checked_add(1)
                .filter(|n| *n > 0)
                .ok_or_else(|| crate::paper::Failure::new("resource_limit"))?,
            false,
        ))
    }
}

#[cfg(test)]
mod paper_transaction_tests {
    use super::*;
    use crate::{paper::*, strategy::*};

    #[test]
    fn dropping_the_recorder_releases_the_lock_even_with_a_duplicate_descriptor() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("workspace.sqlite");
        let (recorder, _) = Recorder::open(&path).unwrap();
        let inherited = recorder._lock.0.try_clone().unwrap();
        assert!(Recorder::open(&path).is_err());
        drop(recorder);

        let (reopened, _) = Recorder::open(&path).unwrap();
        drop(inherited);
        assert!(Recorder::open(&path).is_err());
        drop(reopened);
        assert!(Recorder::open(&path).is_ok());
    }

    #[test]
    fn paper_venue_waits_longer_for_the_write_lock_than_the_recorder() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("paper.sqlite");
        let (recorder, _) = Recorder::open(&path).unwrap();
        let venue = PaperVenue::open(&path).unwrap();
        let timeout = |c: &Connection| {
            c.query_row("PRAGMA busy_timeout", [], |r| r.get::<_, i64>(0))
                .unwrap()
        };
        assert_eq!(timeout(&venue.connection), 2000);
        assert_eq!(timeout(&recorder.connection), 250);
    }

    #[test]
    fn terminal_outcome_and_settlement_are_idempotent_and_conflicts_preserve_money() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("paper.sqlite");
        let engine = crate::Engine::open(&path).unwrap();
        let text = include_str!("../../../examples/learning.json");
        let crate::StrategyResponse::Saved { version } = engine
            .strategies(crate::StrategyRequest::Save {
                json: text.into(),
                reviewed_json: text.into(),
            })
            .unwrap()
        else {
            panic!("saved version")
        };
        engine.stop_and_save().unwrap();
        drop(engine);
        let (mut recorder, _) = Recorder::open(&path).unwrap();
        let scenario = Scenario::parse(include_str!(
            "../../../examples/paper-learning-scenario.json"
        ))
        .unwrap();
        let start = scenario
            .start(
                "00000000-0000-4000-8000-000000000001".into(),
                version.id.parse().unwrap(),
                Settings {
                    quantity: 10,
                    minimum_price: Decimal::parse("0").unwrap(),
                    maximum_price: Decimal::parse("100").unwrap(),
                    starting_balance: 100 * DOLLAR,
                    limits: None,
                },
                1795046400000,
            )
            .unwrap();
        recorder.paper_start(&start, 1, false).unwrap();
        let market = &start.markets[1];
        recorder
            .paper_market_progress(1, &market.market_id, "eligible", true)
            .unwrap();
        let candidate = Candidate {
            market_id: market.market_id.clone(),
            option_id: "opposite".into(),
            side: Side::No,
            remaining_quantity: 10,
            ask: AskWitness {
                binding_id: 4,
                price: Decimal::parse("25").unwrap(),
                value_transaction: 1,
                observation_number: 1,
                valid_until_ms: 1000000,
            },
        };
        let book = Book {
            source_market_id: market.market_id.clone(),
            source_contract_id: market.contract_id.clone(),
            side: Side::No,
            binding_id: 4,
            source_generation: 1,
            value_transaction: 1,
            observation_number: 1,
            ask: candidate.ask.price,
            available_millionths: 10000000,
            accepted_ms: 61000,
            valid_until_ms: 1000000,
        };
        let attempt = recorder
            .paper_intent(1, &candidate, &book, 61000)
            .unwrap()
            .unwrap();
        // An unsent intent can be canceled but never filled.
        assert_eq!(
            recorder
                .paper_finish_attempt(1, attempt, Some(simulate(10, book.ask, 10000000).unwrap()))
                .unwrap_err()
                .code,
            "input_conflict"
        );
        recorder.paper_dispatch(1, attempt, 61000).unwrap();
        // A possibly sent order can never be canceled.
        assert_eq!(
            recorder
                .paper_finish_attempt(1, attempt, None)
                .unwrap_err()
                .code,
            "input_conflict"
        );
        let fill = simulate(10, book.ask, book.available_millionths).unwrap();
        // A confirmed fill must carry the pinned fee exactly, not merely fit the reservation.
        let Fill::Full { charge: exact, .. } = fill else {
            panic!("full fill")
        };
        let feeless = Fill::Full {
            quantity: 10,
            charge: crate::paper::Charge {
                fee: 0,
                debit: exact.cost,
                ..exact
            },
        };
        assert_eq!(
            recorder
                .paper_finish_attempt(1, attempt, Some(feeless))
                .unwrap_err()
                .code,
            "input_conflict"
        );
        recorder
            .paper_finish_attempt(1, attempt, Some(fill))
            .unwrap();
        let confirmed = recorder.paper_summary(1).unwrap();
        assert_eq!(confirmed.cash, "9736870000");
        recorder
            .paper_finish_attempt(1, attempt, Some(fill))
            .unwrap();
        assert_eq!(recorder.paper_summary(1).unwrap(), confirmed);
        assert_eq!(
            recorder
                .paper_finish_attempt(
                    1,
                    attempt,
                    Some(Fill::Zero {
                        available_millionths: 0
                    })
                )
                .unwrap_err()
                .code,
            "input_conflict"
        );
        assert_eq!(recorder.paper_summary(1).unwrap(), confirmed);
        let entry = &start.settlement_manifest[0];
        recorder.paper_settle(1, entry, 901000).unwrap();
        let settled = recorder.paper_summary(1).unwrap();
        assert_eq!(settled.cash, "10736870000");
        recorder.paper_settle(1, entry, 902000).unwrap();
        assert_eq!(recorder.paper_summary(1).unwrap(), settled);
        let mut conflict = entry.clone();
        conflict.winning_side = Side::Yes;
        assert_eq!(
            recorder
                .paper_settle(1, &conflict, 902000)
                .unwrap_err()
                .code,
            "input_conflict"
        );
        conflict = entry.clone();
        conflict.source_contract_id.push('x');
        assert_eq!(
            recorder
                .paper_settle(1, &conflict, 902000)
                .unwrap_err()
                .code,
            "input_conflict"
        );
        assert_eq!(recorder.paper_summary(1).unwrap(), settled);
        // Canceling a retry after a confirmed zero fill keeps the zero fill's reason.
        let market = &start.markets[0];
        recorder
            .paper_market_progress(1, &market.market_id, "eligible", true)
            .unwrap();
        let mut candidate = Candidate {
            market_id: market.market_id.clone(),
            ..candidate
        };
        let mut book = Book {
            source_market_id: market.market_id.clone(),
            source_contract_id: market.contract_id.clone(),
            available_millionths: 0,
            ..book
        };
        let zero = Some(Fill::Zero {
            available_millionths: 0,
        });
        let attempt = recorder
            .paper_intent(1, &candidate, &book, 903000)
            .unwrap()
            .unwrap();
        recorder.paper_dispatch(1, attempt, 903000).unwrap();
        recorder.paper_finish_attempt(1, attempt, zero).unwrap();
        candidate.ask.observation_number = 2;
        book.observation_number = 2;
        let attempt = recorder
            .paper_intent(1, &candidate, &book, 904000)
            .unwrap()
            .unwrap();
        recorder.paper_finish_attempt(1, attempt, None).unwrap();
        let reason: String = recorder
            .connection
            .query_row(
                "SELECT reason FROM paper_markets WHERE run_id=1 AND source_market_id=?1",
                [&market.market_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(reason, "insufficient_quantity");
    }
}
