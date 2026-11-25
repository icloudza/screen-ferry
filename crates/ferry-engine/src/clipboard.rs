//! 剪贴板同步：在数据连接上运行，服务端与客户端共用。数据连接上的文件控制消息也在这里收发，
//! 交给 [`FileLink`]。
//!
//! 同一时刻只有一台是“活动端”（光标所在的那台）。规则只有两条：
//!
//! - **通告**：活动端轮询本机剪贴板，变了就发 `ClipboardOffer`，只说有哪些格式、多大；
//!   成为非活动端的那一刻再检查一次，刚复制完就切走也不会漏。
//! - **拉取**：手上有对方未拉取的通告、且本机是活动端时，立即拉取并写入本机剪贴板。
//!
//! 所以复制多次只传最后一次；光标不过去，对方的剪贴板就不会被覆盖。
//! 例外是刚连上时：服务端先通告一次当前内容，见 [`Start`]。
//! 写入后记下剪贴板的变化计数，自己写入的内容不会再被当作新复制通告回去。

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ferry_net::Framed;
use ferry_platform::Clipboard;
use ferry_proto::limits::MAX_CHUNK_LEN;
use ferry_proto::{
    Cancel, CancelReason, Chunk, ClipboardItem, ClipboardOffer, ClipboardRequest, Message,
};
use ferry_trust::Fingerprint;
use ferry_types::{ClipboardContent, ClipboardFormat};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio::time::{MissedTickBehavior, interval};
use tracing::{debug, info, warn};

use crate::error::{Error, Result};
use crate::events::{ClipSummary, Event, Events};
use crate::files::FileLink;
use crate::link::send;

/// 活动端检查本机剪贴板的间隔。
const POLL_INTERVAL: Duration = Duration::from_millis(250);

/// 单个格式缺省的大小上限：32 MB。
pub const DEFAULT_CLIP_LIMIT: u64 = 32 * 1024 * 1024;

/// 剪贴板单个格式的大小上限，超过的不通告也不拉取。克隆后共用同一个值，会话中途也能改。
#[derive(Clone, Debug)]
pub struct ClipLimit(Arc<AtomicU64>);

impl ClipLimit {
    pub fn new(bytes: u64) -> Self {
        Self(Arc::new(AtomicU64::new(bytes)))
    }

    pub fn get(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }

    pub fn set(&self, bytes: u64) {
        self.0.store(bytes, Ordering::Relaxed);
    }
}

impl Default for ClipLimit {
    fn default() -> Self {
        Self::new(DEFAULT_CLIP_LIMIT)
    }
}

/// 服务端与客户端各持有一份，会话之间复用。
pub(crate) type SharedClipboard<C> = Arc<Mutex<C>>;

#[derive(Clone)]
struct ClipboardSession {
    generation: Arc<AtomicU64>,
    write_gate: Arc<Mutex<()>>,
}

impl ClipboardSession {
    fn invalidate(&self) {
        let _gate = self.write_gate.lock().unwrap_or_else(|e| e.into_inner());
        self.generation.fetch_add(1, Ordering::SeqCst);
    }
}

/// 正在运行的同步任务，丢弃即停止。
pub(crate) struct ClipboardTask {
    handle: JoinHandle<()>,
    session: ClipboardSession,
}

impl Drop for ClipboardTask {
    fn drop(&mut self) {
        self.session.invalidate();
        self.handle.abort();
    }
}

/// 连上时谁先通告本机剪贴板。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Start {
    /// 会话开始时光标总在服务端，它的剪贴板就是用户眼下的剪贴板，连上就通告。
    Announce,
    /// 客户端的旧内容只记下计数，免得一连上就覆盖服务端。
    Quiet,
}

/// 和哪台设备同步，结果报告给谁，单项多大以内才同步。
pub(crate) struct Reporter {
    pub(crate) peer: Fingerprint,
    pub(crate) events: Events,
    pub(crate) limit: ClipLimit,
}

impl Reporter {
    fn skipped(&self, to_peer: bool, format: ClipboardFormat, bytes: u64, limit: u64) {
        self.events.emit(Event::ClipboardSkipped {
            peer: self.peer,
            to_peer,
            format,
            bytes,
            limit,
        });
    }
}

