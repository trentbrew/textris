use std::io;
use std::time::Duration;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, BorderType, Paragraph},
};
use rodio::{source::Source, OutputStream, Sink};

#[derive(Clone, Copy, PartialEq)]
enum GameEvent {
    Move,
    Rotate,
    Lock,
    Clear(u32), // lines cleared
    GameOver,
    Hold,
    LevelUp,
}

#[derive(Clone, Copy, PartialEq)]
enum Cell {
    Empty,
    Filled(Color),
}

#[derive(Clone, Copy)]
struct Tetromino {
    shape: [[bool; 4]; 4],
    color: Color,
    x: i32,
    y: i32,
}

impl Tetromino {
    fn i() -> Self {
        Self {
            shape: [
                [false, false, false, false],
                [true, true, true, true],
                [false, false, false, false],
                [false, false, false, false],
            ],
            color: Color::Cyan,
            x: 3,
            y: 0,
        }
    }

    fn o() -> Self {
        Self {
            shape: [
                [true, true, false, false],
                [true, true, false, false],
                [false, false, false, false],
                [false, false, false, false],
            ],
            color: Color::Yellow,
            x: 4,
            y: 0,
        }
    }

    fn t() -> Self {
        Self {
            shape: [
                [false, true, false, false],
                [true, true, true, false],
                [false, false, false, false],
                [false, false, false, false],
            ],
            color: Color::Magenta,
            x: 3,
            y: 0,
        }
    }

    fn s() -> Self {
        Self {
            shape: [
                [false, true, true, false],
                [true, true, false, false],
                [false, false, false, false],
                [false, false, false, false],
            ],
            color: Color::Green,
            x: 3,
            y: 0,
        }
    }

    fn z() -> Self {
        Self {
            shape: [
                [true, true, false, false],
                [false, true, true, false],
                [false, false, false, false],
                [false, false, false, false],
            ],
            color: Color::Red,
            x: 3,
            y: 0,
        }
    }

    fn l() -> Self {
        Self {
            shape: [
                [false, false, true, false],
                [true, true, true, false],
                [false, false, false, false],
                [false, false, false, false],
            ],
            color: Color::LightYellow,
            x: 3,
            y: 0,
        }
    }

    fn j() -> Self {
        Self {
            shape: [
                [true, false, false, false],
                [true, true, true, false],
                [false, false, false, false],
                [false, false, false, false],
            ],
            color: Color::Blue,
            x: 3,
            y: 0,
        }
    }

    fn rotate(&self) -> Self {
        let mut new_shape = [[false; 4]; 4];
        for y in 0..4 {
            for x in 0..4 {
                new_shape[x][3 - y] = self.shape[y][x];
            }
        }
        Self {
            shape: new_shape,
            ..*self
        }
    }

    fn cells(&self) -> Vec<(i32, i32)> {
        let mut cells = Vec::new();
        for y in 0..4 {
            for x in 0..4 {
                if self.shape[y][x] {
                    cells.push((self.x + x as i32, self.y + y as i32));
                }
            }
        }
        cells
    }
}

struct Game {
    board: Vec<Vec<Cell>>,
    width: usize,
    height: usize,
    current: Tetromino,
    next: Tetromino,
    hold: Option<Tetromino>,
    can_hold: bool,
    score: u32,
    lines: u32,
    level: u32,
    game_over: bool,
    paused: bool,
    tick_count: u64,
    combo: u32,
    lock_delay_start: Option<std::time::Instant>,
    events: Vec<GameEvent>,
}

struct SoundSystem {
    _stream: OutputStream,
    stream_handle: rodio::OutputStreamHandle,
}

impl SoundSystem {
    fn new() -> Option<Self> {
        let (_stream, stream_handle) = OutputStream::try_default().ok()?;
        Some(Self { _stream, stream_handle })
    }

