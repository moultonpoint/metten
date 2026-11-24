use super::{Recorder, paper_storage};
use crate::paper::{DailyLimits, DailyRisk, Failure, objective_bound};
use crate::strategy::Decimal;
use rusqlite::{Connection, OptionalExtension, params};

const DAY_MS: i64 = 86_400_000;

pub(super) const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS run_risk (
    run_id INTEGER PRIMARY KEY REFERENCES paper_runs(run_id),
    scope INTEGER NOT NULL CHECK(scope IN (0,run_id)),
    maximum_loss INTEGER NOT NULL CHECK(maximum_loss>0),
    profit_target INTEGER CHECK(profit_target>0),
    resume_next_day INTEGER NOT NULL CHECK(resume_next_day IN (0,1)),
    funds_adjustment INTEGER NOT NULL DEFAULT 0 CHECK(scope=0 OR funds_adjustment=0),
    stopped_day INTEGER
) STRICT;
CREATE INDEX IF NOT EXISTS risk_runs_by_scope ON run_risk(scope,run_id);
CREATE TABLE IF NOT EXISTS daily_stops (
    scope INTEGER NOT NULL,
    utc_day INTEGER NOT NULL,
    reason TEXT NOT NULL CHECK(reason IN ('daily_loss','daily_profit')),
    PRIMARY KEY(scope,utc_day)
) STRICT;
CREATE INDEX IF NOT EXISTS settlements_by_day ON paper_settlements(close_day,run_id);
PRAGMA user_version=7;";

#[derive(Clone, Copy)]
struct Policy {
    scope: i64,
    limits: DailyLimits,
    stopped_day: Option<i64>,
}

impl Policy {
    fn read(db: &Connection, run: i64) -> Result<Option<Self>, Failure> {
        db.query_row(
            "SELECT scope,maximum_loss,profit_target,resume_next_day,stopped_day
             FROM run_risk WHERE run_id=?1",
            [run],
            |row| {
                Ok(Self {
                    scope: row.get(0)?,
                    limits: DailyLimits {
                        maximum_loss: row.get(1)?,
                        profit_target: row.get(2)?,
                        resume_next_day: row.get(3)?,
                    },
                    stopped_day: row.get(4)?,
                })
            },
        )
        .optional()
        .map_err(paper_storage)
    }

    fn pending(self, db: &Connection) -> Result<(i64, i64), Failure> {
        db.query_row(
            "SELECT coalesce(sum(a.cost+a.fee) FILTER (WHERE a.state IN ('full','partial')),0),
                    coalesce(sum(a.reserved) FILTER (WHERE a.state IN ('intent','dispatched','unknown')),0)
             FROM run_risk r JOIN paper_attempts a USING(run_id)
             LEFT JOIN paper_settlements s USING(run_id,paper_market_id)
             WHERE r.scope=?1 AND s.settlement_id IS NULL",
            [self.scope],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).map_err(paper_storage)
    }

    fn settled(self, db: &Connection, day: i64) -> Result<i64, Failure> {
        db.query_row(
            "SELECT coalesce(sum(s.net),0) FROM paper_settlements s
             JOIN run_risk r USING(run_id) WHERE r.scope=?1 AND s.close_day=?2",
            params![self.scope, day],
            |row| row.get(0),
        )
        .map_err(paper_storage)
    }

    fn stop(self, db: &Connection, day: i64) -> Result<Option<String>, Failure> {
        db.query_row(
            "SELECT reason FROM daily_stops WHERE scope=?1 AND utc_day=?2",
            params![self.scope, day],
            |row| row.get(0),
        )
        .optional()
        .map_err(paper_storage)
    }

    fn latch(self, db: &Connection, day: i64) -> Result<bool, Failure> {
        if let Some(reason) = self.limits.reached(self.settled(db, day)?) {
            db.execute(
                "INSERT INTO daily_stops(scope,utc_day,reason) VALUES(?1,?2,?3)
                 ON CONFLICT(scope,utc_day) DO NOTHING",
                params![self.scope, day, reason],
            )
            .map_err(paper_storage)?;
        }
        Ok(self.stop(db, day)?.is_some())
    }

    fn permits(self, db: &Connection, day: i64, exposure: i64) -> Result<bool, Failure> {
        Ok(i128::from(self.settled(db, day)?) - i128::from(exposure)
            >= -i128::from(self.limits.maximum_loss))
    }
}

fn utc_day(go: i64, at: i64) -> Result<i64, Failure> {
    go.checked_add(at)
        .map(|utc| utc.div_euclid(DAY_MS))
        .ok_or_else(|| Failure::new("range"))
}

