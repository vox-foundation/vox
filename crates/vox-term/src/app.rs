//! Application state + main event loop.
//!
//! `run_with_args()` is the async entry-point for the TUI (awaited directly by `vox-cli`),
//! while `run()` wraps it in a Tokio runtime for standalone `vox-term`. It:
//! 1. Tries to enter raw mode (degrades gracefully under TERM=dumb/headless).
//! 2. Resolves `SessionHost` (`DaemonHost` when reachable, `LocalHost` fallback or `--local`)
//!    and opens or re-attaches to a live session (`--attach [SESSION_ID]`).
//! 3. Runs the crossterm event loop: key events → `InputBox` / raw VT input → `SessionHost`.
//! 4. Drains `OutputFrame`s + `SessionEvent`s each tick with `after_seq` lag recovery.
//! 5. Redraws the ratatui frame on each tick (`ALT_SCREEN` pass-through or structured blocks).

use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use std::io::{self, BufRead, IsTerminal};
use tokio::sync::broadcast::error::TryRecvError;

use vox_terminal_core::{
    Attachment, Block, BlockKind, BlockStatus, OpenSpec, SessionEvent, SessionHost, SessionInfo,
    TermSignal, daemon_host::resolve_session_host,
};

use crate::{
    term_setup::TermSetup,
    theme::is_dumb_terminal,
    ui::{blocks, input::InputBox},
    vt::VtGrid,
};

/// Parsed CLI flags for `vox-term` (`--attach [session_id]`, `--local`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TermCliArgs {
    pub attach: Option<String>,
    pub force_local: bool,
}

impl TermCliArgs {
    pub fn parse_iter<I, S>(args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut parsed = Self::default();
        let mut iter = args.into_iter().skip(1).peekable();
        while let Some(arg) = iter.next() {
            let s = arg.as_ref();
            if s == "--local" {
                parsed.force_local = true;
            } else if s == "--attach" {
                let is_bare = match iter.peek() {
                    None => true,
                    Some(next) => next.as_ref().starts_with("--"),
                };
                if is_bare {
                    parsed.attach = Some("latest".to_string());
                } else if let Some(next) = iter.next() {
                    let val = next.as_ref();
                    parsed.attach = Some(if val.is_empty() {
                        "latest".to_string()
                    } else {
                        val.to_string()
                    });
                }
            } else if let Some(rest) = s.strip_prefix("--attach=") {
                parsed.attach = Some(if rest.is_empty() {
                    "latest".to_string()
                } else {
                    rest.to_string()
                });
            }
        }
        parsed
    }

    pub fn from_env() -> Self {
        Self::parse_iter(std::env::args())
    }
}

/// Resolve the active [`SessionHost`]: connects to `vox-orchestrator-d` when reachable
/// (unless `force_local` is true) and falls back to an in-process `LocalHost`.
pub async fn resolve_host(force_local: bool) -> std::sync::Arc<dyn SessionHost> {
    resolve_session_host(force_local).await
}

/// Select the newest session ID, ordering `term-{n}` sessions by numeric suffix (`10 > 9`)
/// rather than lexicographically (`"term-9" > "term-10"`).
pub fn pick_latest_session_id(sessions: &[SessionInfo]) -> Option<String> {
    sessions
        .iter()
        .max_by(|a, b| {
            let key_a = (
                a.id.strip_prefix("term-")
                    .and_then(|n| n.parse::<u64>().ok()),
                a.id.as_str(),
            );
            let key_b = (
                b.id.strip_prefix("term-")
                    .and_then(|n| n.parse::<u64>().ok()),
                b.id.as_str(),
            );
            key_a.cmp(&key_b)
        })
        .map(|s| s.id.clone())
}

/// Attach to an existing session (`Some("latest")` or `Some(id)`) or open and attach to a fresh
/// session (`None`) on `host`.
pub async fn attach_or_open_session(
    host: &dyn SessionHost,
    attach: Option<&str>,
    cols: u16,
    rows: u16,
) -> Result<(String, Attachment)> {
    match attach {
        None => {
            let sid = host
                .open(OpenSpec {
                    cols,
                    rows,
                    shell: None,
                })
                .await?;
            let att = host.attach(&sid, None).await?;
            Ok((sid, att))
        }
        Some("latest") => {
            let sessions = host.list().await?;
            let Some(sid) = pick_latest_session_id(&sessions) else {
                anyhow::bail!(
                    "no active terminal sessions to attach to; run `vox term` without --attach to open one"
                );
            };
            let att = host
                .attach(&sid, None)
                .await
                .with_context(|| format!("failed to attach to terminal session '{sid}'"))?;
            Ok((sid, att))
        }
        Some(explicit) => {
            let att = host
                .attach(explicit, None)
                .await
                .with_context(|| format!("failed to attach to terminal session '{explicit}'"))?;
            Ok((explicit.to_string(), att))
        }
    }
}

