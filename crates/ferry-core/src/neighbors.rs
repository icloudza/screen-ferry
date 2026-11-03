//! 多台设备的布局与切换：每台邻居贴在本机的某一侧，同一侧的几台平分那条边。
//!
//! 光标在本机时，推出哪一段边缘就进入那一段的邻居；在邻居那里时，从朝向本机的边推出去，
//! 回到本机的同一段。每台邻居的进出换算交给各自的 [`EdgeSwitch`]：在它眼里，本机就是自己那一段。
//! 同一时刻光标最多在一台邻居上。

use std::time::{Duration, Instant};

use ferry_types::{Point, Rect};

use crate::edge_switch::{EdgeSwitch, Transition};
use crate::layout::Side;

/// 一台邻居在布局里的位置。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Place {
    pub side: Side,
    /// 同一侧排第几：小的在上（左右两侧）或在左（上下两侧）。相同时按加入的先后。
    pub rank: u32,
}

struct Neighbor {
    id: u64,
    place: Place,
    /// 加入的先后，排名相同时用它。
    joined: u64,
    switch: EdgeSwitch,
    /// 本机朝向它的那一段：一个与本机同宽（左右两侧）或同高（上下两侧）的矩形。
    segment: Rect,
    scale: u16,
}

/// 本机四周的邻居。
pub struct Neighbors {
    local: Rect,
    list: Vec<Neighbor>,
    /// 光标所在的邻居。
    active: Option<u64>,
    joined: u64,
}

impl Neighbors {
    /// 本机所有显示器的外接矩形为 `local`，还没有邻居。
    pub fn new(local: Rect) -> Self {
        Self {
            local,
            list: Vec::new(),
            active: None,
            joined: 0,
        }
    }

    /// 加入一台邻居：它的屏幕为 `remote`，本机移动 1 个单位它移动 `scale_percent` 个单位的百分数。
    /// 同一侧的邻居随之重新平分那条边；同一个 `id` 已经在时先去掉旧的。
    pub fn insert(&mut self, id: u64, place: Place, remote: Rect, scale_percent: u16) {
        self.list.retain(|n| n.id != id);
        if self.active == Some(id) {
            self.active = None;
        }
        self.joined += 1;
        self.list.push(Neighbor {
            id,
            place,
            joined: self.joined,
            switch: EdgeSwitch::new(self.local, remote, place.side, scale_percent),
            segment: self.local,
            scale: scale_percent,
        });
        self.arrange();
    }

    /// 去掉一台邻居（断开了）。光标原本在它上面时，返回本机光标应放到的位置。
    pub fn remove(&mut self, id: u64) -> Option<Point> {
        let back = if self.active == Some(id) {
            self.active = None;
            self.find_mut(id).and_then(|n| n.switch.abort())
        } else {
            None
        };
        self.list.retain(|n| n.id != id);
        self.arrange();
        back
    }

    /// 改变一台邻居的位置。光标在它上面、而它换到了另一侧时，先回到本机，返回 `Leave`。
    pub fn set_place(&mut self, id: u64, place: Place) -> Option<Transition> {
        let neighbor = self.find_mut(id)?;
        if neighbor.place == place {
            return None;
        }
        let leave = neighbor.switch.set_side(place.side);
        neighbor.place = place;
        if leave.is_some() {
            self.active = None;
        }
        self.arrange();
        leave
    }

    /// 本机的显示器布局变了。各段随之重新划分。
    pub fn set_local(&mut self, local: Rect) {
        self.local = local;
        self.arrange();
    }

    /// 一台邻居的屏幕或缩放比例变了。
    pub fn set_remote(&mut self, id: u64, remote: Rect, scale_percent: u16) {
        if let Some(neighbor) = self.find_mut(id) {
            neighbor.scale = scale_percent;
            neighbor.switch.set_remote(remote, scale_percent);
        }
    }

    /// 一台邻居的光标速度：光标在它上面时，位移再乘以 `numerator / denominator`。
    pub fn set_speed(&mut self, id: u64, numerator: u32, denominator: u32) {
        if let Some(neighbor) = self.find_mut(id) {
            neighbor.switch.set_speed(numerator, denominator);
        }
    }

    /// 进入这台邻居和从它返回本机，都要在切换边缘停够 `delay`。
    pub fn set_delay(&mut self, id: u64, delay: Duration) {
        if let Some(neighbor) = self.find_mut(id) {
            neighbor.switch.set_delay(delay);
        }
    }

    /// 按下或松开鼠标键后重新计算停留时间，拖动期间的等待不带到松开之后。
    pub fn reset_wait(&mut self) {
        for neighbor in &mut self.list {
            neighbor.switch.left_edge();
        }
    }