pub(super) fn save_settings(
    db: &Connection,
    run: i64,
    live: bool,
    limits: Option<DailyLimits>,
    go: i64,
) -> Result<(), Failure> {
    let Some(limits) = limits else {
        return if live {
            Err(Failure::new("range"))
        } else {
            Ok(())
        };
    };
    let scope = if live { 0 } else { run };
    if live {
        let last: Option<i64> = db
            .query_row(
                "SELECT max(r.go_utc_ms + r.virtual_frontier_ms)
             FROM run_risk p JOIN paper_runs r USING(run_id) WHERE p.scope=0",
                [],
                |row| row.get(0),
            )
            .map_err(paper_storage)?;
        if last.is_some_and(|last| go < last) {
            return Err(Failure::new("range"));
        }
    }
    let policy = Policy {
        scope,
        limits,
        stopped_day: None,
    };
    let day = utc_day(go, 0)?;
    let stopped = policy.latch(db, day)?.then_some(day);
    db.execute(
        "INSERT INTO run_risk(run_id,scope,maximum_loss,profit_target,resume_next_day,stopped_day)
         VALUES(?1,?2,?3,?4,?5,?6)",
        params![
            run,
            scope,
            limits.maximum_loss,
            limits.profit_target,
            limits.resume_next_day,
            stopped
        ],
    )
    .map_err(paper_storage)?;
    Ok(())
}

pub(super) fn matches_settings(
    db: &Connection,
    run: i64,
    live: bool,
    limits: Option<DailyLimits>,
) -> Result<bool, Failure> {
    Ok(match Policy::read(db, run)? {
        Some(policy) => (policy.scope == 0) == live && Some(policy.limits) == limits,
        None => !live && limits.is_none(),
    })
}

pub(super) struct PurchaseRisk {
    pub run: i64,
    pub at: i64,
    pub market: i64,
    pub ask: Decimal,
    pub ceiling: Decimal,
    pub quantity: u64,
    pub first_fill: bool,
    pub cash_after_reservations: i64,
}

