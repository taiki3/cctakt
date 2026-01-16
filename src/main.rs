use anyhow::{Context, Result};
use crossterm::{
    cursor::Hide,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{self, disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// アプリケーションの状態
struct App {
    /// 仮想ターミナル（PTY出力をパース）
    parser: Arc<Mutex<vt100::Parser>>,
    /// PTYへの書き込み用
    pty_writer: Option<Box<dyn Write + Send>>,
    /// PTYマスター（リサイズ用）
    pty_master: Option<Box<dyn portable_pty::MasterPty + Send>>,
    /// 終了フラグ
    should_quit: bool,
    /// セッション終了フラグ
    session_ended: bool,
}

impl App {
    fn new(rows: u16, cols: u16) -> Self {
        Self {
            parser: Arc::new(Mutex::new(vt100::Parser::new(rows, cols, 1000))),
            pty_writer: None,
            pty_master: None,
            should_quit: false,
            session_ended: false,
        }
    }

    /// PTYにバイト列を送信
    fn send_bytes(&mut self, bytes: &[u8]) {
        if let Some(ref mut writer) = self.pty_writer {
            let _ = writer.write_all(bytes);
            let _ = writer.flush();
        }
    }

    /// PTYサイズを更新
    fn resize_pty(&mut self, cols: u16, rows: u16) {
        // vt100パーサーをリサイズ
        {
            let mut parser = self.parser.lock().unwrap();
            parser.set_size(rows, cols);
        }
        // PTYをリサイズ
        if let Some(ref master) = self.pty_master {
            let _ = master.resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            });
        }
    }
}

fn main() -> Result<()> {
    loop {
        let action = run_claude_session()?;
        match action {
            SessionAction::Restart => continue,
            SessionAction::Quit => break,
        }
    }
    Ok(())
}

enum SessionAction {
    Restart,
    Quit,
}

