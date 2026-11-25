//! Linux 暂不提供自动更新安装。

use std::path::Path;

use ferry_platform::{Error, Prepared, Result, Updater};

const UNSUPPORTED: &str = "Linux 暂不支持自动更新";

/// 不执行更新的更新器。
#[derive(Clone, Copy, Debug, Default)]
pub struct LinuxUpdater;

impl LinuxUpdater {
    pub fn new() -> Self {
        Self
    }
}

impl Updater for LinuxUpdater {
    fn platform(&self) -> Option<&'static str> {
        None
    }

    fn can_install(&self) -> Result<()> {
        Err(Error::Unsupported(UNSUPPORTED))
    }

    fn proxy(&self) -> Option<String> {
        None
    }

    fn prepare(&self, _: &Path, _: &str) -> Result<Prepared> {
        Err(Error::Unsupported(UNSUPPORTED))
    }

    fn launch(&self, _: &Prepared) -> Result<()> {
        Err(Error::Unsupported(UNSUPPORTED))
    }

    fn confirm(&self, _: &Prepared) {}

    fn rollback(&self, _: &Prepared) -> Result<()> {
        Err(Error::Unsupported(UNSUPPORTED))
    }
}
