use ferry_core::{INPUT_LEASE, InputLease, MAX_INPUT_REQUEST_MS};
use std::time::{Duration, Instant};

#[test]
fn only_contiguous_unexpired_requests_renew_the_lease() {
    let now = Instant::now();
    let mut lease = InputLease::new(now);
    assert!(!lease.accept(0, 400, 0, now));
    assert!(lease.accept(1, 400, 0, now));
    assert!(!lease.accept(1, 401, 1, now));
    assert!(!lease.accept(3, 401, 1, now));
    assert!(lease.accept(2, 401, 1, now));
    assert!(!lease.expired(now + INPUT_LEASE - Duration::from_millis(1)));
    assert!(lease.expired(now + INPUT_LEASE));
}

#[test]
fn expired_or_far_future_deadlines_do_not_renew_authority() {
    let now = Instant::now();
    let mut lease = InputLease::new(now);
    assert!(!lease.accept(1, 100, 100, now));
    assert!(!lease.accept(1, 99, 100, now));
    assert!(!lease.accept(1, 100 + MAX_INPUT_REQUEST_MS + 1, 100, now));
    assert!(lease.expired(now + INPUT_LEASE));
    assert!(!lease.accept(1, 2000, 1900, now + INPUT_LEASE));
}

#[test]
fn invalid_traffic_cannot_keep_a_connection_alive() {
    let now = Instant::now();
    let mut lease = InputLease::new(now);
    assert!(lease.accept(1, 400, 0, now));
    let later = now + Duration::from_millis(1400);
    assert!(!lease.accept(1, 1800, 1400, later), "重放不续期");
    assert!(lease.expired(now + INPUT_LEASE));
}

#[test]
fn valid_keepalives_extend_the_lease_but_cannot_resurrect_it() {
    let now = Instant::now();
    let mut lease = InputLease::new(now);
    assert!(lease.accept(1, 1400, 1000, now + Duration::from_millis(1000)));
    assert!(!lease.expired(now + Duration::from_millis(2000)));
    assert!(lease.expired(now + Duration::from_millis(2500)));
    assert!(!lease.accept(2, 2900, 2500, now + Duration::from_millis(2500)));
}
