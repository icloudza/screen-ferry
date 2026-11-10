//! 屏幕布局：远端设备相对本机的位置，以及显示器的外接矩形。

use ferry_types::{Display, Rect};

/// 远端设备位于本机的哪一侧；定义在 `ferry-types`，配置文件里也要用。
pub use ferry_types::Side;

/// 所有显示器的外接矩形。没有显示器时返回 `None`。
///
/// 切换边缘按这个矩形判断：显示器排成 L 形时，凹进去的那段边缘不会触发切换。
/// 矩形里没有屏幕的空白由 `EdgeSwitch` 按各块显示器的范围避开。
pub fn bounds(displays: &[Display]) -> Option<Rect> {
    let (first, rest) = displays.split_first()?;
    let (mut left, mut top) = (first.bounds.x, first.bounds.y);
    let (mut right, mut bottom) = (first.bounds.right(), first.bounds.bottom());
    for d in rest {
        left = left.min(d.bounds.x);
        top = top.min(d.bounds.y);
        right = right.max(d.bounds.right());
        bottom = bottom.max(d.bounds.bottom());
    }
    Some(Rect::new(
        left,
        top,
        right.abs_diff(left),
        bottom.abs_diff(top),
    ))
}
