//! Conservative local traffic bounds, not an account-tier guarantee.
use super::Failure;
use tokio::time::{Duration, Instant};
#[derive(Clone, Copy)]
pub enum Traffic {
    Read,
    Handshake,
    Control,
    KeepAlive,
}
impl Traffic {
    fn index(self) -> usize {
        match self {
            Self::Read => 0,
            Self::Handshake => 1,
            Self::Control => 2,
            Self::KeepAlive => 3,
        }
    }
    fn cost(self) -> u32 {
        if matches!(self, Self::Read) { 10 } else { 1 }
    }
}
struct Bucket {
    tokens: u32,
    reserved: u32,
    capacity: u32,
    every: Duration,
    updated: Instant,
}
impl Bucket {
    fn refill(&mut self, now: Instant) {
        let periods =
            now.saturating_duration_since(self.updated).as_nanos() / self.every.as_nanos();
        if periods > 0 {
            self.tokens = self.capacity.min(
                self.tokens
                    .saturating_add(periods.min(u32::MAX as u128) as u32),
            );
            self.updated += self.every.mul_f64(periods as f64);
        }
    }
    fn reserve(&mut self, cost: u32, now: Instant) -> bool {
        self.refill(now);
        if self.tokens.saturating_sub(self.reserved) < cost {
            return false;
        }
        self.reserved += cost;
        true
    }
    fn start(&mut self, cost: u32, now: Instant) {
        self.refill(now);
        self.reserved -= cost;
        self.tokens -= cost;
    }
}
pub struct Budget {
    read: Bucket,
    control: Bucket,
    pub attempts: [u64; 3],
    pub pongs: u64,
    pub data_messages: u64,
}
impl Budget {
    pub fn new() -> Self {
        let now = Instant::now();
        let bucket = |capacity, every| Bucket {
            tokens: capacity,
            reserved: 0,
            capacity,
            every,
            updated: now,
        };
        Self {
            read: bucket(50, Duration::from_millis(100)),
            control: bucket(1, Duration::from_secs(1)),
            attempts: [0; 3],
            pongs: 0,
            data_messages: 0,
        }
    }
    fn bucket(&mut self, traffic: Traffic) -> Option<&mut Bucket> {
        match traffic {
            Traffic::Read => Some(&mut self.read),
            Traffic::Control => Some(&mut self.control),
            Traffic::Handshake | Traffic::KeepAlive => None,
        }
    }

    fn reserve(&mut self, traffic: Traffic, deadline: Instant) -> Result<(), Failure> {
        let now = Instant::now();
        if now >= deadline {
            return Err(Failure::Deadline);
        }
        let bucket = self.bucket(traffic);
        let cost = traffic.cost();
        if bucket.is_some_and(|bucket| !bucket.reserve(cost, now)) {
            return Err(Failure::Budget);
        }
        Ok(())
    }
}

use std::sync::{Arc, Mutex};
pub struct Permit {
    budget: Arc<Mutex<Budget>>,
    traffic: Traffic,
    started: bool,
    deadline: Instant,
}
impl Permit {
    pub fn reserve(
        budget: Arc<Mutex<Budget>>,
        traffic: Traffic,
        deadline: Instant,
    ) -> Result<Self, Failure> {
        budget
            .lock()
            .map_err(|_| Failure::Unavailable)?
            .reserve(traffic, deadline)?;
        Ok(Self {
            budget,
            traffic,
            started: false,
            deadline,
        })
    }
    pub fn start(&mut self) -> Result<(), Failure> {
        if self.started {
            return Err(Failure::InvalidRequest);
        }
        if Instant::now() >= self.deadline {
            return Err(Failure::Deadline);
        }
        let mut budget = self.budget.lock().map_err(|_| Failure::Unavailable)?;
        if matches!(self.traffic, Traffic::KeepAlive) {
            budget.pongs = budget.pongs.checked_add(1).ok_or(Failure::Capacity)?;
        } else {
            let index = self.traffic.index();
            budget.attempts[index] = budget.attempts[index]
                .checked_add(1)
                .ok_or(Failure::Capacity)?;
        }
        if let Some(bucket) = budget.bucket(self.traffic) {
            bucket.start(self.traffic.cost(), Instant::now());
        }
        self.started = true;
        Ok(())
    }
}
impl Drop for Permit {
    fn drop(&mut self) {
        if self.started {
            return;
        }
        if let Ok(mut budget) = self.budget.lock() {
            let bucket = budget.bucket(self.traffic);
            let cost = self.traffic.cost();
            if let Some(bucket) = bucket {
                bucket.reserved -= cost;
            }
        }
    }
}
#[cfg(test)]
impl Budget {
    pub fn attempt(&mut self, traffic: Traffic, deadline: Instant) -> Result<(), Failure> {
        self.reserve(traffic, deadline)?;
        let index = traffic.index();
        if let Some(bucket) = self.bucket(traffic) {
            bucket.start(traffic.cost(), Instant::now());
        }
        self.attempts[index] += 1;
        Ok(())
    }
}

pub struct Retry {
    failures: u32,
    seed: u64,
    next: Option<Instant>,
    usable_since: Option<Instant>,
    pace: Duration,
}
impl Retry {
    pub fn new(seed: u64) -> Self {
        Self {
            failures: 0,
            seed,
            next: None,
            usable_since: None,
            pace: Duration::ZERO,
        }
    }
    pub fn claim(&mut self, now: Instant) -> Option<Duration> {
        if let Some(next) = self.next
            && next > now
        {
            return Some(next - now);
        }
        if self.failures > 0 {
            self.next = Some(now + self.pace);
        }
        None
    }
    pub fn slow(&self) -> bool {
        self.failures >= 5
    }
    pub fn usable(&mut self, now: Instant) {
        let since = *self.usable_since.get_or_insert(now);
        if now.saturating_duration_since(since) >= Duration::from_secs(30) {
            self.failures = 0;
            self.next = None;
        }
    }
    pub fn delay(&mut self, failure: Failure) -> Option<Duration> {
        self.usable_since = None;
        if failure != Failure::Transport {
            return None;
        }
        self.failures = self.failures.saturating_add(1);
        self.seed = self.seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let seconds = if self.slow() {
            60
        } else {
            (5u64 << (self.failures - 1)).min(60)
        };
        let delay = (Duration::from_secs(seconds) + Duration::from_millis(1 + self.seed % 250))
            .min(Duration::from_secs(60));
        self.pace = delay;
        self.next = Some(Instant::now() + delay);
        Some(delay)
    }
}
