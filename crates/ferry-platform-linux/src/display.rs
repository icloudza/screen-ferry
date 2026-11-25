//! 通过 Wayland 原生枚举逻辑坐标下的显示器。

use wayland_client::protocol::wl_output::{self, Transform, WlOutput};
use wayland_client::protocol::wl_registry::{self, WlRegistry};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, WEnum};

use ferry_platform::{Displays, Error, Result};
use ferry_types::{Display, DisplayId, Rect};

/// Wayland 显示器枚举器。
pub struct LinuxDisplays;

impl LinuxDisplays {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LinuxDisplays {
    fn default() -> Self {
        Self::new()
    }
}

impl Displays for LinuxDisplays {
    fn displays(&self) -> Result<Vec<Display>> {
        let connection = Connection::connect_to_env().map_err(|error| Error::Os {
            call: "connect_to_env",
            detail: error.to_string(),
        })?;
        let mut queue = connection.new_event_queue();
        let handle = queue.handle();
        let mut state = DisplayState::default();
        let _registry = connection.display().get_registry(&handle, ());
        queue.roundtrip(&mut state).map_err(|error| Error::Os {
            call: "roundtrip",
            detail: error.to_string(),
        })?;
        queue.roundtrip(&mut state).map_err(|error| Error::Os {
            call: "roundtrip",
            detail: error.to_string(),
        })?;
        for output in &mut state.outputs {
            if output.version < 2 {
                output.done = true;
            }
        }

        Ok(state
            .outputs
            .into_iter()
            .filter(|output| output.done)
            .filter_map(OutputInfo::into_display)
            .collect())
    }
}

#[derive(Default)]
struct DisplayState {
    outputs: Vec<OutputInfo>,
}

struct OutputInfo {
    id: u32,
    version: u32,
    output: Option<WlOutput>,
    name: Option<String>,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    scale: u32,
    rotated: bool,
    done: bool,
}

impl OutputInfo {
    fn new(id: u32, output: WlOutput) -> Self {
        let version = output.version();
        Self {
            id,
            version,
            output: Some(output),
            name: None,
            x: 0,
            y: 0,
            width: 0,
            height: 0,
            scale: 1,
            rotated: false,
            done: false,
        }
    }

    fn into_display(self) -> Option<Display> {
        let scale = self.scale.max(1);
        let (width, height) = if self.rotated {
            (self.height, self.width)
        } else {
            (self.width, self.height)
        };
        let _name = self.name;
        Some(Display {
            id: DisplayId(self.id),
            bounds: Rect::new(self.x, self.y, width / scale, height / scale),
            // bounds 已是逻辑单位，与 engine 的桌面坐标一致。
            scale_percent: 100,
            primary: self.x == 0 && self.y == 0,
        })
    }
}

impl Dispatch<WlRegistry, ()> for DisplayState {
    fn event(
        state: &mut Self,
        registry: &WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        handle: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
            && interface == WlOutput::interface().name
        {
            let output = registry.bind(name, version.min(4), handle, ());
            state.outputs.push(OutputInfo::new(name, output));
        }
    }
}

impl Dispatch<WlOutput, ()> for DisplayState {
    fn event(
        state: &mut Self,
        proxy: &WlOutput,
        event: wl_output::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let id = proxy.id();
        let Some(output) = state
            .outputs
            .iter_mut()
            .find(|item| item.output.as_ref().is_some_and(|output| output.id() == id))
        else {
            return;
        };
        match event {
            wl_output::Event::Geometry {
                x,
                y,
                make: _,
                model: _,
                transform,
                ..
            } => {
                output.x = x;
                output.y = y;
                output.rotated = matches!(
                    transform,
                    WEnum::Value(Transform::_90 | Transform::Flipped90 | Transform::Flipped270)
                );
            }
            wl_output::Event::Mode {
                flags: WEnum::Value(flags),
                width,
                height,
                ..
            } if flags.contains(wl_output::Mode::Current) => {
                output.width = u32::try_from(width).unwrap_or_default();
                output.height = u32::try_from(height).unwrap_or_default();
            }
            wl_output::Event::Scale { factor } => {
                output.scale = u32::try_from(factor).unwrap_or(1).max(1);
            }
            wl_output::Event::Name { name } => output.name = Some(name),
            wl_output::Event::Done => {
                output.done = true;
                if output.version >= 3
                    && let Some(proxy) = output.output.take()
                {
                    proxy.release();
                }
            }
            _ => {}
        }
    }
}
