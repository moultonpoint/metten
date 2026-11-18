//! Coinbase Exchange BTC-USD matches, not Advanced Trade or a substitute index.
use super::{Failure, wire::*};
use metten_engine::sources::Stream;
use serde::Deserialize;
use serde_json::value::RawValue;
#[derive(Deserialize)]
struct Message<'a> {
    #[serde(rename = "type")]
    kind: &'a str,
    product_id: Option<&'a str>,
    trade_id: Option<u64>,
    last_trade_id: Option<u64>,
    #[serde(borrow)]
    price: Option<&'a RawValue>,
    time: Option<&'a str>,
    #[serde(borrow)]
    size: Option<&'a RawValue>,
    side: Option<&'a str>,
    sequence: Option<u64>,
    #[serde(borrow)]
    channels: Option<&'a RawValue>,
}
#[derive(Default)]
pub struct Coinbase {
    trades: Trades,
}
impl Coinbase {
    pub fn reconnect(&mut self) {
        self.trades.reconnect()
    }
    pub fn decode(
        &mut self,
        bytes: &[u8],
        mut emit: impl FnMut(Event) -> Result<(), Failure>,
    ) -> Result<(), Failure> {
        let m: Message<'_> = decode(bytes)?;
        match m.kind {
            "subscriptions" => {
                #[derive(Deserialize)]
                struct Channel<'a> {
                    name: &'a str,
                    #[serde(borrow)]
                    product_ids: List<&'a str>,
                }
                let channels: List<Channel<'_>> =
                    decode(m.channels.ok_or(Failure::Protocol)?.get().as_bytes())?;
                if channels.0.len() != 2
                    || !["matches", "heartbeat"].iter().all(|name| {
                        channels
                            .0
                            .iter()
                            .any(|c| c.name == *name && c.product_ids.0 == ["BTC-USD"])
                    })
                {
                    return Err(Failure::Protocol);
                }
                emit(Event::Acknowledged(Stream::Coinbase, 0))
            }
            "error" => Err(Failure::Protocol),
            "heartbeat" => {
                if m.product_id != Some("BTC-USD") {
                    return Err(Failure::Protocol);
                }
                if self
                    .trades
                    .missed(m.last_trade_id.ok_or(Failure::Protocol)?)
                {
                    return Err(Failure::Transport);
                }
                emit(Event::Heartbeat)
            }
            "last_match" | "match" => {
                if m.product_id != Some("BTC-USD") {
                    return Err(Failure::Protocol);
                }
                if m.side.is_some_and(|s| !matches!(s, "buy" | "sell")) {
                    return Err(Failure::Protocol);
                }
                if let Some(q) = m.size {
                    quantity_text(q)?;
                }
                let id = m.trade_id.ok_or(Failure::Protocol)?;
                let value = usd(m.price.ok_or(Failure::Protocol)?)?;
                let time = m.time.ok_or(Failure::Protocol)?;
                identity(time)?;
                if let Some(initialize) = self.trades.accept(id, value, m.kind == "last_match")? {
                    emit(Event::Prices(Prices {
                        stream: Stream::Coinbase,
                        market: None,
                        readings: vec![Reading {
                            field: "last_trade",
                            side: None,
                            value: Some(value),
                        }],
                        identity: id.to_string(),
                        time: Some(time.into()),
                        initialize,
                        details: Some(Details::Trade {
                            sid: None,
                            sending_ts_ms: None,
                            quantity: m.size.map(quantity_text).transpose()?,
                            direction: m.side.map(str::to_owned),
                            role: "maker",
                            sequence: m.sequence,
                            block: None,
                        }),
                    }))?
                }
                Ok(())
            }
            _ => Err(Failure::Protocol),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn initialization_equal_price_and_missing_trade() {
        let mut p = Coinbase::default();
        let mut kinds = vec![];
        for (kind, id) in [
            ("last_match", 10),
            ("match", 11),
            ("match", 11),
            ("match", 12),
        ] {
            let b = format!(
                r#"{{"type":"{kind}","product_id":"BTC-USD","price":"68000.12","trade_id":{id},"time":"2026-11-16T00:00:00Z"}}"#
            );
            p.decode(b.as_bytes(), |e| {
                if let Event::Prices(p) = e {
                    kinds.push(p.initialize)
                }
                Ok(())
            })
            .unwrap();
        }
        assert_eq!(kinds, vec![true, false, false]);
        assert!(
            p.decode(
                br#"{"type":"heartbeat","product_id":"BTC-USD","last_trade_id":13}"#,
                |_| Ok(())
            )
            .is_err()
        );
    }
    #[test]
    fn malformed_wrong_product_conflict_and_wrong_ack_are_rejected() {
        for bytes in [br#"{"type":"match","product_id":"ETH-USD","trade_id":1,"price":"1","time":"t"}"#.as_slice(),br#"{"type":"match","product_id":"BTC-USD","trade_id":1,"price":"1.1234567","time":"t"}"#,br#"{"type":"subscriptions","channels":[{"name":"matches","product_ids":["ETH-USD"]}]}"#,br#"{"type":"match","type":"match"}"#] {
            assert!(Coinbase::default().decode(bytes,|_|Ok(())).is_err());
        }
        let mut p = Coinbase::default();
        for (price, ok) in [("1", true), ("2", false)] {
            let input = format!(
                r#"{{"type":"match","product_id":"BTC-USD","trade_id":1,"price":"{price}","time":"t"}}"#
            );
            assert_eq!(p.decode(input.as_bytes(), |_| Ok(())).is_ok(), ok);
        }
    }
}
