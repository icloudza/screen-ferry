//! 代理不接受程序、配置或脚本路径。管理员的 prepare 命令仅接收安装根目录。

use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mode {
    Run,
    Install,
    Prepare {
        directory: PathBuf,
    },
    Stop,
    Remove,
    Agent {
        parent_pid: u32,
        stop_event: usize,
    },
    Authorize {
        user_sid: String,
        code: String,
    },
    Revoke {
        user_sid: String,
        fingerprint: String,
    },
    Help,
}

pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Mode, String> {
    let mut args = args.into_iter();
    let mode = match args.next().as_deref() {
        None | Some("help" | "--help" | "-h") => Mode::Help,
        Some("run") => Mode::Run,
        Some("install") => Mode::Install,
        Some("prepare") => Mode::Prepare {
            directory: PathBuf::from(args.next().ok_or("prepare 需要安装根目录")?),
        },
        Some("stop") => Mode::Stop,
        Some("remove") => Mode::Remove,
        Some("authorize") => Mode::Authorize {
            user_sid: args
                .next()
                .filter(|s| s.len() <= 184)
                .ok_or("authorize 需要用户 SID")?,
            code: args
                .next()
                .filter(|s| s.len() == 195)
                .ok_or("authorize 需要控制端的公开授权码")?,
        },
        Some("revoke") => Mode::Revoke {
            user_sid: args
                .next()
                .filter(|s| s.len() <= 184)
                .ok_or("revoke 需要用户 SID")?,
            fingerprint: args
                .next()
                .filter(|s| s.len() == 64)
                .ok_or("revoke 需要完整设备指纹")?,
        },
        Some("agent") => {
            let parent_pid = args
                .next()
                .and_then(|v| v.parse::<u32>().ok())
                .filter(|&v| v != 0)
                .ok_or("agent 需要父服务 PID")?;
            let stop_event = args
                .next()
                .and_then(|v| v.parse::<usize>().ok())
                .filter(|&v| v != 0)
                .ok_or("agent 需要父服务的停止事件句柄")?;
            Mode::Agent {
                parent_pid,
                stop_event,
            }
        }
        _ => return Err("未知输入服务命令".into()),
    };
    if args.next().is_some() {
        return Err("输入服务不接受额外参数".into());
    }
    Ok(mode)
}