    fn play(&self, event: GameEvent) {
        if let Ok(sink) = Sink::try_new(&self.stream_handle) {
            match event {
                GameEvent::Move => {
                    let source = rodio::source::SineWave::new(400.0)
                        .take_duration(Duration::from_millis(20))
                        .amplify(0.10);
                    sink.append(source);
                }
                GameEvent::Rotate => {
                    let source = rodio::source::SineWave::new(600.0)
                        .take_duration(Duration::from_millis(40))
                        .amplify(0.10);
                    sink.append(source);
                }
                GameEvent::Lock => {
                    let source = rodio::source::SineWave::new(200.0)
                        .take_duration(Duration::from_millis(100))
                        .amplify(0.30);
                    sink.append(source);
                }
                GameEvent::Clear(lines) => {
                    let base_freq = 800.0;
                    let freq = base_freq + (lines as f32 * 200.0);
                    let source = rodio::source::SineWave::new(freq)
                        .take_duration(Duration::from_millis(200))
                        .amplify(0.20);
                    sink.append(source);
                }
                GameEvent::Hold => {
                    let source = rodio::source::SineWave::new(500.0)
                        .take_duration(Duration::from_millis(50))
                        .amplify(0.10);
                    sink.append(source);
                }
                GameEvent::GameOver => {
                    let source = rodio::source::SineWave::new(100.0)
                        .take_duration(Duration::from_millis(1000))
                        .amplify(0.30);
                    sink.append(source);
                }
                GameEvent::LevelUp => {
                    let source = rodio::source::SineWave::new(880.0)
                        .take_duration(Duration::from_millis(150))
                        .amplify(0.20);
                    sink.append(source);
                    let source2 = rodio::source::SineWave::new(1760.0)
                        .take_duration(Duration::from_millis(300))
                        .amplify(0.20);
                    sink.append(source2);
                }
            }
            sink.detach();
        }
    }
}

impl Game {
    fn new(width: usize, height: usize) -> Self {
        Self {
            board: vec![vec![Cell::Empty; width]; height],
            width,
            height,
            current: Self::random_piece(),
            next: Self::random_piece(),
            hold: None,
            can_hold: true,
            score: 0,
            lines: 0,
            level: 1,
            game_over: false,
            paused: false,
            tick_count: 0,
            combo: 0,
            lock_delay_start: None,
            events: Vec::new(),
        }
    }

    fn random_piece() -> Tetromino {
        let pieces = [Tetromino::i(), Tetromino::o(), Tetromino::t(),
                      Tetromino::s(), Tetromino::z(), Tetromino::l(), Tetromino::j()];
        pieces[fastrand::usize(0..7)]
    }

    fn collision(&self, piece: &Tetromino) -> bool {
        for (x, y) in piece.cells() {
            if x < 0 || x >= self.width as i32 || y >= self.height as i32 {
                return true;
            }
            if y >= 0 && self.board[y as usize][x as usize] != Cell::Empty {
                return true;
            }
        }
        false
    }

    fn lock_piece(&mut self) {
        self.events.push(GameEvent::Lock);
        for (x, y) in self.current.cells() {
            if y >= 0 && y < self.height as i32 && x >= 0 && x < self.width as i32 {
                self.board[y as usize][x as usize] = Cell::Filled(self.current.color);
            }
        }
        self.clear_lines();
        self.spawn_piece();
        self.lock_delay_start = None;
    }

