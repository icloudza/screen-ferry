use std::time::Instant;

use ferry_core::{EdgeSwitch, Side, Transition};
use ferry_types::{Point, Rect};

/// 本机 1000×800，远端 1920×1080；远端缩放 100%，位移一比一。
fn switch(side: Side) -> EdgeSwitch {
    EdgeSwitch::new(
        Rect::new(0, 0, 1000, 800),
        Rect::new(0, 0, 1920, 1080),
        side,
        100,
    )
}

fn enter(s: &mut EdgeSwitch, position: Point, dx: i32, dy: i32) -> Point {
    match s.local_motion(position, dx, dy, Instant::now()) {
        Some(Transition::Enter { at, .. }) => at,
        other => panic!("应进入远端，实际 {other:?}"),
    }
}

#[test]
fn enters_only_when_pushing_past_the_edge() {
    let mut s = switch(Side::Right);
    assert_eq!(
        s.local_motion(Point::new(998, 400), 5, 0, Instant::now()),
        None,
        "还没到边缘"
    );
    assert_eq!(
        s.local_motion(Point::new(999, 400), 0, 3, Instant::now()),
        None,
        "在边缘但没有向外推"
    );
    assert_eq!(
        s.local_motion(Point::new(999, 400), -2, 0, Instant::now()),
        None,
        "在边缘但向里走"
    );
    assert_eq!(
        s.local_motion(Point::new(999, 400), 4, 0, Instant::now()),
        Some(Transition::Enter {
            seq: 1,
            at: Point::new(3, 540)
        }),
        "越过边缘 4，带到远端是第 4 个单位；纵坐标按比例换算：400/800 → 540/1080",
    );
    assert!(s.is_remote());
    assert_eq!(
        s.local_motion(Point::new(999, 400), 4, 0, Instant::now()),
        None,
        "已在远端，本机移动不再处理"
    );
}

#[test]
fn enters_on_every_side() {
    let cases = [
        (Side::Left, Point::new(0, 0), (-1, 0), Point::new(1918, 0)),
        (
            Side::Right,
            Point::new(999, 799),
            (1, 0),
            Point::new(1, 1078),
        ),
        (
            Side::Top,
            Point::new(500, 0),
            (0, -1),
            Point::new(960, 1078),
        ),
        (
            Side::Bottom,
            Point::new(500, 799),
            (0, 1),
            Point::new(960, 1),
        ),
    ];
    for (side, position, (dx, dy), expected) in cases {
        assert_eq!(
            enter(&mut switch(side), position, dx, dy),
            expected,
            "{side:?}：落在离远端边缘一个单位的地方"
        );
    }
}

#[test]
fn moves_inside_remote_and_stops_at_its_far_edges() {
    let mut s = switch(Side::Right);
    enter(&mut s, Point::new(999, 400), 1, 0);

    assert_eq!(
        s.remote_motion(30, -40, true, Instant::now()),
        Some(Transition::Move(Point::new(31, 500)))
    );
    assert_eq!(
        s.remote_motion(5000, 0, true, Instant::now()),
        Some(Transition::Move(Point::new(1919, 500)))
    );
    assert_eq!(
        s.remote_motion(10, 0, true, Instant::now()),
        None,
        "已顶到远端右边，没有变化"
    );
    assert_eq!(
        s.remote_motion(0, -9000, true, Instant::now()),
        Some(Transition::Move(Point::new(1919, 0)))
    );
    assert_eq!(s.remote_position(), Some(Point::new(1919, 0)));
}

