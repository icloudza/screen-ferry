//! 基于 wl-clipboard 的 Wayland 剪贴板。

use std::io::{Read, Write};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

use ferry_platform::{Clipboard, Error, Result};
use ferry_types::{ClipboardContent, ClipboardFormat};

const FORMATS: &[ClipboardFormat] = &[
    ClipboardFormat::TEXT,
    ClipboardFormat::HTML,
    ClipboardFormat::RTF,
    ClipboardFormat::PNG,
];
const TEXT_MIMES: &[&str] = &["text/plain;charset=utf-8", "UTF8_STRING", "text/plain"];
const HTML_MIMES: &[&str] = &["text/html;charset=utf-8", "text/html"];
const RTF_MIMES: &[&str] = &["application/rtf", "text/rtf", "text/richtext"];
const PNG_MIMES: &[&str] = &["image/png"];
const POLL_INTERVAL: Duration = Duration::from_millis(150);
const COMMAND_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_OUTPUT: usize = 64 * 1024 * 1024;
const MAX_ERROR_OUTPUT: usize = 8 * 1024;

#[derive(Default)]
struct WatchState {
    content: ClipboardContent,
    initialized: bool,
    stopped: bool,
}

/// Wayland 剪贴板；首次调用时才启动后台监听。
pub struct LinuxClipboard {
    state: Arc<Mutex<WatchState>>,
    count: Arc<AtomicU64>,
    operation: Arc<Mutex<()>>,
    watching: bool,
}

impl LinuxClipboard {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(WatchState::default())),
            count: Arc::new(AtomicU64::new(0)),
            operation: Arc::new(Mutex::new(())),
            watching: false,
        }
    }

    fn ensure_watcher(&mut self) -> Result<()> {
        if self.watching {
            return Ok(());
        }
        let content = {
            let _operation = self.operation.lock().unwrap_or_else(|e| e.into_inner());
            read_clipboard()?
        };
        store_content(&mut self.lock(), &self.count, content, false);
        let state = Arc::downgrade(&self.state);
        let count = Arc::clone(&self.count);
        let operation = Arc::clone(&self.operation);
        thread::Builder::new()
            .name("screenferry-clipboard".into())
            .spawn(move || {
                loop {
                    let Some(state) = state.upgrade() else {
                        return;
                    };
                    {
                        let state = state.lock().unwrap_or_else(|e| e.into_inner());
                        if state.stopped {
                            return;
                        }
                    }
                    if let Ok(content) = {
                        let _operation = operation.lock().unwrap_or_else(|e| e.into_inner());
                        read_clipboard()
                    } {
                        let mut state = state.lock().unwrap_or_else(|e| e.into_inner());
                        if state.stopped {
                            return;
                        }
                        store_content(&mut state, &count, content, false);
                    }
                    thread::sleep(POLL_INTERVAL);
                }
            })
            .map_err(|e| io_error("spawn clipboard watcher", e))?;
        self.watching = true;
        Ok(())
    }

    fn lock(&self) -> MutexGuard<'_, WatchState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl Default for LinuxClipboard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for LinuxClipboard {
    fn drop(&mut self) {
        self.lock().stopped = true;
    }
}

impl Clipboard for LinuxClipboard {
    fn formats(&self) -> &'static [ClipboardFormat] {
        FORMATS
    }

    fn change_count(&mut self) -> u64 {
        if let Err(error) = self.ensure_watcher() {
            eprintln!("启动剪贴板监听失败：{error}");
        }
        self.count.load(Ordering::Acquire)
    }

    fn read(&mut self) -> Result<ClipboardContent> {
        self.ensure_watcher()?;
        let _operation = self.operation.lock().unwrap_or_else(|e| e.into_inner());
        let content = read_clipboard()?;
        store_content(&mut self.lock(), &self.count, content.clone(), false);
        Ok(content)
    }

    fn write(&mut self, content: &ClipboardContent) -> Result<u64> {
        self.ensure_watcher()?;
        let _operation = self.operation.lock().unwrap_or_else(|e| e.into_inner());
        let (format, data) = content
            .iter()
            .max_by_key(|(format, _)| format_priority(*format))
            .ok_or(Error::Unsupported("剪贴板内容没有支持的格式"))?;
        let mime = mime_for(format)
            .and_then(|mimes| mimes.first().copied())
            .unwrap();
        let output = run_command("wl-copy", &["--type", mime], Some(data))?;
        if !output.status.success() {
            return Err(command_error("wl-copy", &output));
        }

        let mut written = ClipboardContent::new();
        written.insert(format, data.to_vec());
        store_content(&mut self.lock(), &self.count, written, true);
        Ok(self.count.load(Ordering::Acquire))
    }
}