pub(super) fn reserve(db: &Connection, purchase: PurchaseRisk) -> Result<Option<i64>, Failure> {
    let Some(policy) = Policy::read(db, purchase.run)? else {
        return Ok(None);
    };
    let (go, close_day): (i64, i64) = db
        .query_row(
            "SELECT r.go_utc_ms,m.close_day FROM paper_runs r JOIN paper_markets m USING(run_id)
         WHERE r.run_id=?1 AND m.paper_market_id=?2",
            params![purchase.run, purchase.market],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(paper_storage)?;
    let bound = objective_bound(purchase.quantity, purchase.ask)?;
    let required = if purchase.first_fill {
        objective_bound(purchase.quantity, purchase.ceiling)?
    } else {
        bound
    };
    if purchase.cash_after_reservations < required {
        return Err(Failure::new("insufficient_funds"));
    }
    let (holding, attempts) = policy.pending(db)?;
    let exposure = holding
        .checked_add(attempts)
        .and_then(|n| n.checked_add(required))
        .ok_or_else(|| Failure::new("range"))?;
    let today = utc_day(go, purchase.at)?;
    if !policy.permits(db, today, exposure)?
        || (today != close_day && !policy.permits(db, close_day, exposure)?)
    {
        return Err(Failure::new("daily_budget"));
    }
    Ok(Some(bound))
}

pub(super) fn settlement_frontier(db: &Connection, run: i64) -> Result<Option<i64>, Failure> {
    db.query_row(
        "SELECT max(r.go_utc_ms+r.virtual_frontier_ms)-target.go_utc_ms
         FROM run_risk p JOIN paper_runs r USING(run_id) JOIN paper_runs target ON target.run_id=?1
         WHERE p.scope=0 AND EXISTS(SELECT 1 FROM run_risk WHERE run_id=?1 AND scope=0)",
        [run],
        |row| row.get(0),
    )
    .map_err(paper_storage)
}

pub(super) fn settled(db: &Connection, run: i64, close_day: i64, at: i64) -> Result<(), Failure> {
    let Some(paid) = Policy::read(db, run)? else {
        return Ok(());
    };
    let authority = db
        .query_row(
            "SELECT r.run_id,r.go_utc_ms,r.virtual_frontier_ms,r.control_state FROM run_risk p
         JOIN paper_runs r USING(run_id) WHERE p.scope=?1 ORDER BY r.run_id DESC LIMIT 1",
            [paid.scope],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .map_err(paper_storage)?;
    let policy = Policy::read(db, authority.0)?.ok_or_else(|| Failure::new("storage"))?;
    let go: i64 = db
        .query_row(
            "SELECT go_utc_ms FROM paper_runs WHERE run_id=?1",
            [run],
            |row| row.get(0),
        )
        .map_err(paper_storage)?;
    let today = utc_day(go, at)?.max(utc_day(authority.1, authority.2)?);
    if policy.latch(db, close_day)?
        && close_day == today
        && matches!(
            authority.3.as_str(),
            "collecting" | "waiting" | "active" | "paused"
        )
    {
        db.execute(
            "UPDATE run_risk SET stopped_day=coalesce(stopped_day,?2) WHERE run_id=?1",
            params![authority.0, close_day],
        )
        .map_err(paper_storage)?;
        if authority.0 != run {
            db.execute(
                "UPDATE paper_runs SET revision=revision+1 WHERE run_id=?1",
                [authority.0],
            )
            .map_err(paper_storage)?;
        }
    }
    Ok(())
}

impl Recorder {
    #[cfg(test)]
    pub(crate) fn fixture_live_cash(&mut self, run: i64, cash: i64) -> Result<(), Failure> {
        let tx = self.connection.transaction().map_err(paper_storage)?;
        let (previous, adjustment): (i64, i64) = tx
            .query_row(
                "SELECT r.cash,p.funds_adjustment FROM paper_runs r JOIN run_risk p USING(run_id)
             WHERE r.run_id=?1 AND p.scope=0 AND r.reserved=0",
                [run],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(paper_storage)?;
        let updated =
            i64::try_from(i128::from(adjustment) + i128::from(cash) - i128::from(previous))
                .map_err(|_| Failure::new("range"))?;
        tx.execute(
            "UPDATE run_risk SET funds_adjustment=?2 WHERE run_id=?1",
            params![run, updated],
        )
        .map_err(paper_storage)?;
        tx.execute(
            "UPDATE paper_runs SET cash=?2,revision=revision+1 WHERE run_id=?1",
            params![run, cash],
        )
        .map_err(paper_storage)?;
        tx.commit().map_err(paper_storage)
    }

    pub(super) fn daily_risk(&self, run: i64, day: i64) -> Result<Option<DailyRisk>, Failure> {
        let db = &self.connection;
        let Some(policy) = Policy::read(db, run)? else {
            return Ok(None);
        };
        let (holding, attempts) = policy.pending(db)?;
        Ok(Some(DailyRisk {
            mode: if policy.scope == 0 {
                "controlled_live"
            } else {
                "paper"
            },
            utc_day: day.to_string(),
            settled_net: policy.settled(db, day)?.to_string(),
            pending_holdings: holding.to_string(),
            pending_attempts: attempts.to_string(),
            maximum_loss: Some(policy.limits.maximum_loss.to_string()),
            profit_target: policy.limits.profit_target.map(|n| n.to_string()),
            resume_next_day: policy.limits.resume_next_day,
            stop_reason: policy.stop(db, policy.stopped_day.unwrap_or(day))?,
        }))
    }

    pub(crate) fn paper_allow_entry(
        &mut self,
        run: i64,
        market: &str,
        at: i64,
        safe_to_resume: bool,
    ) -> Result<bool, Failure> {
        let tx = self.connection.transaction().map_err(paper_storage)?;
        let Some(policy) = Policy::read(&tx, run)? else {
            return Ok(true);
        };
        let (go, close_day): (i64, i64) = tx.query_row(
            "SELECT r.go_utc_ms,m.close_day FROM paper_markets m JOIN paper_runs r USING(run_id)
             WHERE m.run_id=?1 AND m.source_market_id=?2",
            params![run, market], |row| Ok((row.get(0)?,row.get(1)?)),
        ).map_err(paper_storage)?;
        let today = utc_day(go, at)?;
        let blocked_day = if policy.stop(&tx, today)?.is_some() {
            Some(today)
        } else if policy.stop(&tx, close_day)?.is_some() {
            Some(close_day)
        } else {
            policy
                .stopped_day
                .filter(|day| today <= *day || !policy.limits.resume_next_day || !safe_to_resume)
        };
        tx.execute(
            "UPDATE run_risk SET stopped_day=?2 WHERE run_id=?1",
            params![run, blocked_day],
        )
        .map_err(paper_storage)?;
        if blocked_day.is_some() {
            tx.execute(
                "UPDATE paper_markets SET summary=json_set(summary,'$.risk_block','daily_entry_block'),
                 evidence_revision=evidence_revision+1 WHERE run_id=?1 AND source_market_id=?2
                 AND participated=0",
                params![run, market],
            ).map_err(paper_storage)?;
        }
        tx.commit().map_err(paper_storage)?;
        Ok(blocked_day.is_none())
    }

    pub(crate) fn paper_risk_refusal(
        &self,
        run: i64,
        market: &str,
        reason: &str,
    ) -> Result<(), Failure> {
        self.connection
            .execute(
                "UPDATE paper_markets SET summary=json_set(summary,'$.risk_block',?3,'$.current_risk_block',?3),
             evidence_revision=evidence_revision+1 WHERE run_id=?1 AND source_market_id=?2
             AND (json_extract(summary,'$.risk_block') IS NOT ?3
             OR json_extract(summary,'$.current_risk_block') IS NOT ?3)",
                params![run, market, reason],
            )
            .map_err(paper_storage)?;
        Ok(())
    }
}