#[test]
fn leaves_through_the_edge_facing_local() {
    let mut s = switch(Side::Right);
    enter(&mut s, Point::new(999, 400), 1, 0);
    s.remote_motion(10, 0, true, Instant::now());

    assert_eq!(
        s.remote_motion(-12, 0, true, Instant::now()),
        Some(Transition::Leave {
            seq: 1,
            local_at: Point::new(998, 400)
        }),
        "落在离本机边缘一个单位的地方；纵坐标换算回本机：540/1080 → 400/800",
    );
    assert!(!s.is_remote());

    let at = enter(&mut s, Point::new(999, 0), 1, 0);
    assert_eq!(at, Point::new(1, 0));
    assert!(matches!(
        s.remote_motion(-2, 0, true, Instant::now()),
        Some(Transition::Leave { seq: 2, .. })
    ));
}

#[test]
fn holding_a_button_keeps_the_cursor_on_remote() {
    let mut s = switch(Side::Right);
    enter(&mut s, Point::new(999, 400), 1, 0);
    assert_eq!(
        s.remote_motion(-50, 0, false, Instant::now()),
        Some(Transition::Move(Point::new(0, 540))),
        "顶到边缘为止"
    );
    assert_eq!(
        s.remote_motion(-50, 0, false, Instant::now()),
        None,
        "停在边缘，位置不变"
    );
    assert!(s.is_remote());
    assert!(matches!(
        s.remote_motion(-1, 0, true, Instant::now()),
        Some(Transition::Leave { .. })
    ));
}

#[test]
fn scales_motion_and_carries_the_remainder() {
    let mut s = EdgeSwitch::new(
        Rect::new(0, 0, 1000, 800),
        Rect::new(0, 0, 3000, 2000),
        Side::Right,
        150,
    );
    enter(&mut s, Point::new(999, 0), 1, 0);

    assert_eq!(
        s.remote_motion(10, 0, true, Instant::now()),
        Some(Transition::Move(Point::new(16, 0)))
    );
    assert_eq!(
        s.remote_motion(1, 0, true, Instant::now()),
        Some(Transition::Move(Point::new(17, 0))),
        "1.5 → 1，余 0.5"
    );
    assert_eq!(
        s.remote_motion(1, 0, true, Instant::now()),
        Some(Transition::Move(Point::new(19, 0))),
        "1.5 + 0.5 → 2"
    );
}

#[test]
fn abort_returns_control_to_local() {
    let mut s = switch(Side::Right);
    assert_eq!(s.abort(), None, "本来就在本机");

    enter(&mut s, Point::new(999, 400), 1, 0);
    assert_eq!(s.abort(), Some(Point::new(998, 400)));
    assert!(!s.is_remote());
    assert_eq!(s.remote_motion(5, 5, true, Instant::now()), None);
}

#[test]
fn remote_layout_change_clamps_the_cursor() {
    let mut s = switch(Side::Right);
    enter(&mut s, Point::new(999, 799), 1, 0);
    s.remote_motion(1900, 0, true, Instant::now());

    s.set_remote(Rect::new(0, 0, 1280, 720), 100);
    assert_eq!(s.remote_position(), Some(Point::new(1279, 719)));
}

#[test]
fn never_enters_an_empty_remote() {
    let mut s = EdgeSwitch::new(
        Rect::new(0, 0, 1000, 800),
        Rect::new(0, 0, 0, 0),
        Side::Right,
        100,
    );
    assert_eq!(
        s.local_motion(Point::new(999, 0), 1, 0, Instant::now()),
        None
    );
}

#[test]
fn leave_on_demand_reports_the_same_seq() {
    let mut s = switch(Side::Right);
    assert_eq!(s.leave(), None);

    enter(&mut s, Point::new(999, 400), 1, 0);
    assert_eq!(
        s.leave(),
        Some(Transition::Leave {
            seq: 1,
            local_at: Point::new(998, 400)
        })
    );
    assert!(!s.is_remote());
}

#[test]
fn changing_side_while_local_takes_effect_immediately() {
    let mut s = switch(Side::Right);
    assert_eq!(s.set_side(Side::Left), None);
    assert_eq!(s.side(), Side::Left);
    // 右边缘不再触发，左边缘触发。
    assert_eq!(
        s.local_motion(Point::new(999, 400), 5, 0, Instant::now()),
        None
    );
    enter(&mut s, Point::new(0, 400), -5, 0);
}

