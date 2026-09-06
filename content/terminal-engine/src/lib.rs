//! Terminal Engine used by Vapor's evolving Hello World on Steroids techdemo.
//!
//! The Engine owns:
//!
//! - the Bevy ECS application;
//! - terminal lifecycle;
//! - input acquisition;
//! - frame timing;
//! - the terminal framebuffer;
//! - broad ECS integration phases.
//!
//! Crossterm is an implementation detail of this Engine. Games and Mods use
//! the Engine-owned input/rendering API rather than Crossterm directly.

pub use bevy_app::{App, Startup, Update};

use bevy_ecs::{
    prelude::Resource,
    schedule::{IntoScheduleConfigs, SystemSet},
};
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{
        self, Event as CrosstermEvent, KeyCode, KeyEvent as CrosstermKeyEvent, KeyEventKind,
        KeyModifiers as CrosstermKeyModifiers,
    },
    execute, queue,
    style::Print,
    terminal::{
        self, BeginSynchronizedUpdate, Clear, ClearType, DisableLineWrap, EnableLineWrap,
        EndSynchronizedUpdate, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};
use std::{
    io::{self, Stdout, Write, stdout},
    thread,
    time::{Duration, Instant},
};

const FRAME_INTERVAL: Duration = Duration::from_millis(33);

/// Broad Engine-owned ECS integration phases.
///
/// Configure runs once during Startup.
///
/// Input, Simulate, and Present run every Engine tick.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EngineSet {
    Configure,
    Input,
    Simulate,
    Present,
}

/// Platform-independent key vocabulary exposed by the Terminal Engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Character(char),
    Enter,
    Escape,
    Backspace,
    Tab,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    Delete,
}

/// Key modifiers exposed without leaking Crossterm types into Game APIs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KeyModifiers {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
}

/// One Engine-level key input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyInput {
    pub key: Key,
    pub modifiers: KeyModifiers,
}

impl KeyInput {
    pub const fn plain(key: Key) -> Self {
        Self {
            key,
            modifiers: KeyModifiers {
                shift: false,
                control: false,
                alt: false,
            },
        }
    }
}

/// Input event exposed by the Terminal Engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputEvent {
    Key(KeyInput),
    Resize { width: u16, height: u16 },
}

/// Events observed during the current Engine tick.
///
/// The Engine replaces this collection before every `App::update()`.
#[derive(Debug, Default, Resource)]
pub struct InputState {
    events: Vec<InputEvent>,
}

impl InputState {
    pub fn events(&self) -> &[InputEvent] {
        &self.events
    }

    /// Useful for Engine-compatible tests and synthetic input sources.
    pub fn push_event(&mut self, event: InputEvent) {
        self.events.push(event);
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }

    fn replace(&mut self, events: Vec<InputEvent>) {
        self.events = events;
    }
}

/// Engine lifecycle control exposed to compatible Content.
#[derive(Debug, Default, Resource)]
pub struct EngineControl {
    quit_requested: bool,
}

impl EngineControl {
    pub fn request_quit(&mut self) {
        self.quit_requested = true;
    }

    pub fn quit_requested(&self) -> bool {
        self.quit_requested
    }
}

/// Basic Engine timing information.
///
/// This deliberately remains tiny for now. More sophisticated fixed-step/time
/// semantics should arise from real runtime pressure rather than anticipation.
#[derive(Debug, Resource)]
pub struct EngineTime {
    tick: u64,
    delta: Duration,
    elapsed: Duration,
}

impl Default for EngineTime {
    fn default() -> Self {
        Self {
            tick: 0,
            delta: Duration::ZERO,
            elapsed: Duration::ZERO,
        }
    }
}

impl EngineTime {
    pub fn tick(&self) -> u64 {
        self.tick
    }

    pub fn delta(&self) -> Duration {
        self.delta
    }

    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    fn advance(&mut self, delta: Duration) {
        self.tick += 1;
        self.delta = delta;
        self.elapsed += delta;
    }
}

/// Engine-owned ASCII framebuffer.
///
/// Coordinates are terminal cell coordinates.
///
/// The current renderer intentionally models only characters. Styling/color
/// can be added later if actual Content gives us a reason to model them.
#[derive(Debug, Resource)]
pub struct TerminalFrame {
    width: u16,
    height: u16,
    cells: Vec<char>,
}

impl Default for TerminalFrame {
    fn default() -> Self {
        Self::new(80, 24)
    }
}