/// 在数据连接上启动剪贴板同步。`active` 表示对这台设备而言本机是不是活动端：
/// 客户端上是光标是否在本机，服务端上是光标是否不在这台设备上。
///
/// 数据连接比输入连接晚建立，期间光标可能已经切走，
/// 所以连上时是否通告由 `start` 决定，而不看这一刻谁是活动端。
/// `sync` 为假（没有协商剪贴板能力）时只收发文件控制消息。
pub(crate) fn spawn<S, C>(
    framed: Framed<S>,
    clipboard: SharedClipboard<C>,
    active: watch::Receiver<bool>,
    start: Start,
    report: Reporter,
    sync: bool,
    files: Option<FileLink>,
) -> ClipboardTask
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    C: Clipboard,
{
    let session = ClipboardSession {
        generation: Arc::new(AtomicU64::new(0)),
        write_gate: Arc::new(Mutex::new(())),
    };
    let task_session = session.clone();
    ClipboardTask {
        handle: tokio::spawn(async move {
            let mut sync =
                ClipboardSync::new(framed, clipboard, active, report, sync, files, task_session);
            match sync.run(start).await {
                Ok(()) => debug!("数据连接已关闭"),
                Err(e) => warn!("剪贴板同步中断：{e}"),
            }
        }),
        session,
    }
}

struct ClipboardSync<S, C> {
    framed: Framed<S>,
    clipboard: SharedClipboard<C>,
    active: watch::Receiver<bool>,
    /// 最近一次看到的本机剪贴板变化计数。
    seen: u64,
    /// 最近一次通告出去的本机内容，等对方来拉取。拉走一种格式就放掉一种，大图片不长期占着内存。
    offered: Option<(u32, ClipboardContent)>,
    next_seq: u32,
    /// 对方最近一次通告、还没拉取的内容。
    offer: Option<ClipboardOffer>,
    fetch: Option<Fetch>,
    next_transfer: u32,
    report: Reporter,
    /// 最近一次报告“已被对方拉走”的通告序号，同一份内容的几个格式只报告一次。
    reported: Option<u32>,
    /// 是否同步剪贴板。
    sync: bool,
    /// 文件传输的控制消息。
    files: Option<FileLink>,
    /// 会话状态；任务丢弃后让已排队的写入失效。
    session: ClipboardSession,
}

/// 正在拉取的一份对方内容。各格式都到齐后一次写入本机剪贴板。
struct Fetch {
    seq: u32,
    /// 传输号 → 格式、通告的大小、已收到的数据。
    pending: HashMap<u32, (ClipboardFormat, u64, Vec<u8>)>,
    done: ClipboardContent,
}

