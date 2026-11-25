//! 显示器信息：`Displays` 的 macOS 实现。
//!
//! 坐标使用 macOS 全局显示坐标：以主显示器左上角为原点，单位为点（point）。
//! 点本身就是逻辑单位，所以 `scale_percent` 恒为 100，与 Retina 无关。

use ferry_platform::{Displays, Error, Result};
use ferry_types::{Display, DisplayId, Rect};
use objc2_core_graphics::{
    CGDirectDisplayID, CGDisplayBounds, CGError, CGGetActiveDisplayList, CGGetOnlineDisplayList,
    CGMainDisplayID,
};

const MAX_DISPLAYS: usize = 32;

/// 本机显示器列表。
#[derive(Clone, Copy, Debug, Default)]
pub struct MacDisplays;

impl Displays for MacDisplays {
    fn displays(&self) -> Result<Vec<Display>> {
        let mut ids: [CGDirectDisplayID; MAX_DISPLAYS] = [0; MAX_DISPLAYS];
        let mut count = 0u32;
        // SAFETY: 两个指针都指向足够大的本地变量。
        let err =
            unsafe { CGGetActiveDisplayList(MAX_DISPLAYS as u32, ids.as_mut_ptr(), &mut count) };
        check(err, "CGGetActiveDisplayList")?;
        // 显示器休眠时 active list 为空，但 online list 仍保留几何信息。
        if count == 0 {
            let err = unsafe {
                CGGetOnlineDisplayList(MAX_DISPLAYS as u32, ids.as_mut_ptr(), &mut count)
            };
            check(err, "CGGetOnlineDisplayList")?;
        }

        let main = CGMainDisplayID();
        Ok(ids
            .iter()
            .take(count as usize)
            .map(|&id| describe(id, id == main))
            .collect())
    }
}

fn describe(id: CGDirectDisplayID, primary: bool) -> Display {
    let b = CGDisplayBounds(id);
    Display {
        id: DisplayId(id),
        bounds: Rect::new(
            b.origin.x.round() as i32,
            b.origin.y.round() as i32,
            b.size.width.round() as u32,
            b.size.height.round() as u32,
        ),
        scale_percent: 100,
        primary,
    }
}

pub(crate) fn check(err: CGError, call: &'static str) -> Result<()> {
    if err == CGError::Success {
        return Ok(());
    }
    Err(Error::Os {
        call,
        detail: format!("CGError {}", err.0),
    })
}