/// Apply one [`SessionEvent`] onto `vox-term`'s local view state completely in-place
/// (zero `host.attach` RPC round-trips on normal block lifecycles, preventing output
/// duplication and orphaned daemon subscription streams).
pub async fn apply_session_event(
    _host: &dyn SessionHost,
    _sid: &str,
    _last_seq: u64,
    blocks_state: &mut Vec<Block>,
    agent_lines: &mut Vec<String>,
    alt_screen: &mut bool,
    ev: SessionEvent,
) {
    match ev {
        SessionEvent::OutputAppended { id, chunk } => {
            if let Some(block) = blocks_state.iter_mut().find(|b| b.id == id) {
                block.push(chunk);
            }
        }
        SessionEvent::BlockOpened {
            id,
            kind,
            input,
            origin,
            cwd,
            git_branch,
        } => {
            if let Some(existing) = blocks_state.iter_mut().find(|b| b.id == id) {
                if existing.input.is_empty() && !input.is_empty() {
                    existing.input = input;
                }
                if let Some(k) = kind {
                    existing.kind = k;
                }
                if let Some(o) = origin {
                    existing.origin = o;
                }
                if existing.cwd.is_none() {
                    existing.cwd = cwd;
                }
                if existing.git_branch.is_none() {
                    existing.git_branch = git_branch;
                }
            } else {
                let mut block = Block::new(id, kind.unwrap_or(BlockKind::Shell), input);
                if let Some(o) = origin {
                    block = block.with_origin(o);
                }
                block.cwd = cwd;
                block.git_branch = git_branch;
                blocks_state.push(block);
            }
        }
        SessionEvent::BlockClosed {
            id,
            exit_code,
            status,
        } => {
            if let Some(existing) = blocks_state.iter_mut().find(|b| b.id == id) {
                match status {
                    Some(BlockStatus::Cancelled) => {
                        existing.cancel();
                        if let Some(code) = exit_code {
                            existing.exit_code = Some(code);
                        }
                    }
                    Some(st) => {
                        existing.finish(exit_code.unwrap_or(if st == BlockStatus::Ok {
                            0
                        } else {
                            1
                        }));
                        existing.status = st;
                        existing.exit_code = exit_code;
                    }
                    None => {
                        existing.finish(exit_code.unwrap_or(0));
                    }
                }
            }
        }
        SessionEvent::AgentMessage { text } => {
            agent_lines.push(text);
        }
        SessionEvent::AltScreenChanged { active } => {
            *alt_screen = active;
        }
        SessionEvent::CwdChanged { .. } => {}
        SessionEvent::BlockAwaitingInput {
            id,
            credential_prompt,
            ..
        } => {
            if let Some(existing) = blocks_state.iter_mut().find(|b| b.id == id) {
                existing.status = BlockStatus::AwaitingInput;
                existing.credential_prompt = credential_prompt;
            }
        }
        SessionEvent::BlockPinned { id, pinned } => {
            if let Some(existing) = blocks_state.iter_mut().find(|b| b.id == id) {
                existing.pinned = pinned;
            }
        }
        SessionEvent::SecurityLevelChanged { .. } | SessionEvent::ControlModeChanged { .. } => {}
        SessionEvent::SnapshotResync {
            blocks,
            alt_screen: snap_alt,
            ..
        } => {
            *blocks_state = blocks;
            *alt_screen = snap_alt;
        }
    }
}

/// True when crossterm/ratatui must not be initialized: `TERM=dumb`, or
/// stdin/stdout is not a tty (piped input, redirected output, CI). Checked
/// *before* touching crossterm — its input reader panics/errors when spun up
/// against a non-tty stdin, and raw-mode escapes still land on a redirected
/// stdout even when raw mode itself fails.
fn is_headless() -> bool {
    is_dumb_terminal() || !io::stdin().is_terminal() || !io::stdout().is_terminal()
}

/// Plain-text line loop for headless/dumb terminals: no crossterm, no ANSI
/// escapes. Reads lines from stdin until EOF and exits cleanly.
fn run_plain() -> Result<()> {
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let _line = line?;
        // Headless mode has no interactive block/agent UI to draw; it exists
        // so piped input (CI, `TERM=dumb`) doesn't crash. Intent dispatch for
        // plain mode is out of scope here (tracked separately).
    }
    Ok(())
}

