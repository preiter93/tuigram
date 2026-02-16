#![allow(dead_code)]
mod app;
mod core;
mod layout;
mod render;
mod theme;

use ratatui::crossterm::{
    event::{self, Event as CEvent},
    execute,
};
use tui_world::prelude::*;

use crate::app::GLOBAL;

fn main() -> anyhow::Result<()> {
    run()
}

fn run() -> anyhow::Result<()> {
    let mut terminal = ratatui::init();
    execute!(std::io::stdout())?;

    let mut world = World::default();
    app::setup_world(&mut world)?;

    loop {
        terminal.draw(|frame| app::render(frame, &mut world))?;

        if event::poll(std::time::Duration::from_millis(16))? {
            let active = vec![GLOBAL];

            match event::read()? {
                CEvent::Key(key) => Event::Key(key).handle(&mut world, &active),
                CEvent::Mouse(mouse) => Event::Mouse(mouse).handle(&mut world, &active),
                _ => {}
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