#[test]
fn changing_side_while_remote_returns_to_local_first() {
    let mut s = switch(Side::Right);
    enter(&mut s, Point::new(999, 400), 5, 0);
    match s.set_side(Side::Top) {
        Some(Transition::Leave { local_at, .. }) => assert_eq!(local_at.x, 998),
        other => panic!("应先回到本机，实际 {other:?}"),
    }
    assert!(!s.is_remote());
    assert_eq!(s.set_side(Side::Top), None, "同一侧不算变化");
}

#[test]
fn a_one_unit_wobble_at_the_edge_does_not_bounce_back() {
    let mut s = switch(Side::Right);
    enter(&mut s, Point::new(999, 400), 1, 0);

    assert_eq!(
        s.remote_motion(-1, 0, true, Instant::now()),
        Some(Transition::Move(Point::new(0, 540))),
        "刚进来往回抖一下：仍在远端"
    );
    let Some(Transition::Leave { local_at, .. }) = s.remote_motion(-1, 0, true, Instant::now())
    else {
        panic!("再往回走就回到本机");
    };
    assert_eq!(local_at, Point::new(998, 400));

    assert_eq!(
        s.local_motion(local_at, 1, 0, Instant::now()),
        None,
        "刚回来往外抖一下：还没到边缘"
    );
    enter(&mut s, Point::new(999, 400), 1, 0);
}

#[test]
fn a_fast_crossing_carries_the_overshoot_to_the_other_side() {
    let mut s = switch(Side::Right);
    assert_eq!(
        enter(&mut s, Point::new(999, 400), 30, 0),
        Point::new(29, 540),
        "越过边缘 30，到远端是第 30 个单位"
    );
    assert_eq!(
        s.remote_motion(-50, 0, true, Instant::now()),
        Some(Transition::Leave {
            seq: 1,
            local_at: Point::new(979, 400)
        }),
        "29 - 50：推出边缘 21，回到本机是第 21 个单位"
    );

    let mut s = switch(Side::Left);
    assert_eq!(enter(&mut s, Point::new(0, 0), -5000, 0), Point::new(0, 0));
    assert_eq!(
        s.remote_motion(9000, 0, true, Instant::now()),
        Some(Transition::Leave {
            seq: 1,
            local_at: Point::new(999, 0)
        }),
        "越过的距离再长也只落在屏幕范围内"
    );
}

#[test]
fn the_overshoot_is_scaled_between_the_two_sides() {
    // 远端移动的单位是本机的一半。
    let mut s = EdgeSwitch::new(
        Rect::new(0, 0, 1000, 800),
        Rect::new(0, 0, 1920, 1080),
        Side::Right,
        50,
    );
    assert_eq!(
        enter(&mut s, Point::new(999, 400), 20, 0),
        Point::new(9, 540),
        "本机越过 20 → 远端 10"
    );
    assert_eq!(
        s.remote_motion(-30, 0, true, Instant::now()),
        Some(Transition::Leave {
            seq: 1,
            local_at: Point::new(988, 400)
        }),
        "9 - 15：远端推出 6 → 本机 12"
    );
}

#[test]
fn local_layout_change_moves_the_edges() {
    let mut s = switch(Side::Right);
    s.set_local(Rect::new(0, 0, 800, 600), 100);

    assert_eq!(
        s.local_motion(Point::new(798, 300), 1, 0, Instant::now()),
        None,
        "还没到新的边缘"
    );
    assert_eq!(
        enter(&mut s, Point::new(799, 300), 1, 0),
        Point::new(1, 540),
        "纵坐标按新的高度换算：300/600 → 540/1080"
    );
    assert_eq!(
        s.leave(),
        Some(Transition::Leave {
            seq: 1,
            local_at: Point::new(798, 300)
        }),
        "回来时落在新的范围内"
    );
}