/// Translate a crossterm [`KeyEvent`] into raw VT input bytes, ignoring key-release events.
pub fn key_event_to_vt_bytes(key: &KeyEvent) -> Option<Vec<u8>> {
    if key.kind == KeyEventKind::Release {
        return None;
    }
    if key.code == KeyCode::Enter {
        if key.modifiers.contains(KeyModifiers::SHIFT) {
            return Some(b"\x1b[13;2u".to_vec());
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return Some(b"\x1b[13;5u".to_vec());
        }
    }
    if key.modifiers.contains(KeyModifiers::CONTROL)
        && let KeyCode::Char(c) = key.code
        && c.is_ascii_alphabetic()
    {
        return Some(vec![(c.to_ascii_lowercase() as u8) - b'a' + 1]);
    }
    match key.code {
        KeyCode::Char(c) => {
            let mut buf = [0u8; 4];
            Some(c.encode_utf8(&mut buf).as_bytes().to_vec())
        }
        KeyCode::Enter => Some(b"\r".to_vec()),
        KeyCode::Backspace => Some(vec![0x7f]),
        KeyCode::Tab => Some(b"\t".to_vec()),
        KeyCode::BackTab => Some(b"\x1b[Z".to_vec()),
        KeyCode::Esc => Some(vec![0x1b]),
        KeyCode::Up => Some(b"\x1b[A".to_vec()),
        KeyCode::Down => Some(b"\x1b[B".to_vec()),
        KeyCode::Right => Some(b"\x1b[C".to_vec()),
        KeyCode::Left => Some(b"\x1b[D".to_vec()),
        KeyCode::Home => Some(b"\x1b[H".to_vec()),
        KeyCode::End => Some(b"\x1b[F".to_vec()),
        KeyCode::PageUp => Some(b"\x1b[5~".to_vec()),
        KeyCode::PageDown => Some(b"\x1b[6~".to_vec()),
        KeyCode::Delete => Some(b"\x1b[3~".to_vec()),
        _ => None,
    }
}

pub async fn run_plain_with_host(host: &dyn SessionHost, sid: &str) -> Result<()> {
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let raw = line?;
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        let _ = host.submit(sid, trimmed).await?;
    }
    Ok(())
}

