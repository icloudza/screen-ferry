//! 边缘切换：根据光标移动决定何时进入、离开远端屏幕，并维护远端光标位置。

use std::time::{Duration, Instant};

use ferry_types::{Point, Rect};

use crate::layout::Side;

/// 切换之后，光标离边缘至少留出这么多个坐标单位。
///
/// 正好落在边缘上时，手轻轻一抖、往回退一个单位就会立刻切回去，在两边来回弹。
/// 留出一点距离后，要真的往回走才会切回去。
const INSET: i64 = 1;

/// 一次切换或移动，由调用方执行。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transition {
    /// 进入远端：抓取本机输入，发送 `Enter`。
    Enter { seq: u32, at: Point },
    /// 远端光标移动到新位置。
    Move(Point),
    /// 回到本机：结束抓取并把本机光标放到 `local_at`，发送 `Leave`。
    Leave { seq: u32, local_at: Point },
}

/// 两台设备之间的边缘切换。
///
/// 光标在本机时，撞到远端所在一侧的边缘并继续向外推，就进入远端；
/// 在远端时，从朝向本机的那条边缘推出去，就回到本机。
/// 进出位置沿边缘按比例换算，所以两边分辨率不同也能对齐。
///
/// 越过边缘的那一段位移会带到另一边：快速划过时光标是连贯的，不会在边缘停一下。
/// 落点离边缘至少留出一个单位，免得在边缘来回弹。
///
/// 可以要求光标在边上停够一段时间才切过去（防止误切），也可以让光标在远端走得快一些或慢一些（光标速度）。
#[derive(Clone, Debug)]
pub struct EdgeSwitch {
    local: Rect,
    remote: Rect,
    side: Side,
    /// 本机位移换算到远端的比例（百分数），来自两边显示器的缩放比例。
    scale_percent: i64,
    /// 光标速度：远端的位移再乘以分子 / 分母。
    speed: (i64, i64),
    /// 光标要在边上停这么久才切过去。
    delay: Duration,
    /// 光标贴上切换边缘的时刻；进出共用，切换后清空。
    arrived: Option<Instant>,
    seq: u32,
    remote_at: Option<Point>,
    /// 换算后不足一个单位的余数，累积到下一次。
    carry: (i64, i64),
}

impl EdgeSwitch {
    /// `scale_percent`：本机移动 1 个单位，远端移动多少个单位的百分数。
    pub fn new(local: Rect, remote: Rect, side: Side, scale_percent: u16) -> Self {
        Self {
            local,
            remote,
            side,
            scale_percent: i64::from(scale_percent),
            speed: (1, 1),
            delay: Duration::ZERO,
            arrived: None,
            seq: 0,
            remote_at: None,
            carry: (0, 0),
        }
    }

    /// 光标在远端时，位移再乘以 `numerator / denominator`（例如“这台设备的 DPI / 本机鼠标的 DPI”）。
    /// 缺省 1/1。余数按分母累积，不丢精度。分母为 0 时当作 1。
    pub fn set_speed(&mut self, numerator: u32, denominator: u32) {
        let speed = (i64::from(numerator), i64::from(denominator.max(1)));
        if speed != self.speed {
            self.speed = speed;
            self.carry = (0, 0);
        }
    }

    /// 进入远端和返回本机都要在切换边缘停够 `delay`。缺省为 0，推出去就切。
    pub fn set_delay(&mut self, delay: Duration) {
        if self.delay != delay {
            self.arrived = None;
        }
        self.delay = delay;
    }

    /// 光标离开了这一段边（例如沿边移到了另一台设备那一段）：重新计算停留时间。
    pub fn left_edge(&mut self) {
        self.arrived = None;
    }

    pub fn is_remote(&self) -> bool {
        self.remote_at.is_some()
    }

    /// 光标在远端时的位置，使用远端坐标系。
    pub fn remote_position(&self) -> Option<Point> {
        self.remote_at
    }

    /// 本机的显示器布局变了（改分辨率、插拔显示器）。之后的进出都按新的范围判断。
    pub fn set_local(&mut self, local: Rect, scale_percent: u16) {
        if self.local != local {
            self.arrived = None;
        }
        self.local = local;
        self.scale_percent = i64::from(scale_percent);
    }