impl<S, C> ClipboardSync<S, C>
where
    S: AsyncRead + AsyncWrite + Unpin,
    C: Clipboard,
{
    fn new(
        framed: Framed<S>,
        clipboard: SharedClipboard<C>,
        active: watch::Receiver<bool>,
        report: Reporter,
        sync: bool,
        files: Option<FileLink>,
        session: ClipboardSession,
    ) -> Self {
        Self {
            framed,
            clipboard,
            active,
            seen: 0,
            offered: None,
            next_seq: 1,
            offer: None,
            fetch: None,
            next_transfer: 1,
            report,
            reported: None,
            sync,
            files,
            session,
        }
    }

    async fn run(&mut self, start: Start) -> Result<()> {
        self.seen = Self::change_count(Arc::clone(&self.clipboard)).await;
        if self.sync && start == Start::Announce {
            self.offer_local().await?;
        }

        let mut tick = interval(POLL_INTERVAL);
        tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                message = self.framed.recv() => match message? {
                    None | Some(Message::Bye(_)) => return Ok(()),
                    Some(message) => self.on_message(message).await?,
                },
                changed = self.active.changed() => {
                    if changed.is_err() {
                        return Ok(()); // 会话已结束
                    }
                    if !self.sync {
                        continue;
                    }
                    if self.is_active() {
                        self.start_fetch().await?;
                    } else {
                        self.poll().await?;
                    }
                }
                _ = tick.tick() => {
                    if self.sync && self.is_active() {
                        self.poll().await?;
                    }
                }
                Some(message) = next_file_message(&mut self.files) => {
                    send(&mut self.framed, &message).await?;
                }
            }
        }
    }

    async fn on_message(&mut self, message: Message) -> Result<()> {
        match message {
            Message::Ping { nonce } => send(&mut self.framed, &Message::Pong { nonce }).await?,
            Message::ClipboardOffer(offer) => {
                debug!(seq = offer.seq, items = ?offer.items, "对方剪贴板有新内容");
                // 新通告让正在拉取的旧内容作废；迟到的旧分块按未知传输号丢弃。
                self.fetch = None;
                self.offer = Some(offer);
                if self.is_active() {
                    self.start_fetch().await?;
                }
            }
            Message::ClipboardRequest(request) => self.serve(request).await?,
            Message::Chunk(chunk) => self.on_chunk(chunk).await?,
            Message::Cancel(cancel) => {
                if let Some(fetch) = self.fetch.as_mut()
                    && fetch.pending.remove(&cancel.transfer).is_some()
                {
                    debug!(reason = cancel.reason.0, "对方取消了一个格式的传输");
                    self.finish_if_done().await?;
                }
            }
            message @ (Message::FileOffer(_)
            | Message::FileAccept { .. }
            | Message::FileCancel(_)) => {
                if let Some(files) = &self.files {
                    files.on_control(message);
                }
            }
            other => debug!(
                kind = format_args!("{:#06x}", other.type_id()),
                "数据连接上忽略消息"
            ),
        }
        Ok(())
    }

    /// 本机剪贴板变了就通告。
    async fn poll(&mut self) -> Result<()> {
        if Self::change_count(Arc::clone(&self.clipboard)).await != self.seen {
            self.offer_local().await?;
        }
        Ok(())
    }

    async fn offer_local(&mut self) -> Result<()> {
        let clipboard = Arc::clone(&self.clipboard);
        let (count, content) = tokio::task::spawn_blocking(move || {
            let mut clipboard = clipboard.lock().unwrap_or_else(|e| e.into_inner());
            let count = clipboard.change_count();
            (count, clipboard.read())
        })
        .await
        .expect("剪贴板读取任务意外退出");
        self.seen = count;
        let content = content.unwrap_or_else(|e| {
            warn!("读取本机剪贴板失败：{e}");
            ClipboardContent::new()
        });

        let seq = self.next_seq;
        self.next_seq = self.next_seq.wrapping_add(1);
        // 太大的格式不通告，也不留在内存里。
        let limit = self.report.limit.get();
        let mut content = content;
        let too_large: Vec<(ClipboardFormat, u64)> = content
            .iter()
            .map(|(format, data)| (format, data.len() as u64))
            .filter(|&(_, size)| size > limit)
            .collect();
        for &(format, size) in &too_large {
            content.remove(format);
            info!(?format, size, limit, "剪贴板内容超过大小上限，没有通告");
            self.report.skipped(true, format, size, limit);
        }
        // 没有可共享的格式也照样通告（空列表），让对方丢掉之前的旧通告。
        let items = content
            .iter()
            .map(|(format, data)| ClipboardItem {
                format,
                size: data.len() as u64,
            })
            .collect();
        let offer = ClipboardOffer { seq, items };
        debug!(seq, items = ?offer.items, "通告本机剪贴板");
        self.offered = Some((seq, content));
        send(&mut self.framed, &Message::ClipboardOffer(offer)).await
    }

    /// 拉取对方通告中本机支持的格式。
    async fn start_fetch(&mut self) -> Result<()> {
        if self.fetch.is_some() {
            return Ok(());
        }
        let Some(offer) = self.offer.take() else {
            return Ok(());
        };
        let clipboard = Arc::clone(&self.clipboard);
        let supported = tokio::task::spawn_blocking(move || {
            clipboard
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .formats()
        })
        .await
        .expect("剪贴板格式查询任务意外退出");
        let limit = self.report.limit.get();
        let mut fetch = Fetch {
            seq: offer.seq,
            pending: HashMap::new(),
            done: ClipboardContent::new(),
        };
        for item in offer.items {
            if !supported.contains(&item.format) {
                continue;
            }
            if item.size > limit {
                info!(format = ?item.format, size = item.size, limit, "对方的剪贴板内容超过大小上限，没有拉取");
                self.report.skipped(false, item.format, item.size, limit);
                continue;
            }
            let transfer = self.next_transfer;
            self.next_transfer = self.next_transfer.wrapping_add(1);
            let request = ClipboardRequest {
                seq: offer.seq,
                format: item.format,
                transfer,
            };
            send(&mut self.framed, &Message::ClipboardRequest(request)).await?;
            fetch
                .pending
                .insert(transfer, (item.format, item.size, Vec::new()));
        }
        if !fetch.pending.is_empty() {
            self.fetch = Some(fetch);
        }
        Ok(())
    }

    /// 回应对方的拉取：按块发出内容，发完就放掉这种格式。过期的请求回 `Cancel(STALE)`。
    async fn serve(&mut self, request: ClipboardRequest) -> Result<()> {
        let current = matches!(&self.offered, Some((seq, _)) if *seq == request.seq);
        if current
            && self.reported != Some(request.seq)
            && let Some((_, content)) = &self.offered
        {
            // 对方一次请求全部格式，第一个请求到的时候内容还是完整的。
            self.reported = Some(request.seq);
            self.report.events.emit(Event::Clipboard {
                peer: self.report.peer,
                to_peer: true,
                summary: ClipSummary::of(content),
            });
        }
        let data = match &mut self.offered {
            Some((_, content)) if current => content.remove(request.format),
            _ => None,
        };
        let Some(data) = data else {
            let cancel = Cancel {
                transfer: request.transfer,
                reason: CancelReason::STALE,
            };
            return send(&mut self.framed, &Message::Cancel(cancel)).await;
        };

        // 空内容也要发一块 last = true，对方才知道传完了。
        let count = data.len().div_ceil(MAX_CHUNK_LEN).max(1);
        for (i, part) in data
            .chunks(MAX_CHUNK_LEN)
            .chain(data.is_empty().then_some(&[][..]))
            .enumerate()
        {
            let chunk = Chunk {
                transfer: request.transfer,
                last: i + 1 == count,
                data: part.to_vec(),
            };
            send(&mut self.framed, &Message::Chunk(chunk)).await?;
        }
        Ok(())
    }

    async fn on_chunk(&mut self, chunk: Chunk) -> Result<()> {
        let Some(fetch) = self.fetch.as_mut() else {
            return Ok(());
        };
        let Some((_, size, data)) = fetch.pending.get_mut(&chunk.transfer) else {
            return Ok(()); // 已作废的传输
        };
        data.extend_from_slice(&chunk.data);
        if data.len() as u64 > *size {
            return Err(Error::Protocol("剪贴板内容超过了通告的大小"));
        }
        if chunk.last {
            let (format, _, data) = fetch.pending.remove(&chunk.transfer).expect("刚刚查到过");
            fetch.done.insert(format, data);
            self.finish_if_done().await?;
        }
        Ok(())
    }

    /// 所有格式都到齐（或被取消）后写入本机剪贴板。
    async fn finish_if_done(&mut self) -> Result<()> {
        if self.fetch.as_ref().is_some_and(|f| !f.pending.is_empty()) {
            return Ok(());
        }
        let Some(fetch) = self.fetch.take() else {
            return Ok(());
        };
        if fetch.done.is_empty() {
            return Ok(());
        }
        let content = fetch.done;
        let bytes = content.total_len();
        let summary = ClipSummary::of(&content);
        let clipboard = Arc::clone(&self.clipboard);
        let session = self.session.clone();
        let expected = session.generation.load(Ordering::SeqCst);
        let written = tokio::task::spawn_blocking(move || {
            let _gate = session.write_gate.lock().unwrap_or_else(|e| e.into_inner());
            let mut clipboard = clipboard.lock().unwrap_or_else(|e| e.into_inner());
            if session.generation.load(Ordering::SeqCst) != expected {
                return Ok(None);
            }
            clipboard.write(&content).map(Some)
        })
        .await
        .expect("剪贴板写入任务意外退出");
        match written {
            Ok(Some(count)) => {
                self.seen = count;
                info!(seq = fetch.seq, bytes, "已同步对方的剪贴板");
                self.report.events.emit(Event::Clipboard {
                    peer: self.report.peer,
                    to_peer: false,
                    summary,
                });
            }
            Ok(None) => {}
            Err(e) => warn!("写入本机剪贴板失败：{e}"),
        }
        Ok(())
    }

    async fn change_count(clipboard: SharedClipboard<C>) -> u64 {
        tokio::task::spawn_blocking(move || {
            clipboard
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .change_count()
        })
        .await
        .expect("剪贴板计数任务意外退出")
    }

    fn is_active(&self) -> bool {
        *self.active.borrow()
    }
}

/// 下一条要发出的文件控制消息；没有文件传输时永远等待。
async fn next_file_message(files: &mut Option<FileLink>) -> Option<Message> {
    match files {
        Some(files) => files.next().await,
        None => std::future::pending().await,
    }
}
