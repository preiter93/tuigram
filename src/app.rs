use crate::{
    core::{models::Event, sequence::SequenceDiagram},
    layout::sequence::SequenceLayout,
    render::sequence::render_sequence,
    theme::Theme,
};
use anyhow::Result;
use ratatui::{Frame, layout::Rect};
use tui_world::{KeyBinding, Keybindings, WidgetId, World};

pub const GLOBAL: WidgetId = WidgetId("Global");

#[derive(Default)]
pub struct AppState {
    pub should_quit: bool,
    pub _help_open: bool,
    pub area: Rect,
}

pub fn setup_world(world: &mut World) -> Result<()> {
    world.insert(Theme::default());
    world.insert(AppState::default());

    global_keybindings(world);

    Ok(())
}

fn global_keybindings(world: &mut World) {
    let kb = world.get_mut::<Keybindings>();

    kb.bind(GLOBAL, KeyBinding::ctrl('c'), "Quit", |world| {
        world.get_mut::<AppState>().should_quit = true;
    });

    kb.bind(GLOBAL, '?', "Help", |_world| {
        // help::toggle(world);
    });
}

pub fn render(frame: &mut Frame, world: &mut World) {
    let area = frame.area();
    world.get_mut::<AppState>().area = area;

    let diagram = SequenceDiagram {
        participants: vec!["User".into(), "API".into(), "DB".into()],
        events: vec![
            Event::Message {
                from: 0,
                to: 1,
                text: "Login".into(),
            },
            Event::Message {
                from: 1,
                to: 2,
                text: "Query".into(),
            },
            Event::Message {
                from: 2,
                to: 1,
                text: "Result".into(),
            },
        ],
    };

    let layout = SequenceLayout::compute(&diagram, area.width);
    render_sequence(frame, area, &layout);
}