fn read_clipboard() -> Result<ClipboardContent> {
    let listed = run_command("wl-paste", &["--list-types"], None)?;
    if !listed.status.success() {
        let detail = String::from_utf8_lossy(&listed.stderr);
        if detail.contains("Nothing is copied") || detail.contains("No selection") {
            return Ok(ClipboardContent::new());
        }
        return Err(command_error("wl-paste --list", &listed));
    }
    let list = String::from_utf8_lossy(&listed.stdout);
    let available: Vec<&str> = list.split_whitespace().collect();
    let mut content = ClipboardContent::new();
    for &format in FORMATS {
        let Some(mime) = mime_for(format)
            .and_then(|mimes| mimes.iter().copied().find(|mime| available.contains(mime)))
        else {
            continue;
        };
        let pasted = run_command("wl-paste", &["--type", mime, "--no-newline"], None)?;
        if !pasted.status.success() {
            let detail = String::from_utf8_lossy(&pasted.stderr);
            if detail.contains("Nothing is copied") || detail.contains("No selection") {
                return Ok(ClipboardContent::new());
            }
            return Err(command_error("wl-paste", &pasted));
        }
        content.insert(format, pasted.stdout);
    }
    Ok(content)
}

fn store_content(
    state: &mut WatchState,
    count: &AtomicU64,
    content: ClipboardContent,
    force_change: bool,
) {
    if force_change || (state.initialized && state.content != content) {
        let _ = count.fetch_update(Ordering::Release, Ordering::Relaxed, |value| {
            Some(value.saturating_add(1))
        });
    }
    state.content = content;
    state.initialized = true;
}

fn run_command(program: &str, args: &[&str], input: Option<&[u8]>) -> Result<Output> {
    let call = program_name(program);
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|e| io_error(call, e))?;
    let writer = input.map(|data| {
        let data = data.to_vec();
        let mut stdin = child.stdin.take().expect("piped stdin");
        thread::spawn(move || stdin.write_all(&data))
    });
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let stdout_reader = thread::spawn(move || read_limited(stdout, MAX_OUTPUT));
    let stderr_reader = thread::spawn(move || read_limited(stderr, MAX_ERROR_OUTPUT));

    let deadline = Instant::now() + COMMAND_TIMEOUT;
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| io_error(call, e))? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            if let Some(writer) = writer {
                let _ = writer.join();
            }
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            return Err(Error::Os {
                call,
                detail: format!("命令超过 {} 秒", COMMAND_TIMEOUT.as_secs()),
            });
        }
        thread::sleep(Duration::from_millis(10));
    };
    let (stdout, stdout_truncated) = stdout_reader.join().expect("stdout reader");
    let (stderr, _) = stderr_reader.join().expect("stderr reader");
    if let Some(writer) = writer {
        writer
            .join()
            .expect("stdin writer")
            .map_err(|e| io_error(call, e))?;
    }
    if stdout_truncated {
        return Err(Error::Os {
            call,
            detail: format!("输出超过 {MAX_OUTPUT} 字节"),
        });
    }
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

fn read_limited(mut reader: impl Read, limit: usize) -> (Vec<u8>, bool) {
    let mut output = Vec::new();
    let mut truncated = false;
    let mut buffer = [0u8; 8192];
    loop {
        let Ok(read) = reader.read(&mut buffer) else {
            break;
        };
        if read == 0 {
            break;
        }
        if output.len() + read > limit {
            truncated = true;
        }
        if output.len() < limit {
            let take = (limit - output.len()).min(read);
            output.extend_from_slice(&buffer[..take]);
        }
    }
    (output, truncated)
}

fn format_priority(format: ClipboardFormat) -> u8 {
    match format {
        ClipboardFormat::PNG => 4,
        ClipboardFormat::HTML => 3,
        ClipboardFormat::RTF => 2,
        ClipboardFormat::TEXT => 1,
        _ => 0,
    }
}

fn mime_for(format: ClipboardFormat) -> Option<&'static [&'static str]> {
    match format {
        ClipboardFormat::TEXT => Some(TEXT_MIMES),
        ClipboardFormat::HTML => Some(HTML_MIMES),
        ClipboardFormat::RTF => Some(RTF_MIMES),
        ClipboardFormat::PNG => Some(PNG_MIMES),
        _ => None,
    }
}

fn command_error(call: &'static str, output: &Output) -> Error {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stderr = stderr.trim();
    let detail = if stderr.is_empty() {
        format!("退出状态 {}", output.status)
    } else {
        format!("退出状态 {}：{stderr}", output.status)
    };
    Error::Os { call, detail }
}

fn program_name(program: &str) -> &'static str {
    match program {
        "wl-copy" => "wl-copy",
        "wl-paste" => "wl-paste",
        _ => "clipboard watcher",
    }
}

fn io_error(call: &'static str, error: std::io::Error) -> Error {
    Error::Os {
        call,
        detail: error.to_string(),
    }
}
