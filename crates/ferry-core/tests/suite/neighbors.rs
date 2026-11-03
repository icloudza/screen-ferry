use std::time::Instant;

use ferry_core::{Neighbors, Place, Side, Transition};
use ferry_types::{Point, Rect};

const LOCAL: Rect = Rect::new(0, 0, 1000, 800);
const REMOTE: Rect = Rect::new(0, 0, 1920, 1080);

fn place(side: Side, rank: u32) -> Place {
    Place { side, rank }
}

/// A、B 都在右侧（A 在上），C 在左侧。缩放都是 100%，位移一比一。
fn three() -> Neighbors {
    let mut n = Neighbors::new(LOCAL);
    n.insert(1, place(Side::Right, 0), REMOTE, 100);
    n.insert(2, place(Side::Right, 1), REMOTE, 100);
    n.insert(3, place(Side::Left, 0), REMOTE, 100);
    n
}

fn enter(n: &mut Neighbors, position: Point, dx: i32) -> (u64, Point) {
    match n.local_motion(position, dx, 0, Instant::now()) {
        Some((id, Transition::Enter { at, .. })) => (id, at),
        other => panic!("应进入一台邻居，实际 {other:?}"),
    }
}

fn leave(n: &mut Neighbors, dx: i32) -> (u64, Point) {
    match n.remote_motion(dx, 0, true, Instant::now()) {
        Some((id, Transition::Leave { local_at, .. })) => (id, local_at),
        other => panic!("应回到本机，实际 {other:?}"),
    }
}

#[test]
fn neighbors_on_the_same_side_share_the_edge_in_order() {
    let mut n = three();
    assert_eq!(
        n.segment(1),
        Some((Side::Right, Rect::new(0, 0, 1000, 400)))
    );
    assert_eq!(
        n.segment(2),
        Some((Side::Right, Rect::new(0, 400, 1000, 400)))
    );
    assert_eq!(n.segment(3), Some((Side::Left, LOCAL)));

    // 上半段进 A：段内 1/4 处对应 A 整条边的 1/4。
    let (id, at) = enter(&mut n, Point::new(999, 100), 4);
    assert_eq!((id, at.y), (1, 270));
    assert_eq!(n.active(), Some(1));
    let (id, back) = leave(&mut n, -10);
    assert_eq!(id, 1);
    assert!(back.y < 400, "回到上半段：{back:?}");

    // 下半段进 B。
    let (id, at) = enter(&mut n, Point::new(999, 600), 4);
    assert_eq!((id, at.y), (2, 540));
    let (_, back) = leave(&mut n, -10);
    assert!((400..800).contains(&back.y), "回到下半段：{back:?}");

    // 左边整条边都是 C。
    let (id, at) = enter(&mut n, Point::new(0, 400), -4);
    assert_eq!((id, at.y), (3, 540));
}

#[test]
fn while_on_a_neighbor_other_edges_do_not_switch() {
    let mut n = three();
    enter(&mut n, Point::new(999, 100), 4);
    assert_eq!(
        n.local_motion(Point::new(0, 400), -4, 0, Instant::now()),
        None
    );
    assert_eq!(
        n.local_motion(Point::new(999, 600), 4, 0, Instant::now()),
        None
    );
    assert!(n.remote_position().is_some());
}

#[test]
fn a_disconnected_neighbor_gives_its_part_of_the_edge_back() {
    let mut n = three();
    enter(&mut n, Point::new(999, 100), 4);
    let back = n.remove(1).expect("光标在它上面，要放回本机");
    assert!(back.x < 1000 && back.y < 400, "{back:?}");
    assert_eq!(n.active(), None);

    // B 独占右边。
    assert_eq!(n.segment(2), Some((Side::Right, LOCAL)));
    let (id, _) = enter(&mut n, Point::new(999, 100), 4);
    assert_eq!(id, 2);
}

#[test]
fn moving_the_neighbor_under_the_cursor_to_another_side_returns_first() {
    let mut n = three();
    enter(&mut n, Point::new(999, 100), 4);
    let t = n.set_place(1, place(Side::Top, 0));
    assert!(matches!(t, Some(Transition::Leave { .. })), "{t:?}");
    assert_eq!(n.active(), None);
    assert_eq!(n.segment(2), Some((Side::Right, LOCAL)));
    assert_eq!(n.segment(1), Some((Side::Top, LOCAL)));

    // 只是在同一侧换了先后：光标还在原来那台上，回来时落在新的那一段。
    let (id, _) = enter(&mut n, Point::new(999, 100), 4);
    assert_eq!(id, 2);
    n.insert(4, place(Side::Right, 0), REMOTE, 100);
    assert_eq!(n.active(), Some(2));
    assert_eq!(
        n.segment(2),
        Some((Side::Right, Rect::new(0, 400, 1000, 400)))
    );
    let (_, back) = leave(&mut n, -10);
    assert!(back.y >= 400, "{back:?}");
}

