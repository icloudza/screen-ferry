//! 特权输入连接的严格序号和有限租约。期限只在完整且有效的请求到达时续期。

use std::time::{Duration, Instant};

pub const INPUT_LEASE: Duration = Duration::from_millis(1500);
pub const MAX_INPUT_REQUEST_MS: u64 = 500;

pub struct InputLease {
    next: Option<u64>,
    deadline: Instant,
}

impl InputLease {
    pub fn new(now: Instant) -> Self {
        Self {
            next: Some(1),
            deadline: now + INPUT_LEASE,
        }
    }

    pub fn expired(&self, now: Instant) -> bool {
        now >= self.deadline
    }

    /// 两种时钟分别是 Windows 的跨进程 tick 和本进程 Instant；不受系统时间调整影响。
    pub fn accept(&mut self, sequence: u64, expires_ms: u64, ticks_ms: u64, now: Instant) -> bool {
        if self.expired(now)
            || self.next != Some(sequence)
            || expires_ms <= ticks_ms
            || expires_ms - ticks_ms > MAX_INPUT_REQUEST_MS
        {
            return false;
        }
        self.next = sequence.checked_add(1);
        self.deadline = now + INPUT_LEASE;
        true
    }
}
