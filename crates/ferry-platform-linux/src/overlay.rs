//! Linux 边缘浮层占位实现；不提供边缘拖放能力。

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use ferry_platform::{Overlay, OverlaySink, Result};
use ferry_types::{Frame, Scene};

/// 不支持边缘拖放的浮层；宿主必须关闭 EDGE_DROP 能力。
pub struct LinuxOverlay {
    _sink: Option<OverlaySink>,
}

impl LinuxOverlay {
    pub fn new() -> Self {
        Self { _sink: None }
    }
}

impl Default for LinuxOverlay {
    fn default() -> Self {
        Self::new()
    }
}

impl Overlay for LinuxOverlay {
    fn start(&mut self, sink: OverlaySink) -> Result<()> {
        self._sink = Some(sink);
        Ok(())
    }

    fn show(&mut self, _: Frame, _: f32, _: Scene) {}

    fn hide(&mut self) {}

    fn drag_count(&self) -> i64 {
        0
    }

    fn dragged_files(&self) -> Vec<PathBuf> {
        Vec::new()
    }

    fn offer(&mut self, _: Vec<PathBuf>) {}

    fn watch_edge(&mut self, _: Option<Frame>) {}

    fn open(&self, path: &Path) {
        open_with_xdg(path);
    }

    fn reveal(&self, path: &Path) {
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        open_with_xdg(parent);
    }
}

fn open_with_xdg(path: &Path) {
    if let Err(error) = Command::new("xdg-open")
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        eprintln!("打开文件或文件夹失败：{error}");
    }
}