/// Async entry-point for `vox term` / `vox-term` driving a [`SessionHost`] session.
pub async fn run_with_args(args: TermCliArgs) -> Result<()> {
    if is_headless() {
        if args.attach.is_some() {
            let host = resolve_host(args.force_local).await;
            let (sid, _att) =
                attach_or_open_session(host.as_ref(), args.attach.as_deref(), 120, 32).await?;
            return run_plain_with_host(host.as_ref(), &sid).await;
        }
        return run_plain();
    }

    let (cols, rows) = crossterm::terminal::size().unwrap_or((120, 32));
    let host = resolve_host(args.force_local).await;
    let (sid, mut att) =
        attach_or_open_session(host.as_ref(), args.attach.as_deref(), cols, rows).await?;

    // We're on a real, non-dumb tty; enter raw mode + alternate screen.
    let _setup = TermSetup::new()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let mut input = InputBox::new();
    let mut grid = VtGrid::new(cols, rows);
    if !att.replay.is_empty() {
        grid.feed(&att.replay);
    }
    let mut last_seq = att.seq;
    let mut blocks_state = att.blocks;
    let mut alt_screen = att.alt_screen || grid.is_alt_screen();

    // Agent strip: accumulates streamed agent messages for display below the block list.
    let mut agent_lines: Vec<String> = Vec::new();

    loop {
        // Drain structured SessionEvents first so any `SnapshotResync { resync_required: true }`
        // resets `grid` before the accompanying replay `OutputFrame` is fed below.
        loop {
            match att.events.try_recv() {
                Ok(ev) => {
                    if matches!(
                        &ev,
                        SessionEvent::SnapshotResync {
                            resync_required: true,
                            ..
                        }
                    ) {
                        grid.reset();
                    }
                    apply_session_event(
                        host.as_ref(),
                        &sid,
                        last_seq,
                        &mut blocks_state,
                        &mut agent_lines,
                        &mut alt_screen,
                        ev,
                    )
                    .await;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Lagged(_)) => {
                    if let Ok(mut new_att) = host.attach(&sid, Some(last_seq)).await {
                        if new_att.resync_required {
                            grid.reset();
                        }
                        if !new_att.replay.is_empty() {
                            grid.feed(&new_att.replay);
                        }
                        blocks_state = std::mem::take(&mut new_att.blocks);
                        last_seq = new_att.seq;
                        alt_screen = new_att.alt_screen || grid.is_alt_screen();
                        att = new_att;
                    } else {
                        break;
                    }
                }
                Err(TryRecvError::Closed) => break,
            }
        }

        // Drain raw PTY output frames (non-blocking), recovering via after_seq on lag.
        loop {
            match att.output.try_recv() {
                Ok(frame) => {
                    grid.feed(&frame.bytes);
                    last_seq = frame.seq;
                    alt_screen = grid.is_alt_screen();
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Lagged(_)) => {
                    if let Ok(mut new_att) = host.attach(&sid, Some(last_seq)).await {
                        if new_att.resync_required {
                            grid.reset();
                        }
                        if !new_att.replay.is_empty() {
                            grid.feed(&new_att.replay);
                        }
                        blocks_state = std::mem::take(&mut new_att.blocks);
                        last_seq = new_att.seq;
                        alt_screen = new_att.alt_screen || grid.is_alt_screen();
                        att = new_att;
                    } else {
                        break;
                    }
                }
                Err(TryRecvError::Closed) => break,
            }
        }

        let in_alt = alt_screen || grid.is_alt_screen();

        // Draw
        terminal.draw(|frame| {
            let area = frame.area();
            if in_alt {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(1)])
                    .split(area);
                frame.render_widget(Paragraph::new(grid.render_lines()), chunks[0]);
                return;
            }

            // Layout: block list | agent strip (if any) | input bar
            let has_agent = !agent_lines.is_empty();
            let constraints = if has_agent {
                vec![
                    Constraint::Min(1),
                    Constraint::Length(3),
                    Constraint::Length(1),
                ]
            } else {
                vec![Constraint::Min(1), Constraint::Length(1)]
            };
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(constraints)
                .split(area);

            let mut block_grid = VtGrid::new(chunks[0].width.max(1), chunks[0].height.max(1));
            blocks::render(frame, chunks[0], &blocks_state, &mut block_grid);

            if has_agent {
                let tail: Vec<Line> = agent_lines
                    .iter()
                    .rev()
                    .take(3)
                    .rev()
                    .map(|t| {
                        Line::from(vec![
                            Span::styled("AI ", Style::default().fg(Color::Cyan)),
                            Span::raw(t.clone()),
                        ])
                    })
                    .collect();
                frame.render_widget(Paragraph::new(tail), chunks[1]);
                input.render(frame, chunks[2], input.active_mode());
            } else {
                input.render(frame, chunks[1], input.active_mode());
            }
        })?;

        // Poll for input events (50 ms tick = ~20 fps).
        // vox:defactored-from vox-config 2026-09-27
        const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(50);
        if !event::poll(POLL_INTERVAL)? {
            continue;
        }
        match event::read()? {
            Event::Resize(c, r) => {
                grid.resize(c, r);
                let _ = host.resize(&sid, c, r).await;
            }
            Event::Key(key) if key.kind == KeyEventKind::Release => {
                continue;
            }
            Event::Key(key) if alt_screen || grid.is_alt_screen() => {
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && matches!(key.code, KeyCode::Char('c' | 'C'))
                {
                    let _ = host.signal(&sid, TermSignal::Interrupt).await;
                } else if let Some(bytes) = key_event_to_vt_bytes(&key) {
                    let _ = host.input(&sid, &bytes).await;
                }
            }
            Event::Key(KeyEvent {
                code: KeyCode::Char('c' | 'C'),
                modifiers,
                ..
            }) if modifiers.contains(KeyModifiers::CONTROL) => {
                if blocks_state.last().is_some_and(|b| {
                    matches!(b.status, BlockStatus::Running | BlockStatus::AwaitingInput)
                }) {
                    let _ = host.signal(&sid, TermSignal::Interrupt).await;
                } else {
                    // Cleanly detach without calling host.close(&sid) so daemon sessions survive.
                    break;
                }
            }
            Event::Key(KeyEvent {
                code: KeyCode::Enter,
                ..
            }) => {
                let line = std::mem::take(&mut input.buf);
                input.ghost_suffix.clear();
                let is_active = blocks_state.last().is_some_and(|b| {
                    matches!(b.status, BlockStatus::Running | BlockStatus::AwaitingInput)
                });
                if !line.trim().is_empty() || is_active {
                    let _ = host.submit(&sid, &line).await;
                }
            }
            Event::Key(KeyEvent {
                code: KeyCode::Tab | KeyCode::Right,
                ..
            }) => {
                let _ = input.accept_ghost();
            }
            Event::Key(KeyEvent {
                code: KeyCode::Backspace,
                ..
            }) => {
                input.backspace();
                let history: Vec<String> = blocks_state.iter().map(|b| b.input.clone()).collect();
                input.update_ghost(&history);
            }
            Event::Key(KeyEvent {
                code: KeyCode::Char(c),
                ..
            }) => {
                input.push_char(c);
                let history: Vec<String> = blocks_state.iter().map(|b| b.input.clone()).collect();
                input.update_ghost(&history);
            }
            _ => {}
        }
    }

    Ok(())
}

/// Synchronous entry-point for standalone `vox-term` (`crates/vox-term/src/main.rs`).
pub fn run() -> Result<()> {
    let args = TermCliArgs::from_env();
    if is_headless() && args.attach.is_none() {
        return run_plain();
    }
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    rt.block_on(run_with_args(args))
}