// ── 光标速度与切换前停留（ADR 0009） ────────────────

#[test]
fn pointer_speed_is_exact_to_one_dpi() {
    let mut s = switch(Side::Right);
    s.set_speed(2000, 1600);
    enter(&mut s, Point::new(999, 400), 1, 0);
    let start = s.remote_position().unwrap();
    // 8 × 2000 / 1600 = 10。
    assert_eq!(
        s.remote_motion(8, 0, true, Instant::now()),
        Some(Transition::Move(Point::new(start.x + 10, start.y)))
    );
    // 每次 1 × 1.25：前三次各走 1，余下的累积起来，第四次多走 1。
    let mut x = start.x + 10;
    for expected in [1, 1, 1, 2] {
        x += expected;
        assert_eq!(
            s.remote_motion(1, 0, true, Instant::now()),
            Some(Transition::Move(Point::new(x, start.y)))
        );
    }
}

#[test]
fn a_slower_pointer_keeps_the_small_steps() {
    let mut s = switch(Side::Right);
    s.set_speed(800, 1600);
    enter(&mut s, Point::new(999, 400), 1, 0);
    let start = s.remote_position().unwrap();
    assert_eq!(
        s.remote_motion(1, 0, true, Instant::now()),
        None,
        "半个单位先攒着"
    );
    assert_eq!(
        s.remote_motion(1, 0, true, Instant::now()),
        Some(Transition::Move(Point::new(start.x + 1, start.y)))
    );
}

#[test]
fn with_a_delay_the_cursor_must_rest_on_the_edge_first() {
    use std::time::Duration;

    let mut s = switch(Side::Right);
    s.set_delay(Duration::from_millis(200));
    let t0 = Instant::now();
    let at = |ms| t0 + Duration::from_millis(ms);
    let edge = Point::new(999, 400);
    assert_eq!(s.local_motion(edge, 5, 0, at(0)), None, "刚贴上边");
    assert_eq!(s.local_motion(edge, 5, 0, at(150)), None, "还没停够");
    assert!(
        matches!(
            s.local_motion(edge, 5, 0, at(200)),
            Some(Transition::Enter { .. })
        ),
        "停够了再推就过去"
    );
}

#[test]
fn moving_off_the_edge_starts_the_wait_over() {
    use std::time::Duration;

    let mut s = switch(Side::Right);
    s.set_delay(Duration::from_millis(200));
    let t0 = Instant::now();
    let at = |ms| t0 + Duration::from_millis(ms);
    let edge = Point::new(999, 400);
    assert_eq!(s.local_motion(edge, 5, 0, at(0)), None);
    assert_eq!(s.local_motion(Point::new(990, 400), -9, 0, at(100)), None);
    assert_eq!(
        s.local_motion(edge, 9, 0, at(250)),
        None,
        "离开过边，重新计时"
    );
    assert_eq!(s.local_motion(edge, 1, 0, at(400)), None);
    assert!(matches!(
        s.local_motion(edge, 1, 0, at(450)),
        Some(Transition::Enter { .. })
    ));
}

#[test]
fn resting_on_the_edge_without_pushing_also_counts() {
    use std::time::Duration;

    let mut s = switch(Side::Right);
    s.set_delay(Duration::from_millis(200));
    let t0 = Instant::now();
    let at = |ms| t0 + Duration::from_millis(ms);
    // 沿着边移动，没有往外推。
    assert_eq!(s.local_motion(Point::new(999, 400), 0, 3, at(0)), None);
    assert!(matches!(
        s.local_motion(Point::new(999, 403), 2, 0, at(300)),
        Some(Transition::Enter { .. })
    ));
}