    /// 光标在本机时的一次移动。推出某一段边缘时返回那台邻居和 `Enter`。
    ///
    /// `position` 是光标被边缘挡住后的位置，`dx`、`dy` 是这次的位移，`now` 是这次移动的时刻。
    pub fn local_motion(
        &mut self,
        position: Point,
        dx: i32,
        dy: i32,
        now: Instant,
    ) -> Option<(u64, Transition)> {
        if self.active.is_some() {
            return None;
        }
        let mut entered = None;
        for neighbor in &mut self.list {
            if entered.is_some() || !along_inside(neighbor.place.side, neighbor.segment, position) {
                // 光标不在这一段（或者已经切到了别的邻居）：重新计算在这一段上停了多久。
                neighbor.switch.left_edge();
                continue;
            }
            if let Some(t) = neighbor.switch.local_motion(position, dx, dy, now) {
                entered = Some((neighbor.id, t));
            }
        }
        if let Some((id, _)) = entered {
            self.active = Some(id);
        }
        entered
    }

    /// 光标在邻居上时的一次相对移动。`can_leave` 为假时（例如还按着鼠标键）不回本机。
    pub fn remote_motion(
        &mut self,
        dx: i32,
        dy: i32,
        can_leave: bool,
        now: Instant,
    ) -> Option<(u64, Transition)> {
        let id = self.active?;
        let t = self
            .find_mut(id)?
            .switch
            .remote_motion(dx, dy, can_leave, now)?;
        if matches!(t, Transition::Leave { .. }) {
            self.active = None;
        }
        Some((id, t))
    }

    /// 主动回到本机（例如按了快捷键）。光标原本在邻居上时返回它和 `Leave`。
    pub fn leave(&mut self) -> Option<(u64, Transition)> {
        let id = self.active.take()?;
        let t = self.find_mut(id)?.switch.leave()?;
        Some((id, t))
    }

    /// 出了问题，立即回到本机。光标原本在邻居上时，返回它和本机光标应放到的位置。
    pub fn abort(&mut self) -> Option<(u64, Point)> {
        let id = self.active.take()?;
        let at = self.find_mut(id)?.switch.abort()?;
        Some((id, at))
    }

    /// 光标所在的邻居。
    pub fn active(&self) -> Option<u64> {
        self.active
    }

    /// 光标在邻居上时的位置，使用那台邻居的坐标系。
    pub fn remote_position(&self) -> Option<Point> {
        let id = self.active?;
        self.list
            .iter()
            .find(|n| n.id == id)
            .and_then(|n| n.switch.remote_position())
    }

    /// 一台邻居在哪一侧，以及本机朝向它的那一段。
    pub fn segment(&self, id: u64) -> Option<(Side, Rect)> {
        let n = self.list.iter().find(|n| n.id == id)?;
        Some((n.place.side, n.segment))
    }

    /// 所有邻居。
    pub fn ids(&self) -> impl Iterator<Item = u64> + '_ {
        self.list.iter().map(|n| n.id)
    }

    fn find_mut(&mut self, id: u64) -> Option<&mut Neighbor> {
        self.list.iter_mut().find(|n| n.id == id)
    }

    /// 同一侧的邻居按排名平分那条边。
    fn arrange(&mut self) {
        for side in [Side::Left, Side::Right, Side::Top, Side::Bottom] {
            let mut on: Vec<usize> = (0..self.list.len())
                .filter(|&i| self.list[i].place.side == side)
                .collect();
            on.sort_by_key(|&i| (self.list[i].place.rank, self.list[i].joined));
            let count = on.len();
            for (k, i) in on.into_iter().enumerate() {
                let segment = split(self.local, side, k, count);
                let neighbor = &mut self.list[i];
                neighbor.segment = segment;
                neighbor.switch.set_local(segment, neighbor.scale);
            }
        }
    }
}

/// `local` 朝向 `side` 的那条边平分成 `count` 段，第 `k` 段对应的矩形。
fn split(local: Rect, side: Side, k: usize, count: usize) -> Rect {
    let (k, count) = (k as u64, count.max(1) as u64);
    let cut = |len: u32| -> (u32, u32) {
        let len = u64::from(len);
        let start = len * k / count;
        let end = len * (k + 1) / count;
        (start as u32, (end - start) as u32)
    };
    match side {
        Side::Left | Side::Right => {
            let (start, len) = cut(local.height);
            Rect::new(local.x, local.y + start as i32, local.width, len)
        }
        Side::Top | Side::Bottom => {
            let (start, len) = cut(local.width);
            Rect::new(local.x + start as i32, local.y, len, local.height)
        }
    }
}

/// 沿着 `side` 那条边看，`p` 是否落在 `segment` 这一段里。
fn along_inside(side: Side, segment: Rect, p: Point) -> bool {
    match side {
        Side::Left | Side::Right => p.y >= segment.y && p.y < segment.bottom(),
        Side::Top | Side::Bottom => p.x >= segment.x && p.x < segment.right(),
    }
}
