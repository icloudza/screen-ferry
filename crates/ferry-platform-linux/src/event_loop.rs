//! 基于 calloop 的主线程事件循环与 SIGHUP 唤醒。

use std::time::Duration;

use calloop::EventLoop as CalloopEventLoop;
use calloop::channel::{Event, channel as calloop_channel};
use calloop::signals::{Signal, Signals};

use ferry_platform::{Dispatcher, Error, EventLoop, MainTask, Result};

/// 主线程事件循环。
pub struct LinuxEventLoop {
    event_loop: CalloopEventLoop<'static, LoopState>,
    dispatcher: Dispatcher,
    reopen: Option<Box<dyn Fn()>>,
}

struct LoopState {
    reopen: Option<Box<dyn Fn()>>,
}

impl LinuxEventLoop {
    pub fn new() -> Result<Self> {
        let (sender, channel) = calloop_channel::<MainTask>();
        let event_loop = CalloopEventLoop::try_new().map_err(|error| Error::Os {
            call: "calloop::EventLoop::try_new",
            detail: error.to_string(),
        })?;
        let signals = Signals::new(&[Signal::SIGHUP]).map_err(|error| Error::Os {
            call: "calloop::Signals::new",
            detail: error.to_string(),
        })?;
        event_loop
            .handle()
            .insert_source(channel, |event, _, _: &mut LoopState| {
                if let Event::Msg(task) = event {
                    task();
                }
            })
            .map_err(|error| Error::Os {
                call: "insert_source(channel)",
                detail: error.to_string(),
            })?;
        event_loop
            .handle()
            .insert_source(signals, |_, _, state| {
                if let Some(handler) = &state.reopen {
                    handler();
                }
            })
            .map_err(|error| Error::Os {
                call: "insert_source(signals)",
                detail: error.to_string(),
            })?;
        let dispatcher = Dispatcher::new(move |task| {
            let _ = sender.send(task);
        });
        Ok(Self {
            event_loop,
            dispatcher,
            reopen: None,
        })
    }
}

impl EventLoop for LinuxEventLoop {
    fn dispatcher(&self) -> Dispatcher {
        self.dispatcher.clone()
    }

    fn on_reopen(&mut self, handler: Box<dyn Fn()>) {
        self.reopen = Some(handler);
    }

    fn run(mut self) -> ! {
        let mut state = LoopState {
            reopen: self.reopen.take(),
        };
        loop {
            if let Err(error) = self.event_loop.run(None::<Duration>, &mut state, |_| {}) {
                eprintln!("主线程事件循环失败：{error}");
                std::process::exit(1);
            }
        }
    }
}