#[test]
fn returning_waits_at_each_edge_and_still_requires_an_outward_push() {
    use std::time::Duration;

    for (side, edge, outward, tangent) in [
        (Side::Right, Point::new(999, 400), (1, 0), (0, 1)),
        (Side::Left, Point::new(0, 400), (-1, 0), (0, 1)),
        (Side::Bottom, Point::new(500, 799), (0, 1), (1, 0)),
        (Side::Top, Point::new(500, 0), (0, -1), (1, 0)),
    ] {
        let mut s = switch(side);
        let t0 = Instant::now();
        let at = |ms| t0 + Duration::from_millis(ms);
        // 先进入，再启用延迟，只测返回方向。
        assert!(matches!(
            s.local_motion(edge, outward.0, outward.1, at(0)),
            Some(Transition::Enter { .. })
        ));
        s.set_delay(Duration::from_millis(500));
        let back = (-outward.0, -outward.1);
        assert!(
            matches!(
                s.remote_motion(back.0, back.1, true, at(0)),
                Some(Transition::Move(_))
            ),
            "{side:?}：刚抵达返回边缘"
        );
        assert_eq!(s.remote_motion(back.0, back.1, true, at(499)), None);
        assert!(s.is_remote());
        assert!(
            matches!(
                s.remote_motion(tangent.0, tangent.1, true, at(500)),
                Some(Transition::Move(_))
            ),
            "沿边移动不切换"
        );
        assert!(
            matches!(
                s.remote_motion(back.0, back.1, true, at(500)),
                Some(Transition::Leave { .. })
            ),
            "{side:?}：停够时间并向外推才返回"
        );
    }
}

#[test]
fn leaving_the_return_edge_or_dragging_restarts_the_wait() {
    use std::time::Duration;

    let mut s = switch(Side::Right);
    let t0 = Instant::now();
    let at = |ms| t0 + Duration::from_millis(ms);
    s.local_motion(Point::new(999, 400), 1, 0, at(0));
    s.set_delay(Duration::from_millis(200));
    s.remote_motion(-50, 0, true, at(0));
    s.remote_motion(10, 0, true, at(150));
    assert!(matches!(
        s.remote_motion(-50, 0, true, at(300)),
        Some(Transition::Move(_))
    ));
    assert_eq!(s.remote_motion(-1, 0, true, at(499)), None);
    // 按住鼠标时即使没有可见位移也清空计时。
    assert_eq!(s.remote_motion(0, 0, false, at(500)), None);
    assert_eq!(s.remote_motion(-1, 0, true, at(900)), None);
    assert_eq!(s.remote_motion(-1, 0, true, at(1099)), None);
    assert!(matches!(
        s.remote_motion(-1, 0, true, at(1100)),
        Some(Transition::Leave { .. })
    ));
}

#[test]
fn forced_return_and_disconnect_clear_the_wait_without_delaying() {
    use std::time::Duration;

    for disconnected in [false, true] {
        let mut s = switch(Side::Right);
        let t0 = Instant::now();
        let at = |ms| t0 + Duration::from_millis(ms);
        s.local_motion(Point::new(999, 400), 1, 0, at(0));
        s.set_delay(Duration::from_millis(500));
        s.remote_motion(-50, 0, true, at(0));
        if disconnected {
            assert!(s.abort().is_some());
        } else {
            assert!(matches!(s.leave(), Some(Transition::Leave { .. })));
        }
        assert_eq!(s.local_motion(Point::new(999, 400), 1, 0, at(1000)), None);
        assert!(matches!(
            s.local_motion(Point::new(999, 400), 1, 0, at(1500)),
            Some(Transition::Enter { .. })
        ));
        assert!(
            matches!(
                s.remote_motion(-50, 0, true, at(2000)),
                Some(Transition::Move(_))
            ),
            "进入时的等待不能用于返回"
        );
        assert_eq!(s.remote_motion(-1, 0, true, at(2499)), None);
    }
}