fn run_claude_session() -> Result<SessionAction> {
    // ターミナルサイズを取得
    let (cols, rows) = terminal::size().context("Failed to get terminal size")?;
    let content_rows = rows.saturating_sub(3); // ヘッダー1行 + ボーダー2行

    // ターミナルのセットアップ
    enable_raw_mode().context("Failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, Hide)?;
    execute!(stdout, crossterm::terminal::SetTitle("cctakt - Claude Code Orchestrator"))?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // アプリケーション初期化
    let mut app = App::new(content_rows, cols.saturating_sub(2)); // ボーダー分引く

    // PTYのセットアップ
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: content_rows,
            cols: cols.saturating_sub(2),
            pixel_width: 0,
            pixel_height: 0,
        })
        .context("Failed to open pty")?;

    // Claude Code を起動
    let cmd = CommandBuilder::new("claude");
    let mut child = pair.slave.spawn_command(cmd).context("Failed to spawn claude")?;
    drop(pair.slave);

    // PTYの読み書き設定
    let reader = pair.master.try_clone_reader().context("Failed to clone reader")?;
    app.pty_writer = Some(pair.master.take_writer().context("Failed to take writer")?);
    app.pty_master = Some(pair.master);

    // PTY出力読み取りスレッド -> vt100にフィード
    let parser = Arc::clone(&app.parser);
    let output_handle = std::thread::spawn(move || {
        let mut reader = reader;
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let mut parser = parser.lock().unwrap();
                    parser.process(&buf[..n]);
                }
                Err(_) => break,
            }
        }
    });

    // メインループ
    let mut user_quit = false;
    loop {
        // 描画
        terminal.draw(|f| ui(f, &app))?;

        // イベントをポーリング（16msごと ≈ 60fps）
        if event::poll(Duration::from_millis(16))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if app.session_ended {
                        // セッション終了後のメニュー
                        match key.code {
                            KeyCode::Char('r') | KeyCode::Char('R') => {
                                break; // restart
                            }
                            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                                user_quit = true;
                                break;
                            }
                            _ => {}
                        }
                    } else {
                        // 通常のキー入力処理
                        match (key.modifiers, key.code) {
                            (KeyModifiers::CONTROL, KeyCode::Char('q')) => {
                                user_quit = true;
                                break;
                            }
                            (KeyModifiers::CONTROL, KeyCode::Char(c)) => {
                                let ctrl_char = (c as u8) & 0x1f;
                                app.send_bytes(&[ctrl_char]);
                            }
                            (_, KeyCode::Enter) => app.send_bytes(b"\r"),
                            (_, KeyCode::Backspace) => app.send_bytes(&[0x7f]),
                            (_, KeyCode::Tab) => app.send_bytes(b"\t"),
                            (_, KeyCode::Esc) => app.send_bytes(&[0x1b]),
                            (_, KeyCode::Up) => app.send_bytes(b"\x1b[A"),
                            (_, KeyCode::Down) => app.send_bytes(b"\x1b[B"),
                            (_, KeyCode::Right) => app.send_bytes(b"\x1b[C"),
                            (_, KeyCode::Left) => app.send_bytes(b"\x1b[D"),
                            (_, KeyCode::Home) => app.send_bytes(b"\x1b[H"),
                            (_, KeyCode::End) => app.send_bytes(b"\x1b[F"),
                            (_, KeyCode::PageUp) => app.send_bytes(b"\x1b[5~"),
                            (_, KeyCode::PageDown) => app.send_bytes(b"\x1b[6~"),
                            (_, KeyCode::Delete) => app.send_bytes(b"\x1b[3~"),
                            (_, KeyCode::Char(c)) => {
                                let mut buf = [0u8; 4];
                                let s = c.encode_utf8(&mut buf);
                                app.send_bytes(s.as_bytes());
                            }
                            _ => {}
                        }
                    }
                }
                Event::Resize(new_cols, new_rows) => {
                    let content_rows = new_rows.saturating_sub(3);
                    app.resize_pty(new_cols.saturating_sub(2), content_rows);
                }
                _ => {}
            }
        }

        // 子プロセスが終了したかチェック
        if !app.session_ended {
            if let Ok(Some(_)) = child.try_wait() {
                app.session_ended = true;
            }
        }

        if app.should_quit {
            break;
        }
    }

    let _ = output_handle.join();

    // クリーンアップ
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        crossterm::cursor::Show,
        LeaveAlternateScreen
    )?;

    if user_quit {
        Ok(SessionAction::Quit)
    } else {
        Ok(SessionAction::Restart)
    }
}

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // ヘッダー
            Constraint::Min(1),    // メインエリア
        ])
        .split(f.area());

    // ヘッダー
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " cctakt ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " Claude Code Orchestrator ",
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled("[Ctrl+Q: quit]", Style::default().fg(Color::DarkGray)),
    ]));
    f.render_widget(header, chunks[0]);

    // メインエリア
    if app.session_ended {
        // セッション終了メニュー
        let menu = Paragraph::new(vec![
            Line::from(""),
            Line::from("  Claude Code session ended."),
            Line::from(""),
            Line::from(vec![
                Span::styled("  [R]", Style::default().fg(Color::Green)),
                Span::raw(" Restart Claude Code"),
            ]),
            Line::from(vec![
                Span::styled("  [Q]", Style::default().fg(Color::Red)),
                Span::raw(" Quit cctakt"),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "  Press R or Q...",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        f.render_widget(menu, chunks[1]);
    } else {
        // vt100画面を描画
        let parser = app.parser.lock().unwrap();
        let screen = parser.screen();

        let content_height = chunks[1].height.saturating_sub(2) as usize;
        let content_width = chunks[1].width.saturating_sub(2) as usize;

        let mut lines: Vec<Line> = Vec::new();

        for row in 0..content_height {
            let mut spans: Vec<Span> = Vec::new();
            let mut current_text = String::new();
            let mut current_style = Style::default();

            for col in 0..content_width {
                let cell = screen.cell(row as u16, col as u16);
                if let Some(cell) = cell {
                    let cell_style = cell_to_style(cell);

                    if cell_style != current_style {
                        if !current_text.is_empty() {
                            spans.push(Span::styled(current_text.clone(), current_style));
                            current_text.clear();
                        }
                        current_style = cell_style;
                    }

                    current_text.push_str(&cell.contents());
                }
            }

            if !current_text.is_empty() {
                spans.push(Span::styled(current_text, current_style));
            }

            lines.push(Line::from(spans));
        }

        let terminal_widget = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        f.render_widget(terminal_widget, chunks[1]);
    }
}

/// vt100のセル属性をratatuiのStyleに変換
fn cell_to_style(cell: &vt100::Cell) -> Style {
    let mut style = Style::default();

    // 前景色
    let fg = cell.fgcolor();
    if !matches!(fg, vt100::Color::Default) {
        style = style.fg(vt100_color_to_ratatui(fg));
    }

    // 背景色
    let bg = cell.bgcolor();
    if !matches!(bg, vt100::Color::Default) {
        style = style.bg(vt100_color_to_ratatui(bg));
    }

    // 属性
    if cell.bold() {
        style = style.add_modifier(Modifier::BOLD);
    }
    if cell.italic() {
        style = style.add_modifier(Modifier::ITALIC);
    }
    if cell.underline() {
        style = style.add_modifier(Modifier::UNDERLINED);
    }
    if cell.inverse() {
        style = style.add_modifier(Modifier::REVERSED);
    }

    style
}

/// vt100の色をratatuiの色に変換
fn vt100_color_to_ratatui(color: vt100::Color) -> Color {
    match color {
        vt100::Color::Default => Color::Reset,
        vt100::Color::Idx(0) => Color::Black,
        vt100::Color::Idx(1) => Color::Red,
        vt100::Color::Idx(2) => Color::Green,
        vt100::Color::Idx(3) => Color::Yellow,
        vt100::Color::Idx(4) => Color::Blue,
        vt100::Color::Idx(5) => Color::Magenta,
        vt100::Color::Idx(6) => Color::Cyan,
        vt100::Color::Idx(7) => Color::Gray,
        vt100::Color::Idx(8) => Color::DarkGray,
        vt100::Color::Idx(9) => Color::LightRed,
        vt100::Color::Idx(10) => Color::LightGreen,
        vt100::Color::Idx(11) => Color::LightYellow,
        vt100::Color::Idx(12) => Color::LightBlue,
        vt100::Color::Idx(13) => Color::LightMagenta,
        vt100::Color::Idx(14) => Color::LightCyan,
        vt100::Color::Idx(15) => Color::White,
        vt100::Color::Idx(idx) => Color::Indexed(idx),
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}
