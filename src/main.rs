//! workedin: the LinkedIn experience in a terminal, with fictional coworkers.

mod app;
mod data;
mod ui;

use std::io;
use std::panic;

use ratatui::DefaultTerminal;
use ratatui::crossterm::cursor;
use ratatui::crossterm::event;
use ratatui::crossterm::event::Event;
use ratatui::crossterm::event::KeyEventKind;
use ratatui::crossterm::execute;

use crate::app::App;

fn run(terminal: &mut DefaultTerminal) -> io::Result<()> {
    let mut app = App::new();
    while !app.should_quit {
        terminal.draw(|frame| ui::draw(frame, &app))?;
        if let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            app.on_key(key);
        }
    }
    Ok(())
}

/// Raw mode off and the alternate screen left (`ratatui::restore`), plus the
/// cursor shown again, which `restore` doesn't do by itself.
fn restore() {
    ratatui::restore();
    let _ = execute!(io::stdout(), cursor::Show);
}

fn main() -> io::Result<()> {
    // `init` installs a panic hook that calls `ratatui::restore` before the
    // default hook prints the message. Wrap it so a panic shows the cursor too.
    let mut terminal = ratatui::init();
    let hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let _ = execute!(io::stdout(), cursor::Show);
        hook(info);
    }));

    let result = run(&mut terminal);
    restore();
    result
}