    fn clear_lines(&mut self) {
        let mut lines_cleared = 0;
        let mut y = self.height as i32 - 1;

        while y >= 0 {
            let full = (0..self.width).all(|x| self.board[y as usize][x] != Cell::Empty);
            if full {
                lines_cleared += 1;
                for row in (1..=y as usize).rev() {
                    self.board[row] = self.board[row - 1].clone();
                }
                self.board[0] = vec![Cell::Empty; self.width];
            } else {
                y -= 1;
            }
        }

        if lines_cleared > 0 {
            self.lines += lines_cleared;
            self.combo += 1;
            self.events.push(GameEvent::Clear(lines_cleared));

            // Score calculation with combo bonus
            let base_score = match lines_cleared {
                1 => 100,
                2 => 300,
                3 => 500,
                4 => 800,
                _ => 0,
            };

            let combo_bonus = if self.combo > 1 { (self.combo - 1) * 50 * self.level } else { 0 };
            self.score += (base_score * self.level) + combo_bonus;

            let old_level = self.level;
            self.level = (self.lines / 10) + 1;
            if self.level > old_level {
                self.events.push(GameEvent::LevelUp);
            }
        } else {
            self.combo = 0;
        }
    }

    fn spawn_piece(&mut self) {
        self.tick_count += 1;
        self.current = self.next;
        self.current.x = 3;
        self.current.y = 0;
        self.next = Self::random_piece();
        self.can_hold = true;

        if self.collision(&self.current) {
            self.game_over = true;
            self.events.push(GameEvent::GameOver);
        }
    }

    fn move_piece(&mut self, dx: i32, dy: i32) -> bool {
        let mut moved = self.current;
        moved.x += dx;
        moved.y += dy;
        if !self.collision(&moved) {
            self.current = moved;
            self.lock_delay_start = None; // Reset lock delay on successful move for smoother gameplay
            if dx != 0 {
                self.events.push(GameEvent::Move);
            }
            true
        } else {
            false
        }
    }

    fn rotate(&mut self, clockwise: bool) {
        // O piece (square) doesn't rotate
        if self.current.color == Color::Yellow {
            return;
        }

        let rotated = if clockwise {
            self.current.rotate()
        } else {
            // Rotate 3 times for counter-clockwise
            self.current.rotate().rotate().rotate()
        };

        if !self.collision(&rotated) {
            self.current = rotated;
            self.lock_delay_start = None; // Reset lock delay on rotation
            self.events.push(GameEvent::Rotate);
        }
    }

    fn hard_drop(&mut self) {
        while self.move_piece(0, 1) {}
        self.lock_piece();
    }

    fn hold_piece(&mut self) {
        if !self.can_hold {
            return;
        }

        self.events.push(GameEvent::Hold);

        if let Some(held_piece) = self.hold {
            self.hold = Some(self.current);
            self.current = held_piece;
            self.current.x = 3;
            self.current.y = 0;
        } else {
            self.hold = Some(self.current);
            self.spawn_piece();
        }
        self.can_hold = false;
    }

    fn get_ghost_y_offset(&self) -> i32 {
        let mut ghost = self.current;
        while !self.collision(&ghost) {
            ghost.y += 1;
        }
        ghost.y -= 1;
        ghost.y - self.current.y
    }

    fn tick(&mut self) {
        if self.game_over || self.paused {
            return;
        }

        // Try to move down
        if !self.move_piece(0, 1) {
            // Lock delay logic
            if let Some(start) = self.lock_delay_start {
                if start.elapsed() > std::time::Duration::from_millis(500) {
                    self.lock_piece();
                }
            } else {
                self.lock_delay_start = Some(std::time::Instant::now());
            }
        } else {
            // If we moved down successfully, reset lock delay
            self.lock_delay_start = None;
        }
    }