#[test]
fn the_hotkey_and_errors_bring_the_cursor_back() {
    let mut n = three();
    enter(&mut n, Point::new(0, 400), -4);
    assert!(matches!(n.leave(), Some((3, Transition::Leave { .. }))));
    assert_eq!(n.leave(), None, "已经在本机");

    enter(&mut n, Point::new(999, 600), 4);
    let (id, at) = n.abort().expect("光标在 B 上");
    assert_eq!(id, 2);
    assert!(at.y >= 400);
    assert_eq!(n.active(), None);
}

#[test]
fn each_neighbor_has_its_own_delay() {
    use std::time::Duration;

    let mut n = three();
    n.set_delay(1, Duration::from_millis(500));
    let t0 = Instant::now();
    let at = |ms| t0 + Duration::from_millis(ms);
    // A（右侧上半段）要停 0.5 秒，B（下半段）立即切换。
    assert_eq!(n.local_motion(Point::new(999, 100), 4, 0, at(0)), None);
    assert!(matches!(
        n.local_motion(Point::new(999, 600), 4, 0, at(10)),
        Some((2, Transition::Enter { .. }))
    ));
}

#[test]
fn the_active_neighbors_delay_applies_when_returning_too() {
    use std::time::Duration;

    let mut n = three();
    n.set_delay(1, Duration::from_millis(500));
    let t0 = Instant::now();
    let at = |ms| t0 + Duration::from_millis(ms);
    let edge = Point::new(999, 100);
    assert_eq!(n.local_motion(edge, 1, 0, at(0)), None);
    assert!(matches!(
        n.local_motion(edge, 1, 0, at(500)),
        Some((1, Transition::Enter { .. }))
    ));
    assert!(matches!(
        n.remote_motion(-50, 0, true, at(1000)),
        Some((1, Transition::Move(_)))
    ));
    assert_eq!(n.remote_motion(-1, 0, true, at(1499)), None);
    assert_eq!(n.active(), Some(1));
    n.reset_wait();
    assert_eq!(n.remote_motion(-1, 0, true, at(1500)), None);
    assert!(matches!(
        n.remote_motion(-1, 0, true, at(2000)),
        Some((1, Transition::Leave { .. }))
    ));
    assert_eq!(n.active(), None);
    // B 没设延迟，进出仍然立即生效。
    enter(&mut n, Point::new(999, 600), 1);
    leave(&mut n, -50);
}

#[test]
fn sliding_along_the_edge_to_another_part_starts_the_wait_over() {
    use std::time::Duration;

    let mut n = three();
    n.set_delay(1, Duration::from_millis(200));
    n.set_delay(2, Duration::from_millis(200));
    let t0 = Instant::now();
    let at = |ms| t0 + Duration::from_millis(ms);
    assert_eq!(n.local_motion(Point::new(999, 100), 4, 0, at(0)), None);
    // 沿边滑到 B 那一段，再回到 A：A 要重新计时。
    assert_eq!(n.local_motion(Point::new(999, 600), 0, 9, at(100)), None);
    assert_eq!(n.local_motion(Point::new(999, 100), 4, 0, at(250)), None);
    assert!(matches!(
        n.local_motion(Point::new(999, 100), 4, 0, at(450)),
        Some((1, Transition::Enter { .. }))
    ));
}

#[test]
fn each_neighbor_has_its_own_pointer_speed() {
    let mut n = three();
    n.set_speed(1, 2, 1);
    let (_, at) = enter(&mut n, Point::new(999, 100), 4);
    assert_eq!(
        n.remote_motion(10, 0, true, Instant::now()),
        Some((1, Transition::Move(Point::new(at.x + 20, at.y))))
    );
    leave(&mut n, -5000);
    let (_, at) = enter(&mut n, Point::new(999, 600), 4);
    assert_eq!(
        n.remote_motion(10, 0, true, Instant::now()),
        Some((2, Transition::Move(Point::new(at.x + 10, at.y)))),
        "B 没改速度"
    );
}
