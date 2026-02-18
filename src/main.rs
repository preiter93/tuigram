mod app;
mod core;
mod render;
mod theme;
mod ui;

use clap::Parser;
use ratatui::crossterm::{
    event::{self, Event as CEvent},
    execute,
};
use std::path::PathBuf;
use tui_world::prelude::*;

use crate::core::SequenceDiagram;

/// A TUI sequence diagram editor
#[derive(Parser, Debug)]
#[command(name = "tuigram")]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Import a Mermaid (.mmd) sequence diagram file
    #[arg(short, long, value_name = "FILE")]
    import: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let diagram = if let Some(path) = args.import {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| anyhow::anyhow!("Failed to read file '{}': {}", path.display(), e))?;

        SequenceDiagram::from_mermaid(&content).map_err(|e| {
            anyhow::anyhow!("Failed to parse mermaid file '{}': {}", path.display(), e)
        })?
    } else {
        SequenceDiagram::new()
    };

    run(diagram)
}

fn run(diagram: SequenceDiagram) -> anyhow::Result<()> {
    let mut terminal = ratatui::init();
    execute!(std::io::stdout())?;

    let mut world = World::default();
    app::setup_world(&mut world, diagram);

    loop {
        terminal.draw(|frame| app::render(frame, &mut world))?;

        if event::poll(std::time::Duration::from_millis(16))? {
            let active = app::active_widgets(&world);

            if let CEvent::Key(key) = event::read()? {
                InputEvent::Key(key).handle(&mut world, &active);
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
