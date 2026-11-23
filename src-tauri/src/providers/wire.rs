//! Shared lexical bounds and exact prices; provider schemas remain in their modules.
use super::{Failure, MAX_BYTES};
use metten_engine::{
    sources::Stream,
    strategy::{Decimal, Side},
};
use serde::{
    Deserialize,
    de::{self, SeqAccess, Visitor},
};
use serde_json::value::RawValue;
use std::fmt;

pub fn decode<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, Failure> {
    bounds(bytes)?;
    serde_json::from_slice(bytes).map_err(|error| {
        if error.to_string().starts_with("capacity") {
            Failure::Capacity
        } else {
            Failure::Protocol
        }
    })
}
fn bounds(bytes: &[u8]) -> Result<(), Failure> {
    if bytes.len() > MAX_BYTES {
        return Err(Failure::Capacity);
    }
    let (mut depth, mut string, mut escape, mut length) = (0usize, false, false, 0usize);
    for &b in bytes {
        if string {
            if !escape && b == b'"' {
                string = false;
                continue;
            }
            length += 1;
            if length > 16384 {
                return Err(Failure::Capacity);
            }
            if escape {
                escape = false;
            } else if b == b'\\' {
                escape = true;
            }
        } else {
            match b {
                b'"' => {
                    string = true;
                    length = 0
                }
                b'{' | b'[' => {
                    depth += 1;
                    if depth > 48 {
                        return Err(Failure::Capacity);
                    }
                }
                b'}' | b']' => depth = depth.checked_sub(1).ok_or(Failure::Protocol)?,
                _ => (),
            }
        }
    }
    if string || depth != 0 {
        return Err(Failure::Protocol);
    }
    Ok(())
}
pub struct List<T>(pub Vec<T>);
impl<'de, T: Deserialize<'de>> Deserialize<'de> for List<T> {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Items<T>(std::marker::PhantomData<T>);
        impl<'de, T: Deserialize<'de>> Visitor<'de> for Items<T> {
            type Value = List<T>;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("bounded list")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<Self::Value, A::Error> {
                let mut items = Vec::new();
                while let Some(item) = a.next_element()? {
                    if items.len() == 8192 {
                        return Err(de::Error::custom("capacity"));
                    }
                    items.push(item);
                }
                Ok(List(items))
            }
        }
        d.deserialize_seq(Items(std::marker::PhantomData))
    }
}
pub fn number(raw: &RawValue) -> Result<Decimal, Failure> {
    let text = raw.get();
    let text = if text.starts_with('"') {
        serde_json::from_str::<&str>(text).map_err(|_| Failure::Protocol)?
    } else {
        text
    };
    if let Some(index) = text.find(['e', 'E']) {
        if raw.get().starts_with('"') || text.len() > 32 {
            return Err(Failure::Protocol);
        }
        let exponent: i32 = text[index + 1..].parse().map_err(|_| Failure::Protocol)?;
        if !(-18..=18).contains(&exponent) {
            return Err(Failure::Protocol);
        }
        let mantissa = &text[..index];
        let negative = mantissa.starts_with('-');
        let mantissa = mantissa.strip_prefix('-').unwrap_or(mantissa);
        let point = mantissa.find('.').unwrap_or(mantissa.len()) as i32 + exponent;
        let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
        // The strict decimal grammar refuses leading zeros, so 0.68e5 must become 68000.
        let whole = |part: &str| match part.trim_start_matches('0') {
            "" => "0".to_owned(),
            part => part.to_owned(),
        };
        let expanded = if point <= 0 {
            format!("0.{}{}", "0".repeat((-point) as usize), digits)
        } else if point as usize >= digits.len() {
            whole(&format!(
                "{}{}",
                digits,
                "0".repeat(point as usize - digits.len())
            ))
        } else {
            format!(
                "{}.{}",
                whole(&digits[..point as usize]),
                &digits[point as usize..]
            )
        };
        return Decimal::parse(&format!("{}{}", if negative { "-" } else { "" }, expanded))
            .map_err(|_| Failure::Protocol);
    }
    Decimal::parse(text).map_err(|_| Failure::Protocol)
}
pub fn scaled(value: i128) -> Result<Decimal, Failure> {
    let negative = value < 0;
    let n = value.unsigned_abs();
    Decimal::parse(&format!(
        "{}{}.{:06}",
        if negative { "-" } else { "" },
        n / 1_000_000,
        n % 1_000_000
    ))
    .map_err(|_| Failure::Protocol)
}
pub fn usd(raw: &RawValue) -> Result<Decimal, Failure> {
    let n = number(raw)?;
    if n.millionths() <= 0 {
        return Err(Failure::Protocol);
    }
    Ok(n)
}
pub fn cents(raw: &RawValue) -> Result<Decimal, Failure> {
    let text = if raw.get().starts_with('"') {
        serde_json::from_str::<&str>(raw.get()).map_err(|_| Failure::Protocol)?
    } else {
        raw.get()
    };
    if text.len() > 32 || text.is_empty() {
        return Err(Failure::Protocol);
    }
    let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
    if whole.is_empty()
        || !whole
            .bytes()
            .chain(fraction.bytes())
            .all(|b| b.is_ascii_digit())
        || fraction.len() > 8
    {
        return Err(Failure::Protocol);
    }
    let whole: u128 = whole.parse().map_err(|_| Failure::Protocol)?;
    let fraction: u128 = if fraction.is_empty() {
        0
    } else {
        fraction.parse().map_err(|_| Failure::Protocol)?
    };
    let n = whole
        .checked_mul(100_000_000)
        .and_then(|n| {
            n.checked_add(
                fraction
                    * 10_u128.pow((8 - text.split_once('.').map_or(0, |(_, f)| f.len())) as u32),
            )
        })
        .ok_or(Failure::Protocol)?;
    if n > 100_000_000 {
        return Err(Failure::Protocol);
    }
    scaled(n as i128)
}
pub fn identity(s: &str) -> Result<(), Failure> {
    if s.is_empty() || s.len() > 128 || s.chars().any(char::is_control) {
        Err(Failure::Protocol)
    } else {
        Ok(())
    }
}
#[derive(Debug)]
pub struct Reading {
    pub field: &'static str,
    pub side: Option<Side>,
    pub value: Option<Decimal>,
}
pub struct Prices {
    pub stream: Stream,
    pub market: Option<String>,
    pub readings: Vec<Reading>,
    pub identity: String,
    pub time: Option<String>,
    pub initialize: bool,
    pub details: Option<Details>,
}
#[allow(clippy::large_enum_variant)] // One synchronous stack event; boxing would add an allocation per price.
pub enum Event {
    Prices(Prices),
    Acknowledged(Stream, u32),
    ControlAcknowledged(u32),
    ControlRejected(u32, u32),
    SubscriptionEnded(Stream, u32),
    Unsubscribed(Stream, u32),
    MetadataChanged,
    Absent(Stream),
    Gap(Stream, u32),
    Heartbeat,
}
#[derive(Default, Clone)]
pub struct Trades {
    last: Option<(u64, Decimal)>,
    initialized: bool,
}
impl Trades {
    pub fn reconnect(&mut self) {
        self.initialized = false;
    }
    pub fn accept(
        &mut self,
        id: u64,
        value: Decimal,
        initial: bool,
    ) -> Result<Option<bool>, Failure> {
        if let Some((last, old)) = self.last {
            if id < last {
                return Ok(None);
            }
            if id == last {
                if old != value {
                    return Err(Failure::Protocol);
                }
                if self.initialized {
                    return Ok(None);
                }
            }
        }
        let initialize = initial || !self.initialized;
        self.last = Some((id, value));
        self.initialized = true;
        Ok(Some(initialize))
    }
    pub fn missed(&self, id: u64) -> bool {
        self.last.is_some_and(|(last, _)| id > last)
    }
}