    fn restart(&mut self) {
        let width = self.width;
        let height = self.height;
        *self = Game::new(width, height);
    }
}

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    // Determine board size based on terminal height
    // Reserve space for header (3) and footer (2) and some padding
    let terminal_height = terminal.size()?.height;
    let board_height = (terminal_height as usize).saturating_sub(6).max(20);
    // Keep standard width for gameplay balance, but could be adjusted
    let board_width = 10;

    let mut game = Game::new(board_width, board_height);
    let mut last_tick = std::time::Instant::now();
    let sound_system = SoundSystem::new();

    loop {
        let frame_start = std::time::Instant::now();
        let tick_rate = std::time::Duration::from_millis((500 / game.level as u64).max(50));

        if last_tick.elapsed() >= tick_rate {
            game.tick();
            last_tick = std::time::Instant::now();
        }

        // Process game events and play sounds
        for event in game.events.drain(..) {
            if let Some(ref sounds) = sound_system {
                sounds.play(event);
            }
        }

        terminal.draw(|frame| {
            let main_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(board_height as u16),
                    Constraint::Length(2),
                ])
                .split(frame.area());

            // Header
            let status = if game.game_over {
                Span::styled("GAME OVER! ", Style::default().fg(Color::Red).bold())
            } else if game.paused {
                Span::styled("PAUSED ", Style::default().fg(Color::Yellow).bold())
            } else if game.combo > 1 {
                Span::styled(format!("COMBO x{}! ", game.combo), Style::default().fg(Color::LightRed).bold())
            } else {
                Span::styled("", Style::default())
            };

            let header = Paragraph::new(vec![Line::from(vec![
                status,
                Span::styled("Tetris", Style::default().fg(Color::Cyan).bold()),
                Span::raw("  │  "),
                Span::styled("Score: ", Style::default().fg(Color::DarkGray)),
                Span::styled(game.score.to_string(), Style::default().fg(Color::Yellow).bold()),
                Span::raw("  │  "),
                Span::styled("Lines: ", Style::default().fg(Color::DarkGray)),
                Span::styled(game.lines.to_string(), Style::default().fg(Color::Green)),
                Span::raw("  │  "),
                Span::styled("Level: ", Style::default().fg(Color::DarkGray)),
                Span::styled(game.level.to_string(), Style::default().fg(Color::Magenta)),
            ])])
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::BOTTOM));
            frame.render_widget(header, main_chunks[0]);

            // Game area
            let game_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Fill(1),
                    Constraint::Length(24),
                    Constraint::Length(12),
                    Constraint::Fill(1),
                ])
                .split(main_chunks[1]);

            // Board
            let mut board_lines: Vec<Line> = Vec::new();
            let ghost_y_offset = game.get_ghost_y_offset();

            for y in 0..game.height {
                let mut spans = Vec::new();
                for x in 0..game.width {
                    let cell = game.board[y][x];

                    // Check if current piece occupies this cell
                    let piece_here = game.current.cells().iter().any(|(px, py)| {
                        *px == x as i32 && *py == y as i32
                    });

                    // Check if ghost piece occupies this cell (at drop position)
                    let ghost_here = game.current.cells().iter().any(|(px, py)| {
                        *px == x as i32 && (*py + ghost_y_offset) == y as i32
                    });

                    let (ch, color, style) = if piece_here {
                        ("██", game.current.color, Style::default())
                    } else if ghost_here && !piece_here {
                        ("░░", game.current.color, Style::default())
                    } else {
                        match cell {
                            Cell::Empty => ("· ", Color::DarkGray, Style::default().dim()),
                            Cell::Filled(c) => ("▓▓", c, Style::default()),
                        }
                    };
                    spans.push(Span::styled(ch, style.fg(color)));
                }
                board_lines.push(Line::from(spans));
            }

            let board = Paragraph::new(board_lines)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Thick)
                        .border_style(Style::default().fg(Color::White))
                        .title(format!(" Board ({}x{}) ", game.width, game.height)),
                )
                .alignment(Alignment::Center);
            frame.render_widget(board, game_chunks[1]);

            // Side panel with next and hold
            let side_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(10),
                    Constraint::Length(10),
                ])
                .split(game_chunks[2]);

            // Next piece preview
            let mut next_lines: Vec<Line> = vec![Line::from(""), Line::from("Next:")];
            for y in 0..4 {
                let mut spans = Vec::new();
                spans.push(Span::raw(" "));
                for x in 0..4 {
                    if game.next.shape[y][x] {
                        spans.push(Span::styled("██", Style::default().fg(game.next.color)));
                    } else {
                        spans.push(Span::raw("  "));
                    }
                }
                next_lines.push(Line::from(spans));
            }

            let next_panel = Paragraph::new(next_lines)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::Yellow))
                        .title(" Next "),
                );
            frame.render_widget(next_panel, side_chunks[0]);

            // Hold piece preview
            let mut hold_lines: Vec<Line> = vec![Line::from(""), Line::from("Hold:")];
            if let Some(ref hold_piece) = game.hold {
                for y in 0..4 {
                    let mut spans = Vec::new();
                    spans.push(Span::raw(" "));
                    for x in 0..4 {
                        if hold_piece.shape[y][x] {
                            spans.push(Span::styled("██", Style::default().fg(hold_piece.color)));
                        } else {
                            spans.push(Span::raw("  "));
                        }
                    }
                    hold_lines.push(Line::from(spans));
                }
            } else {
                hold_lines.push(Line::from("   Empty"));
            }

            let hold_panel = Paragraph::new(hold_lines)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::Cyan))
                        .title(" Hold "),
                );
            frame.render_widget(hold_panel, side_chunks[1]);

            // Footer
            let footer_text = if game.game_over {
                vec![
                    Span::styled("r", Style::default().fg(Color::Green)),
                    Span::raw(" restart  "),
                    Span::styled("q", Style::default().fg(Color::Magenta)),
                    Span::raw(" quit"),
                ]
            } else {
                vec![
                    Span::styled("←→/hl", Style::default().fg(Color::Green)),
                    Span::raw(" move  "),
                    Span::styled("↑/k", Style::default().fg(Color::Yellow)),
                    Span::raw(" rot-R "),
                    Span::styled("z", Style::default().fg(Color::LightYellow)),
                    Span::raw(" rot-L "),
                    Span::styled("↓/j", Style::default().fg(Color::Blue)),
                    Span::raw(" soft  "),
                    Span::styled("Space", Style::default().fg(Color::Cyan)),
                    Span::raw(" drop  "),
                    Span::styled("c", Style::default().fg(Color::LightMagenta)),
                    Span::raw(" hold  "),
                    Span::styled("p", Style::default().fg(Color::DarkGray)),
                    Span::raw(" pause  "),
                    Span::styled("q", Style::default().fg(Color::Magenta)),
                    Span::raw(" quit"),
                ]
            };

            let footer = Paragraph::new(vec![Line::from(footer_text)])
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::TOP));
            frame.render_widget(footer, main_chunks[2]);
        })?;

        // Handle input - process all pending events to prevent stutter
        while event::poll(std::time::Duration::from_millis(0))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => {
                            disable_raw_mode()?;
                            io::stdout().execute(LeaveAlternateScreen)?;
                            return Ok(());
                        }
                        KeyCode::Char('r') => game.restart(),
                        KeyCode::Char('p') | KeyCode::Char(' ') if !game.game_over => {
                            if key.code == KeyCode::Char('p') {
                                game.paused = !game.paused;
                            } else {
                                game.hard_drop();
                            }
                        }
                        KeyCode::Left | KeyCode::Char('h') => { game.move_piece(-1, 0); }
                        KeyCode::Right | KeyCode::Char('l') => { game.move_piece(1, 0); }
                        KeyCode::Down | KeyCode::Char('j') => { game.move_piece(0, 1); }
                        KeyCode::Up | KeyCode::Char('k') => game.rotate(true),
                        KeyCode::Char('z') => game.rotate(false),
                        KeyCode::Char('c') => game.hold_piece(),
                        _ => {}
                    }
                }
            }
        }

        // Limit frame rate to approx 60 FPS to prevent 100% CPU usage
        let elapsed = frame_start.elapsed();
        if elapsed < std::time::Duration::from_millis(16) {
            std::thread::sleep(std::time::Duration::from_millis(16) - elapsed);
        }
    }
}
