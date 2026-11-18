//! Kraken v2 lexical prices and ordered individual trades.
use super::{Failure, wire::*};
use metten_engine::sources::Stream;
use serde::Deserialize;
use serde_json::value::RawValue;
#[derive(Deserialize)]
struct Message<'a> {
    channel: Option<&'a str>,
    #[serde(rename = "type")]
    kind: Option<&'a str>,
    method: Option<&'a str>,
    success: Option<bool>,
    #[serde(borrow)]
    result: Option<&'a RawValue>,
    #[serde(borrow)]
    data: Option<&'a RawValue>,
}
#[derive(Deserialize)]
struct Trade<'a> {
    symbol: &'a str,
    #[serde(borrow)]
    price: &'a RawValue,
    trade_id: u64,
    timestamp: &'a str,
    #[serde(borrow)]
    qty: Option<&'a RawValue>,
    side: Option<&'a str>,
}
#[derive(Default)]
pub struct Kraken {
    trades: Trades,
}
impl Kraken {
    pub fn reconnect(&mut self) {
        self.trades.reconnect()
    }
    pub fn decode(
        &mut self,
        bytes: &[u8],
        mut emit: impl FnMut(Event) -> Result<(), Failure>,
    ) -> Result<(), Failure> {
        let m: Message<'_> = decode(bytes)?;
        if m.method == Some("subscribe") {
            return if m.success == Some(true) {
                #[derive(Deserialize)]
                struct Ack<'a> {
                    channel: &'a str,
                    symbol: &'a str,
                    snapshot: bool,
                }
                let result: Ack<'_> = decode(m.result.ok_or(Failure::Protocol)?.get().as_bytes())?;
                if result.channel != "trade" || result.symbol != "BTC/USD" || !result.snapshot {
                    return Err(Failure::Protocol);
                }
                emit(Event::Acknowledged(Stream::Kraken, 0))
            } else {
                Err(Failure::Protocol)
            };
        }
        match m.channel {
            Some("heartbeat") => emit(Event::Heartbeat),
            Some("trade") => {
                let initial = match m.kind {
                    Some("snapshot") => true,
                    Some("update") => false,
                    _ => return Err(Failure::Protocol),
                };
                let trades: List<Trade<'_>> =
                    decode(m.data.ok_or(Failure::Protocol)?.get().as_bytes())?;
                // Validate every member before exposing any part of a malformed provider batch.
                for t in &trades.0 {
                    if t.symbol != "BTC/USD" {
                        return Err(Failure::Protocol);
                    }
                    usd(t.price)?;
                    identity(t.timestamp)?;
                    if t.side.is_some_and(|s| !matches!(s, "buy" | "sell")) {
                        return Err(Failure::Protocol);
                    }
                    if let Some(q) = t.qty {
                        quantity_text(q)?;
                    }
                }
                let mut check = self.trades.clone();
                for t in &trades.0 {
                    check.accept(t.trade_id, usd(t.price)?, initial)?;
                }
                let selected: Vec<_> = if initial {
                    trades
                        .0
                        .iter()
                        .max_by_key(|t| t.trade_id)
                        .into_iter()
                        .collect()
                } else {
                    trades.0.iter().collect()
                };
                for t in selected {
                    let value = usd(t.price)?;
                    if let Some(initialize) = self.trades.accept(t.trade_id, value, initial)? {
                        emit(Event::Prices(Prices {
                            stream: Stream::Kraken,
                            market: None,
                            readings: vec![Reading {
                                field: "last_trade",
                                side: None,
                                value: Some(value),
                            }],
                            identity: t.trade_id.to_string(),
                            time: Some(t.timestamp.into()),
                            initialize,
                            details: Some(Details::Trade {
                                sid: None,
                                sending_ts_ms: None,
                                quantity: t.qty.map(quantity_text).transpose()?,
                                direction: t.side.map(str::to_owned),
                                role: "taker",
                                sequence: None,
                                block: None,
                            }),
                        }))?
                    }
                }
                Ok(())
            }
            Some("status") => emit(Event::Heartbeat),
            _ => Err(Failure::Protocol),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snapshot_then_ordered_same_price_events() {
        let mut p = Kraken::default();
        let mut seen = vec![];
        for (kind, ids) in [("snapshot", vec![10, 11]), ("update", vec![12, 13])] {
            let rows:Vec<_>=ids.iter().map(|id|format!(r#"{{"symbol":"BTC/USD","price":68000.123456,"trade_id":{id},"timestamp":"2026-11-16T00:00:00Z"}}"#)).collect();
            let b = format!(
                r#"{{"channel":"trade","type":"{kind}","data":[{}]}}"#,
                rows.join(",")
            );
            p.decode(b.as_bytes(), |e| {
                if let Event::Prices(p) = e {
                    seen.push((
                        p.identity,
                        p.initialize,
                        p.readings[0].value.unwrap().millionths(),
                    ))
                }
                Ok(())
            })
            .unwrap();
        }
        assert_eq!(
            seen,
            vec![
                ("11".into(), true, 68000123456),
                ("12".into(), false, 68000123456),
                ("13".into(), false, 68000123456)
            ]
        );
    }
    #[test]
    fn descending_snapshot_initializes_latest_and_invalid_batch_emits_nothing() {
        let mut p = Kraken::default();
        let mut ids = Vec::new();
        p.decode(br#"{"channel":"trade","type":"snapshot","data":[{"symbol":"BTC/USD","price":1,"trade_id":13,"timestamp":"t"},{"symbol":"BTC/USD","price":1,"trade_id":12,"timestamp":"t"}]}"#,|e|{if let Event::Prices(p)=e {ids.push(p.identity)}Ok(())}).unwrap();
        assert_eq!(ids, ["13"]);
        let bad=br#"{"channel":"trade","type":"update","data":[{"symbol":"BTC/USD","price":1,"trade_id":14,"timestamp":"t"},{"symbol":"BTC/USD","price":1.1234567,"trade_id":15,"timestamp":"t"}]}"#;
        let mut emitted = 0;
        assert!(
            p.decode(bad, |_| {
                emitted += 1;
                Ok(())
            })
            .is_err()
        );
        assert_eq!(emitted, 0);
        assert!(p.decode(br#"{"method":"subscribe","success":true,"result":{"channel":"trade","symbol":"ETH/USD","snapshot":true}}"#,|_|Ok(())).is_err());
    }
}