#[test]
fn changing_delay_or_remote_geometry_restarts_the_return_wait() {
    use std::time::Duration;

    let mut s = switch(Side::Right);
    let t0 = Instant::now();
    let at = |ms| t0 + Duration::from_millis(ms);
    s.local_motion(Point::new(999, 400), 1, 0, at(0));
    s.set_delay(Duration::from_millis(200));
    s.remote_motion(-50, 0, true, at(0));
    s.set_delay(Duration::from_millis(300));
    assert_eq!(s.remote_motion(-1, 0, true, at(400)), None);
    s.set_remote(Rect::new(0, 0, 2000, 1200), 100);
    assert_eq!(s.remote_motion(-1, 0, true, at(800)), None);
    assert_eq!(s.remote_motion(-1, 0, true, at(1099)), None);
    assert!(matches!(
        s.remote_motion(-1, 0, true, at(1100)),
        Some(Transition::Leave { .. })
    ));
}

/// 两台电脑各有一大一小两块屏幕：B 主屏 1920×1080、右边一块竖屏 1080×1920；
/// A 在 B 左边，主屏 4096×2160、右边一块 1920×1080 的副屏，顶部对齐。
fn uneven() -> EdgeSwitch {
    let mut s = EdgeSwitch::new(
        Rect::new(0, 0, 3000, 1920),
        Rect::new(0, 0, 6016, 2160),
        Side::Left,
        100,
    );
    s.set_local_screens(&[Rect::new(0, 0, 1920, 1080), Rect::new(1920, 0, 1080, 1920)]);
    s.set_remote_screens(&[Rect::new(0, 0, 4096, 2160), Rect::new(4096, 0, 1920, 1080)]);
    s
}

#[test]
fn entering_lands_on_the_screen_that_touches_the_edge() {
    // 从 B 主屏左边缘的最下面推出去：落在 A 副屏右边缘的最下面，而不是副屏下方没有屏幕的空白。
    let mut s = uneven();
    let t = s.local_motion(Point::new(0, 1079), -5, 0, Instant::now());
    let Some(Transition::Enter { at, .. }) = t else {
        panic!("应当进入远端：{t:?}");
    };
    assert!(Rect::new(4096, 0, 1920, 1080).contains(at), "{at:?}");
    assert_eq!(at.y, 1079);
}

#[test]
fn the_cursor_slides_along_a_screen_instead_of_entering_a_gap() {
    let mut s = uneven();
    s.local_motion(Point::new(0, 1000), -5, 0, Instant::now());
    // 在 A 副屏上往下推：停在副屏的下边，不进空白。
    s.remote_motion(0, 500, true, Instant::now());
    assert_eq!(s.remote_position().map(|p| p.y), Some(1079));
    // 斜着推：横向照走，纵向被挡住。
    s.remote_motion(-100, 50, true, Instant::now());
    let at = s.remote_position().unwrap();
    assert_eq!(at.y, 1079);
    assert!(Rect::new(4096, 0, 1920, 1080).contains(at));
    // 往左走到 A 主屏上，就能到下半部分。
    s.remote_motion(-3000, 0, true, Instant::now());
    s.remote_motion(0, 900, true, Instant::now());
    let at = s.remote_position().unwrap();
    assert!(at.x < 4096 && at.y > 1080, "{at:?}");
    // 在主屏下半部分往右推：被副屏下方的空白挡住，回不了本机。
    let t = s.remote_motion(9000, 0, true, Instant::now());
    assert!(!matches!(t, Some(Transition::Leave { .. })));
    assert_eq!(s.remote_position().map(|p| p.x), Some(4095));
}

#[test]
fn leaving_maps_back_onto_the_local_screen_that_touches_the_edge() {
    let mut s = uneven();
    s.local_motion(Point::new(0, 540), -5, 0, Instant::now());
    s.remote_motion(0, 539, true, Instant::now());
    // 从 A 副屏右边缘最下面回来：回到 B 主屏左边缘最下面。
    let t = s.remote_motion(9000, 0, true, Instant::now());
    let Some(Transition::Leave { local_at, .. }) = t else {
        panic!("应当回到本机：{t:?}");
    };
    assert_eq!(local_at.y, 1079);
}