impl TerminalFrame {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            cells: vec![' '; usize::from(width) * usize::from(height)],
        }
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
        self.cells = vec![' '; usize::from(width) * usize::from(height)];
    }

    pub fn clear(&mut self) {
        self.cells.fill(' ');
    }

    pub fn put(&mut self, x: u16, y: u16, character: char) {
        if x >= self.width || y >= self.height {
            return;
        }

        let index = usize::from(y) * usize::from(self.width) + usize::from(x);

        self.cells[index] = character;
    }

    pub fn write(&mut self, x: u16, y: u16, text: impl AsRef<str>) {
        if y >= self.height {
            return;
        }

        for (offset, character) in text.as_ref().chars().enumerate() {
            let Ok(offset) = u16::try_from(offset) else {
                break;
            };

            let Some(x) = x.checked_add(offset) else {
                break;
            };

            if x >= self.width {
                break;
            }

            self.put(x, y, character);
        }
    }

    pub fn write_centered(&mut self, y: u16, text: impl AsRef<str>) {
        let text = text.as_ref();

        let width = text.chars().count().min(usize::from(u16::MAX)) as u16;

        let x = self.width.saturating_sub(width) / 2;

        self.write(x, y, text);
    }

    pub fn draw_border(&mut self) {
        if self.width < 2 || self.height < 2 {
            return;
        }

        let right = self.width - 1;
        let bottom = self.height - 1;

        self.put(0, 0, '+');
        self.put(right, 0, '+');
        self.put(0, bottom, '+');
        self.put(right, bottom, '+');

        for x in 1..right {
            self.put(x, 0, '-');
            self.put(x, bottom, '-');
        }

        for y in 1..bottom {
            self.put(0, y, '|');
            self.put(right, y, '|');
        }
    }

    fn row_text(&self, y: u16) -> String {
        if y >= self.height {
            return String::new();
        }

        let start = usize::from(y) * usize::from(self.width);
        let end = start + usize::from(self.width);

        self.cells[start..end].iter().collect()
    }
}

/// Construct an Engine-owned ECS application without opening a real terminal.
///
/// This is intentionally usable by tests and future embedding scenarios.
pub fn create_app() -> App {
    let mut app = App::new();

    app.init_resource::<InputState>()
        .init_resource::<EngineControl>()
        .init_resource::<EngineTime>()
        .init_resource::<TerminalFrame>()
        .configure_sets(Startup, EngineSet::Configure)
        .configure_sets(
            Update,
            (EngineSet::Input, EngineSet::Simulate, EngineSet::Present).chain(),
        );

    app
}

/// Run one real Terminal Engine application.
///
/// The callback is the current explicit Engine-specific static integration
/// surface used by the Vapor-generated composition.
pub fn run(configure: impl FnOnce(&mut App)) {
    if let Err(error) = try_run(configure) {
        panic!("Terminal Engine failed: {error}");
    }
}

pub fn try_run(configure: impl FnOnce(&mut App)) -> io::Result<()> {
    let mut terminal = TerminalSession::enter()?;

    let (width, height) = terminal::size()?;

    let mut app = create_app();

    app.world_mut()
        .resource_mut::<TerminalFrame>()
        .resize(width, height);

    configure(&mut app);

    let mut previous_tick = None;

    loop {
        let tick_started = Instant::now();

        let delta = previous_tick
            .map(|previous: Instant| tick_started.saturating_duration_since(previous))
            .unwrap_or(FRAME_INTERVAL);

        previous_tick = Some(tick_started);

        let input = poll_terminal_input()?;

        let engine_quit = input.events.iter().any(engine_quit_requested);

        {
            app.world_mut()
                .resource_mut::<InputState>()
                .replace(input.events);
        }

        if let Some((width, height)) = input.resize {
            app.world_mut()
                .resource_mut::<TerminalFrame>()
                .resize(width, height);
        }

        if engine_quit {
            app.world_mut()
                .resource_mut::<EngineControl>()
                .request_quit();
        }

        app.world_mut().resource_mut::<EngineTime>().advance(delta);

        app.world_mut().resource_mut::<TerminalFrame>().clear();

        app.update();

        {
            let frame = app.world().resource::<TerminalFrame>();
            terminal.present(frame)?;
        }

        if app.world().resource::<EngineControl>().quit_requested() {
            break;
        }

        if let Some(remaining) = FRAME_INTERVAL.checked_sub(tick_started.elapsed()) {
            thread::sleep(remaining);
        }
    }

    Ok(())
}

struct PolledInput {
    events: Vec<InputEvent>,
    resize: Option<(u16, u16)>,
}

