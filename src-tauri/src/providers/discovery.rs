//! One bounded Kalshi current/next discovery, independent of screen refresh.
use super::{Failure, Fence, Route, SourceCommand, transport::Transport, wire::*};
use metten_engine::strategy::MarketDescriptor;
use serde::Deserialize;
use serde_json::value::RawValue;
use tokio::{
    sync::watch,
    time::{Duration, Instant},
};
#[derive(Deserialize)]
struct Page<'a> {
    #[serde(borrow)]
    markets: &'a RawValue,
    #[serde(borrow)]
    cursor: Option<std::borrow::Cow<'a, str>>,
}
#[derive(Deserialize)]
struct Market<'a> {
    ticker: &'a str,
    event_ticker: &'a str,
    market_id: Option<&'a str>,
    open_time: &'a str,
    close_time: &'a str,
    status: &'a str,
}
#[derive(Deserialize)]
struct Detail<'a> {
    #[serde(borrow)]
    market: Market<'a>,
}
pub struct Discovered {
    pub markets: Vec<MarketDescriptor>,
    _charge: tokio::sync::OwnedSemaphorePermit,
}
#[allow(clippy::too_many_arguments)] // Keep fixed transport, cancellation and captured metadata identity explicit.
pub async fn pair(
    transport: &Transport,
    stop: &mut watch::Receiver<bool>,
    fence: &Fence,
    now: i64,
    epoch: i64,
    startup: bool,
    parser: &tokio::sync::mpsc::Sender<SourceCommand>,
    token: metten_engine::sources::StreamToken,
) -> Result<Discovered, Failure> {
    if now < 0 || epoch < 0 || epoch > now {
        return Err(Failure::InvalidRequest);
    }
    let _collector = transport.memory().reserve(256 * 1024)?;
    let charge = transport.memory().reserve(2 * 512)?;
    let deadline = Instant::now() + Duration::from_secs(10);
    if startup {
        for route in [Route::ExchangeStatus, Route::Series] {
            fetch(
                transport, stop, fence, route, epoch, now, deadline, parser, token,
            )
            .await?;
        }
    }
    let mut cursor = None;
    let mut seen_cursor = None;
    let mut candidates = Vec::new();
    for page_index in 0..2 {
        let parsed = fetch(
            transport,
            stop,
            fence,
            Route::Markets {
                min_close_ts: now as u64 / 1000,
                max_close_ts: (now as u64 / 1000)
                    .checked_add(3600)
                    .ok_or(Failure::InvalidRequest)?,
                cursor: cursor.clone(),
            },
            epoch,
            now,
            deadline,
            parser,
            token,
        )
        .await?;
        let Parsed {
            data,
            _charge: _response_charge,
        } = parsed;
        let Metadata::Page {
            markets: rows,
            cursor: page_cursor,
        } = data
        else {
            return Err(Failure::Protocol);
        };
        for descriptor in rows {
            if descriptor.close_ms > now - epoch
                && candidates
                    .iter()
                    .all(|m: &MarketDescriptor| m.market_id != descriptor.market_id)
            {
                if candidates.len() == 400 {
                    return Err(Failure::Capacity);
                }
                candidates.push(descriptor)
            }
        }
        match page_cursor.as_deref().filter(|c| !c.is_empty()) {
            None => break,
            Some(next) => {
                if page_index == 1 || seen_cursor.as_deref() == Some(next) {
                    return Err(Failure::Protocol);
                }
                seen_cursor = Some(next.to_owned());
                cursor = Some(next.to_owned());
            }
        }
    }
    let selected = select_pair(
        candidates,
        now.checked_sub(epoch).ok_or(Failure::InvalidRequest)?,
    )?;
    let mut confirmed = Vec::new();
    for market in selected {
        let parsed = fetch(
            transport,
            stop,
            fence,
            Route::Market(market.market_id.clone()),
            epoch,
            now,
            deadline,
            parser,
            token,
        )
        .await?;
        let Parsed {
            data,
            _charge: _response_charge,
        } = parsed;
        let Metadata::Market(exact) = data else {
            return Err(Failure::Protocol);
        };
        if exact != market {
            return Err(Failure::Protocol);
        }
        confirmed.push(exact);
    }
    Ok(Discovered {
        markets: confirmed,
        _charge: charge,
    })
}
pub struct Parsed {
    pub data: Metadata,
    _charge: tokio::sync::OwnedSemaphorePermit,
}
pub enum Metadata {
    Validated,
    Page {
        markets: Vec<MarketDescriptor>,
        cursor: Option<String>,
    },
    Market(MarketDescriptor),
}
// Only the existing worker invokes this; HTTP tasks retain route, generation and charged bytes.
pub fn parse(
    route: &Route,
    bytes: &[u8],
    epoch: i64,
    now: i64,
    memory: &super::memory::Memory,
) -> Result<Parsed, Failure> {
    let charge = memory.reserve(4 * super::MAX_BYTES)?;
    let data = match route {
        Route::ExchangeStatus => {
            #[derive(Deserialize)]
            struct Status {
                exchange_active: bool,
            }
            if !decode::<Status>(bytes)?.exchange_active {
                return Err(Failure::Unavailable);
            }
            Ok(Metadata::Validated)
        }
        Route::Series => {
            #[derive(Deserialize)]
            struct Series<'a> {
                #[serde(borrow)]
                series: Id<'a>,
            }
            #[derive(Deserialize)]
            struct Id<'a> {
                ticker: &'a str,
            }
            if decode::<Series<'_>>(bytes)?.series.ticker != "KXBTC15M" {
                return Err(Failure::Protocol);
            }
            Ok(Metadata::Validated)
        }
        Route::Markets { .. } => {
            let page: Page<'_> = decode(bytes)?;
            if page.cursor.as_ref().is_some_and(|c| c.len() > 1024) {
                return Err(Failure::Capacity);
            }
            let rows: List<Market<'_>> = decode(page.markets.get().as_bytes())?;
            if rows.0.len() > 200 {
                return Err(Failure::Capacity);
            }
            Ok(Metadata::Page {
                markets: rows
                    .0
                    .iter()
                    .map(|m| descriptor(m, epoch, now))
                    .collect::<Result<_, _>>()?,
                cursor: page.cursor.map(|c| c.into_owned()),
            })
        }
        Route::Market(ticker) => {
            let detail: Detail<'_> = decode(bytes)?;
            let market = descriptor(&detail.market, epoch, now)?;
            if market.market_id != *ticker {
                return Err(Failure::Protocol);
            }
            Ok(Metadata::Market(market))
        }
        _ => Err(Failure::InvalidRequest),
    }?;
    Ok(Parsed {
        data,
        _charge: charge,
    })
}