    /// 远端的显示器布局变了。光标在远端时会被收进新的范围内。
    pub fn set_remote(&mut self, remote: Rect, scale_percent: u16) {
        if self.remote != remote {
            self.arrived = None;
        }
        self.remote = remote;
        self.scale_percent = i64::from(scale_percent);
        if let Some(at) = self.remote_at {
            self.remote_at = Some(clamp_into(at, remote));
        }
    }

    /// 光标在本机时的一次移动。需要进入远端时返回 `Enter`。
    ///
    /// `position` 是光标被边缘挡住后的位置，`dx`、`dy` 是这次的位移，`now` 是这次移动的时刻。
    /// 设了停留时间时，光标贴上边之后要停够这么久，再往外推才切过去。
    pub fn local_motion(
        &mut self,
        position: Point,
        dx: i32,
        dy: i32,
        now: Instant,
    ) -> Option<Transition> {
        if self.is_remote() || is_empty(self.remote) {
            return None;
        }
        let (l, r) = (self.local, self.remote);
        let (at_edge, outward) = match self.side {
            Side::Right => (position.x >= l.right() - 1, dx > 0),
            Side::Left => (position.x <= l.x, dx < 0),
            Side::Bottom => (position.y >= l.bottom() - 1, dy > 0),
            Side::Top => (position.y <= l.y, dy < 0),
        };
        if !at_edge {
            self.arrived = None;
            return None;
        }
        let arrived = *self.arrived.get_or_insert(now);
        if !outward || now.saturating_duration_since(arrived) < self.delay {
            return None;
        }
        self.arrived = None;

        // 没有边缘挡着的话，这次位移会把光标带出边缘多远。
        let (x, y) = (
            i64::from(position.x) + i64::from(dx),
            i64::from(position.y) + i64::from(dy),
        );
        let beyond = match self.side {
            Side::Right => x - i64::from(l.right() - 1),
            Side::Left => i64::from(l.x) - x,
            Side::Bottom => y - i64::from(l.bottom() - 1),
            Side::Top => i64::from(l.y) - y,
        };
        let depth = (self.to_remote(beyond) - 1).max(INSET);
        let at = match self.side {
            Side::Right => Point::new(offset(r.x, depth), along_y(position.y, l, r)),
            Side::Left => Point::new(offset(r.right() - 1, -depth), along_y(position.y, l, r)),
            Side::Bottom => Point::new(along_x(position.x, l, r), offset(r.y, depth)),
            Side::Top => Point::new(along_x(position.x, l, r), offset(r.bottom() - 1, -depth)),
        };
        let at = clamp_into(at, r);
        self.seq = self.seq.wrapping_add(1);
        self.remote_at = Some(at);
        self.carry = (0, 0);
        Some(Transition::Enter { seq: self.seq, at })
    }

    /// 光标在远端时的一次相对移动。
    ///
    /// `can_leave` 为假时（例如还按着鼠标键）光标停在边缘，不回本机，也不累计停留时间。
    /// `now` 是这次移动的时刻；在返回边缘停够延迟后，再往外推才回本机。
    pub fn remote_motion(
        &mut self,
        dx: i32,
        dy: i32,
        can_leave: bool,
        now: Instant,
    ) -> Option<Transition> {
        let at = self.remote_at?;
        if !can_leave {
            self.arrived = None;
        }
        let (sx, sy) = self.scale(dx, dy);
        if sx == 0 && sy == 0 {
            return None;
        }
        let r = self.remote;
        let (x, y) = (i64::from(at.x) + sx, i64::from(at.y) + sy);
        let next = Point::new(
            x.clamp(i64::from(r.x), i64::from(r.right() - 1)) as i32,
            y.clamp(i64::from(r.y), i64::from(r.bottom() - 1)) as i32,
        );

        // 从朝向本机的边缘推出去多远。
        let beyond = match self.side {
            Side::Right => i64::from(r.x) - x,
            Side::Left => x - i64::from(r.right() - 1),
            Side::Bottom => i64::from(r.y) - y,
            Side::Top => y - i64::from(r.bottom() - 1),
        };
        let at_edge = match self.side {
            Side::Right => next.x == r.x,
            Side::Left => next.x == r.right() - 1,
            Side::Bottom => next.y == r.y,
            Side::Top => next.y == r.bottom() - 1,
        };
        let ready = if can_leave && at_edge {
            let arrived = *self.arrived.get_or_insert(now);
            now.saturating_duration_since(arrived) >= self.delay
        } else {
            self.arrived = None;
            false
        };
        if ready && beyond > 0 {
            self.arrived = None;
            self.remote_at = None;
            let depth = (self.to_local(beyond) - 1).max(INSET);
            return Some(Transition::Leave {
                seq: self.seq,
                local_at: self.local_point(next, depth),
            });
        }

        if next == at {
            return None;
        }
        self.remote_at = Some(next);
        Some(Transition::Move(next))
    }