fn poll_terminal_input() -> io::Result<PolledInput> {
    let mut events = Vec::new();
    let mut resize = None;

    while event::poll(Duration::ZERO)? {
        match event::read()? {
            CrosstermEvent::Key(event)
                if matches!(event.kind, KeyEventKind::Press | KeyEventKind::Repeat) =>
            {
                if let Some(key) = translate_key(event) {
                    events.push(InputEvent::Key(key));
                }
            }

            CrosstermEvent::Resize(width, height) => {
                resize = Some((width, height));

                events.push(InputEvent::Resize { width, height });
            }

            _ => {}
        }
    }

    Ok(PolledInput { events, resize })
}

fn translate_key(event: CrosstermKeyEvent) -> Option<KeyInput> {
    let key = match event.code {
        KeyCode::Char(character) => Key::Character(character),
        KeyCode::Enter => Key::Enter,
        KeyCode::Esc => Key::Escape,
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Tab => Key::Tab,
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Left => Key::Left,
        KeyCode::Right => Key::Right,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::PageDown => Key::PageDown,
        KeyCode::Insert => Key::Insert,
        KeyCode::Delete => Key::Delete,

        _ => return None,
    };

    Some(KeyInput {
        key,
        modifiers: KeyModifiers {
            shift: event.modifiers.contains(CrosstermKeyModifiers::SHIFT),

            control: event.modifiers.contains(CrosstermKeyModifiers::CONTROL),

            alt: event.modifiers.contains(CrosstermKeyModifiers::ALT),
        },
    })
}

fn engine_quit_requested(event: &InputEvent) -> bool {
    match event {
        InputEvent::Key(KeyInput {
            key: Key::Escape, ..
        }) => true,

        InputEvent::Key(KeyInput {
            key: Key::Character('c' | 'C'),
            modifiers,
        }) if modifiers.control => true,

        _ => false,
    }
}

/// Owns the actual host terminal while the Engine is running.
///
/// Drop restores the user's terminal even when normal Engine execution returns
/// through an error.
struct TerminalSession {
    stdout: Stdout,
}

impl TerminalSession {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;

        let mut stdout = stdout();

        if let Err(error) = execute!(
            stdout,
            EnterAlternateScreen,
            DisableLineWrap,
            Hide,
            Clear(ClearType::All),
        ) {
            let _ = disable_raw_mode();
            let _ = execute!(stdout, Show, EnableLineWrap, LeaveAlternateScreen,);

            return Err(error);
        }

        Ok(Self { stdout })
    }

    fn present(&mut self, frame: &TerminalFrame) -> io::Result<()> {
        queue!(self.stdout, BeginSynchronizedUpdate)?;

        for y in 0..frame.height() {
            queue!(self.stdout, MoveTo(0, y), Print(frame.row_text(y)),)?;
        }

        queue!(self.stdout, EndSynchronizedUpdate)?;

        self.stdout.flush()
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = execute!(self.stdout, Show, EnableLineWrap, LeaveAlternateScreen,);

        let _ = disable_raw_mode();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::prelude::{ResMut, Resource};

    #[derive(Resource, Default)]
    struct Trace(Vec<&'static str>);

    fn configure(mut trace: ResMut<Trace>) {
        trace.0.push("configure");
    }

    fn input(mut trace: ResMut<Trace>) {
        trace.0.push("input");
    }

    fn simulate(mut trace: ResMut<Trace>) {
        trace.0.push("simulate");
    }

    fn present(mut trace: ResMut<Trace>) {
        trace.0.push("present");
    }

    #[test]
    fn engine_lifecycle_runs_in_declared_order() {
        let mut app = create_app();

        app.init_resource::<Trace>()
            .add_systems(Startup, configure.in_set(EngineSet::Configure))
            .add_systems(
                Update,
                (
                    input.in_set(EngineSet::Input),
                    simulate.in_set(EngineSet::Simulate),
                    present.in_set(EngineSet::Present),
                ),
            );

        app.update();
        app.update();

        assert_eq!(
            app.world().resource::<Trace>().0,
            vec![
                "configure",
                "input",
                "simulate",
                "present",
                "input",
                "simulate",
                "present",
            ]
        );
    }

    #[test]
    fn framebuffer_clips_writes_to_its_bounds() {
        let mut frame = TerminalFrame::new(5, 2);

        frame.write(3, 0, "hello");
        frame.write(0, 5, "invisible");

        assert_eq!(frame.row_text(0), "   he");
        assert_eq!(frame.row_text(1), "     ");
    }
}