#[allow(clippy::too_many_arguments)] // The receipt must retain route, epoch and captured token through the worker handoff.
async fn fetch(
    transport: &Transport,
    stop: &mut watch::Receiver<bool>,
    fence: &Fence,
    route: Route,
    epoch: i64,
    now: i64,
    deadline: Instant,
    parser: &tokio::sync::mpsc::Sender<SourceCommand>,
    token: metten_engine::sources::StreamToken,
) -> Result<Parsed, Failure> {
    let bytes = transport.get(&route, stop, deadline, fence).await?;
    let (reply, receive) = tokio::sync::oneshot::channel();
    fence
        .run(
            deadline,
            super::bounded(stop, deadline, async {
                parser
                    .send(SourceCommand::Metadata {
                        route,
                        bytes,
                        epoch,
                        now,
                        token,
                        reply,
                    })
                    .await
                    .map_err(|_| Failure::Stopped)?;
                receive.await.map_err(|_| Failure::Stopped)?
            }),
        )
        .await
}
#[allow(clippy::too_many_arguments)] // Capture UTC with the same metadata identity; no second mutable catalog.
pub async fn revalidate(
    transport: &Transport,
    stop: &mut watch::Receiver<bool>,
    fence: &Fence,
    markets: &[MarketDescriptor],
    epoch: i64,
    now: i64,
    parser: &tokio::sync::mpsc::Sender<SourceCommand>,
    token: metten_engine::sources::StreamToken,
) -> Result<Discovered, Failure> {
    if markets.len() > 2 {
        return Err(Failure::Capacity);
    }
    let _scratch = transport.memory().reserve(256 * 1024)?;
    let charge = transport.memory().reserve(2 * 512)?;
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut confirmed = Vec::new();
    for market in markets {
        let parsed = fetch(
            transport,
            stop,
            fence,
            Route::Market(market.market_id.clone()),
            epoch,
            now,
            deadline,
            parser,
            token,
        )
        .await?;
        let Parsed {
            data,
            _charge: _response_charge,
        } = parsed;
        let Metadata::Market(m) = data else {
            return Err(Failure::Protocol);
        };
        confirmed.push(m);
    }
    Ok(Discovered {
        markets: confirmed,
        _charge: charge,
    })
}
fn descriptor(row: &Market<'_>, epoch: i64, now: i64) -> Result<MarketDescriptor, Failure> {
    identity(row.ticker)?;
    identity(row.event_ticker)?;
    if !row.ticker.starts_with("KXBTC15M-")
        || !row.event_ticker.starts_with("KXBTC15M-")
        || !matches!(row.status, "initialized" | "active")
    {
        return Err(Failure::Protocol);
    }
    if row.status == "initialized" && utc_ms(row.open_time)? <= now {
        return Err(Failure::Unavailable);
    }
    let open = utc_ms(row.open_time)?
        .checked_sub(epoch)
        .ok_or(Failure::Protocol)?;
    let close = utc_ms(row.close_time)?
        .checked_sub(epoch)
        .ok_or(Failure::Protocol)?;
    if close.checked_sub(open) != Some(900000) {
        return Err(Failure::Protocol);
    }
    let contract = row.market_id.unwrap_or(row.ticker);
    identity(contract)?;
    Ok(MarketDescriptor {
        series_id: "KXBTC15M".into(),
        market_id: row.ticker.into(),
        contract_id: contract.into(),
        open_ms: open,
        close_ms: close,
    })
}
pub fn select_pair(
    mut markets: Vec<MarketDescriptor>,
    now: i64,
) -> Result<Vec<MarketDescriptor>, Failure> {
    if markets.len() > 400 {
        return Err(Failure::Capacity);
    }
    markets.sort_unstable_by_key(|m| m.open_ms);
    let current: Vec<_> = markets
        .iter()
        .filter(|m| m.open_ms <= now && now < m.close_ms)
        .collect();
    if current.len() != 1 {
        return Err(Failure::Unavailable);
    }
    let current = current[0].clone();
    let next: Vec<_> = markets
        .iter()
        .filter(|m| m.open_ms == current.close_ms)
        .collect();
    if next.len() > 1 {
        return Err(Failure::Unavailable);
    }
    let mut pair = vec![current];
    if let Some(next) = next.first() {
        pair.push((*next).clone())
    }
    Ok(pair)
}
// Strict UTC timestamps from fixed provider metadata; no timezone or leap-second guesses.
pub fn utc_ms(text: &str) -> Result<i64, Failure> {
    let b = text.as_bytes();
    if b.len() < 20
        || b.get(4) != Some(&b'-')
        || b.get(7) != Some(&b'-')
        || b.get(10) != Some(&b'T')
        || b.get(13) != Some(&b':')
        || b.get(16) != Some(&b':')
        || b.last() != Some(&b'Z')
    {
        return Err(Failure::Protocol);
    }
    let n = |a: usize, z: usize| -> Result<i64, Failure> {
        let s = text.get(a..z).ok_or(Failure::Protocol)?;
        if !s.bytes().all(|b| b.is_ascii_digit()) {
            return Err(Failure::Protocol);
        }
        s.parse().map_err(|_| Failure::Protocol)
    };
    let (y, m, d, h, min, sec) = (
        n(0, 4)?,
        n(5, 7)?,
        n(8, 10)?,
        n(11, 13)?,
        n(14, 16)?,
        n(17, 19)?,
    );
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if !(1..=12).contains(&m)
        || d < 1
        || d > days[(m - 1) as usize]
        || h > 23
        || min > 59
        || sec > 59
    {
        return Err(Failure::Protocol);
    }
    let fraction = if b.len() == 20 {
        0
    } else {
        if b.get(19) != Some(&b'.') || !(22..=24).contains(&b.len()) {
            return Err(Failure::Protocol);
        }
        let len = b.len() - 21;
        n(20, b.len() - 1)? * 10_i64.pow((3 - len) as u32)
    };
    let year = y - i64::from(m <= 2);
    let era = year.div_euclid(400);
    let yoe = year - era * 400;
    let shifted = m + if m > 2 { -3 } else { 9 };
    let doy = (153 * shifted + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Ok(((era * 146097 + doe - 719468) * 86400 + h * 3600 + min * 60 + sec) * 1000 + fraction)
}

/// Three missing-work opportunities, never replayed after setup or sleep.
pub struct PreparationChecks {
    open: i64,
    passed: u8,
    next_missing: Option<i64>,
}
impl PreparationChecks {
    pub fn new(open: i64, now: i64) -> Self {
        let mut checks = Self {
            open,
            passed: 0,
            next_missing: None,
        };
        checks.skip_through(now);
        checks
    }
    pub fn skip_through(&mut self, now: i64) {
        for (i, seconds) in [60, 30, 10].into_iter().enumerate() {
            if self.open.saturating_sub(seconds * 1000) <= now {
                self.passed |= 1 << i
            }
        }
    }
    pub fn due(&mut self, now: i64, ready: bool, in_flight: bool, stopped: bool) -> bool {
        let mut due = false;
        for (i, seconds) in [60, 30, 10].into_iter().enumerate() {
            if self.passed & (1 << i) == 0 && now >= self.open.saturating_sub(seconds * 1000) {
                self.passed |= 1 << i;
                due = true
            }
        }
        if now >= self.open && self.next_missing.is_none_or(|t| now >= t) {
            self.next_missing = Some(now.saturating_add(60000));
            due = true
        }
        due && !ready && !in_flight && !stopped
    }
}
#[cfg(test)]
mod schedule_tests {
    use super::*;
    #[test]
    fn setup_and_sleep_do_not_burst_and_missing_recovery_is_paced() {
        let mut checks = PreparationChecks::new(900000, 875000);
        assert!(!checks.due(875000, false, false, false));
        assert!(checks.due(890000, false, false, false));
        assert!(!checks.due(890001, false, false, false));
        assert!(checks.due(900000, false, false, false));
        assert!(!checks.due(930000, false, false, false));
        assert!(checks.due(960000, false, false, false));
        let mut asleep = PreparationChecks::new(1800000, 0);
        asleep.skip_through(1795000);
        assert!(!asleep.due(1795000, false, false, false));
        assert!(!asleep.due(1800000, false, false, true));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn utc_and_pair_selection_are_exact() {
        assert_eq!(utc_ms("1970-01-01T00:00:00Z").unwrap(), 0);
        assert_eq!(utc_ms("2000-02-29T00:00:00.123Z").unwrap(), 951782400123);
        assert!(utc_ms("2026-02-29T00:00:00Z").is_err());
        let m = |id: &str, open| MarketDescriptor {
            series_id: "KXBTC15M".into(),
            market_id: id.into(),
            contract_id: id.into(),
            open_ms: open,
            close_ms: open + 900000,
        };
        assert_eq!(
            select_pair(vec![m("n", 900000), m("m", 0)], 1).unwrap()[0].market_id,
            "m"
        );
        assert!(select_pair(vec![m("m", 0), m("ambiguous", 1), m("n", 900000)], 2).is_err());
    }
    #[test]
    fn initialized_future_can_prepare_but_elapsed_initialized_market_is_unavailable() {
        let bytes=br#"{"market":{"ticker":"KXBTC15M-M","event_ticker":"KXBTC15M-E","open_time":"1970-01-01T00:15:00Z","close_time":"1970-01-01T00:30:00Z","status":"initialized"}}"#;
        let route = Route::Market("KXBTC15M-M".into());
        assert!(
            parse(
                &route,
                bytes,
                0,
                1,
                &super::super::memory::Memory::default()
            )
            .is_ok()
        );
        assert!(matches!(
            parse(
                &route,
                bytes,
                0,
                900000,
                &super::super::memory::Memory::default()
            ),
            Err(Failure::Unavailable)
        ));
    }
    #[test]
    fn parsed_metadata_reservation_survives_handoff_and_releases_on_caller_cancel() {
        let memory = super::super::memory::Memory::default();
        let parsed = parse(
            &Route::Series,
            br#"{"series":{"ticker":"KXBTC15M"}}"#,
            0,
            0,
            &memory,
        )
        .unwrap();
        let (reply, receive) = tokio::sync::oneshot::channel();
        assert!(reply.send(parsed).is_ok());
        assert_eq!(memory.used(), 4 * super::super::MAX_BYTES);
        drop(receive);
        assert_eq!(memory.used(), 0);
        let held = memory
            .reserve(32 * super::super::MAX_BYTES - 4 * super::super::MAX_BYTES + 1)
            .unwrap();
        assert!(matches!(
            parse(
                &Route::Series,
                br#"{"series":{"ticker":"KXBTC15M"}}"#,
                0,
                0,
                &memory
            ),
            Err(Failure::Capacity)
        ));
        drop(held);
    }
    #[test]
    fn opaque_cursor_decodes_escapes_and_is_encoded_as_one_query_value() {
        let memory = super::super::memory::Memory::default();
        let result = parse(
            &Route::Markets {
                min_close_ts: 1,
                max_close_ts: 2,
                cursor: None,
            },
            br#"{"markets":[],"cursor":"opaque \/\u00e9"}"#,
            0,
            1,
            &memory,
        )
        .unwrap();
        let Metadata::Page { cursor, .. } = result.data else {
            panic!("page")
        };
        let path = Route::Markets {
            min_close_ts: 1,
            max_close_ts: 2,
            cursor,
        }
        .path()
        .unwrap();
        assert!(path.ends_with("cursor=opaque%20%2F%C3%A9"));
    }
}