    /// 远端所在的一侧。
    pub fn side(&self) -> Side {
        self.side
    }

    /// 改变远端所在的一侧。光标在远端时先回到本机，返回 `Leave`。
    pub fn set_side(&mut self, side: Side) -> Option<Transition> {
        if side == self.side {
            return None;
        }
        let leave = self.leave();
        self.side = side;
        leave
    }

    /// 主动回到本机（例如按了快捷键）。光标原本在远端时返回 `Leave`。
    pub fn leave(&mut self) -> Option<Transition> {
        let local_at = self.abort()?;
        Some(Transition::Leave {
            seq: self.seq,
            local_at,
        })
    }

    /// 连接中断等情况下立即回到本机。光标原本在远端时，返回本机光标应放到的位置。
    pub fn abort(&mut self) -> Option<Point> {
        self.arrived = None;
        let at = self.remote_at.take()?;
        Some(self.local_point(at, INSET))
    }

    /// 回到本机时光标的位置：沿边缘与远端位置 `at` 对齐，离边缘 `depth` 个单位。
    fn local_point(&self, at: Point, depth: i64) -> Point {
        let (l, r) = (self.local, self.remote);
        let p = match self.side {
            Side::Right => Point::new(offset(l.right() - 1, -depth), along_y(at.y, r, l)),
            Side::Left => Point::new(offset(l.x, depth), along_y(at.y, r, l)),
            Side::Bottom => Point::new(along_x(at.x, r, l), offset(l.bottom() - 1, -depth)),
            Side::Top => Point::new(along_x(at.x, r, l), offset(l.y, depth)),
        };
        clamp_into(p, l)
    }

    /// 换算比例：本机移动 1 个单位，远端移动 `分子 / 分母` 个单位。
    fn ratio(&self) -> (i64, i64) {
        (self.scale_percent * self.speed.0, 100 * self.speed.1)
    }

    /// 本机的一段距离换算到远端。
    fn to_remote(&self, v: i64) -> i64 {
        let (n, d) = self.ratio();
        v * n / d
    }

    /// 远端的一段距离换算回本机。
    fn to_local(&self, v: i64) -> i64 {
        let (n, d) = self.ratio();
        v * d / n.max(1)
    }

    fn scale(&mut self, dx: i32, dy: i32) -> (i64, i64) {
        let (n, d) = self.ratio();
        self.carry.0 += i64::from(dx) * n;
        self.carry.1 += i64::from(dy) * n;
        let step = (self.carry.0 / d, self.carry.1 / d);
        self.carry = (self.carry.0 % d, self.carry.1 % d);
        step
    }
}

fn is_empty(r: Rect) -> bool {
    r.width == 0 || r.height == 0
}

/// 从 `edge` 移动 `by` 个单位，超出 `i32` 时取边界值。
fn offset(edge: i32, by: i64) -> i32 {
    (i64::from(edge) + by).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

fn clamp_into(p: Point, r: Rect) -> Point {
    Point::new(
        p.x.clamp(r.x, r.right() - 1),
        p.y.clamp(r.y, r.bottom() - 1),
    )
}

/// 把 `from` 中的纵坐标按比例换算到 `to`。
fn along_y(y: i32, from: Rect, to: Rect) -> i32 {
    proportional(y, from.y, from.height, to.y, to.height)
}

/// 把 `from` 中的横坐标按比例换算到 `to`。
fn along_x(x: i32, from: Rect, to: Rect) -> i32 {
    proportional(x, from.x, from.width, to.x, to.width)
}

fn proportional(v: i32, from: i32, from_len: u32, to: i32, to_len: u32) -> i32 {
    if from_len == 0 || to_len == 0 {
        return to;
    }
    let offset = (i64::from(v) - i64::from(from)).clamp(0, i64::from(from_len) - 1);
    let mapped = i64::from(to) + offset * i64::from(to_len) / i64::from(from_len);
    mapped.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}
