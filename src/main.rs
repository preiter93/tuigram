#![allow(dead_code)]
mod app;
mod core;
mod render;
mod theme;
mod ui;

use ratatui::crossterm::{
    event::{self, Event as CEvent},
    execute,
};
use tui_world::prelude::*;

fn main() -> anyhow::Result<()> {
    run()
}

fn run() -> anyhow::Result<()> {
    let mut terminal = ratatui::init();
    execute!(std::io::stdout())?;

    let mut world = World::default();
    app::setup_world(&mut world);

    loop {
        terminal.draw(|frame| app::render(frame, &mut world))?;

        if event::poll(std::time::Duration::from_millis(16))? {
            let active = app::active_widgets(&world);

            if let CEvent::Key(key) = event::read()? {
                Event::Key(key).handle(&mut world, &active);
            }
        }

        if world.get::<app::AppState>().should_quit {
            break;
        }
    }

    execute!(std::io::stdout())?;
    ratatui::restore();

    Ok(())
}