#[derive(Clone)]
pub enum Details {
    Book {
        sending_ts_ms: Option<u64>,
        clock: Option<u64>,
        sid: u64,
        sequence: u64,
        quantities: [Option<Decimal>; 4],
    },
    Trade {
        sid: Option<u64>,
        sending_ts_ms: Option<u64>,
        quantity: Option<String>,
        direction: Option<String>,
        role: &'static str,
        sequence: Option<u64>,
        block: Option<bool>,
    },
    Brti {
        sid: u64,
        sequence: u64,
        received_at: Option<u64>,
        sending_ts_ms: Option<u64>,
        trailing: Option<Average>,
        final_minute: Option<Average>,
    },
}
#[derive(Clone)]
pub struct Average {
    pub value: String,
    pub count: u64,
    pub start: Option<u64>,
    pub end_exclusive: Option<u64>,
}
pub fn quantity_text(raw: &RawValue) -> Result<String, Failure> {
    let text = if raw.get().starts_with('"') {
        serde_json::from_str::<&str>(raw.get()).map_err(|_| Failure::Protocol)?
    } else {
        raw.get()
    };
    identity(text)?;
    if !text.as_bytes()[0].is_ascii_digit() || serde_json::from_str::<&RawValue>(text).is_err() {
        return Err(Failure::Protocol);
    }
    Ok(text.to_owned())
}
pub fn average(raw: Option<&RawValue>) -> Result<Option<Average>, Failure> {
    #[derive(Deserialize)]
    struct Wire<'a> {
        #[serde(borrow)]
        value: &'a RawValue,
        window_size: u64,
        window_start_ts_ms: Option<u64>,
        window_end_ts_exclusive: Option<u64>,
    }
    raw.map(|raw| {
        let a: Wire<'_> = decode(raw.get().as_bytes())?;
        Ok(Average {
            value: quantity_text(a.value)?,
            count: a.window_size,
            start: a.window_start_ts_ms,
            end_exclusive: a.window_end_ts_exclusive,
        })
    })
    .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounds_and_exact_numbers() {
        assert!(decode::<&RawValue>(&vec![b' '; MAX_BYTES + 1]).is_err());
        assert!(
            decode::<&RawValue>(format!("{}0{}", "[".repeat(49), "]".repeat(49)).as_bytes())
                .is_err()
        );
        assert!(usd(decode::<&RawValue>(b"68000.1234567").unwrap()).is_err());
        assert_eq!(
            cents(decode::<&RawValue>(b"\"0.3600\"").unwrap())
                .unwrap()
                .millionths(),
            36_000_000
        );
    }
    #[test]
    fn precision_exponents_and_exact_collection_limits() {
        for (text, want) in [
            ("6.800012e4", 68000120000),
            ("1e-6", 1),
            ("1.234567e1", 12345670),
            ("0.68e5", 68000000000),
            ("0.5e1", 5000000),
        ] {
            assert_eq!(
                usd(decode::<&RawValue>(text.as_bytes()).unwrap())
                    .unwrap()
                    .millionths(),
                want
            );
        }
        assert!(usd(decode::<&RawValue>(b"0.1e-6").unwrap()).is_err());
        assert_eq!(
            cents(decode::<&RawValue>(br#""0.12345678""#).unwrap())
                .unwrap()
                .millionths(),
            12345678
        );
        assert!(cents(decode::<&RawValue>(br#""0.123456789""#).unwrap()).is_err());
        for count in [8192, 8193] {
            let input = format!("[{}]", vec!["0"; count].join(","));
            let result = decode::<List<u64>>(input.as_bytes());
            if count == 8192 {
                assert_eq!(result.unwrap().0.len(), count)
            } else {
                assert!(matches!(result, Err(Failure::Capacity)))
            }
        }
        let exact = format!("\"{}\"", "x".repeat(16384));
        assert!(decode::<&str>(exact.as_bytes()).is_ok());
        assert!(decode::<&str>(format!("\"{}\"", "x".repeat(16385)).as_bytes()).is_err());
    }
}
