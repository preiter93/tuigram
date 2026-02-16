use ratatui::style::{Color, Style};
use tui_theme_builder::ThemeBuilder;

#[derive(ThemeBuilder)]
#[builder(context = Colors)]
pub struct Theme {
    #[style(fg = text)]
    pub text: Style,

    #[style(fg = muted)]
    pub muted: Style,

    #[style(fg = accent, add_modifier = bold)]
    pub accent: Style,

    #[style(fg = success, add_modifier = bold)]
    pub success: Style,

    #[style(fg = black, bg = accent, add_modifier = bold)]
    pub status_normal: Style,

    #[style(fg = black, bg = success, add_modifier = bold)]
    pub status_input: Style,

    #[style(fg = black, bg = warning, add_modifier = bold)]
    pub status_select: Style,

    #[style(fg = black, bg = help, add_modifier = bold)]
    pub status_help: Style,

    #[style(fg = accent)]
    pub border: Style,

    #[style(fg = highlight)]
    pub selected: Style,

    #[style(fg = highlight)]
    pub key: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self::build(&Colors::default())
    }
}

pub struct Colors {
    pub text: Color,
    pub muted: Color,
    pub accent: Color,
    pub highlight: Color,
    pub success: Color,
    pub warning: Color,
    pub help: Color,
    pub black: Color,
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            text: Color::White,
            muted: Color::DarkGray,
            accent: Color::Rgb(138, 180, 248),
            highlight: Color::Yellow,
            success: Color::Green,
            warning: Color::Yellow,
            help: Color::Magenta,
            black: Color::Black,
        }
    }
}
