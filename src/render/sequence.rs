use std::mem;

use ratatui::{
    Frame,
    buffer::{Buffer, Cell},
    layout::{Alignment, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, Widget},
};
use tui_world::World;

use crate::{
    core::{
        ArrowStyle, BoxColor, DEFAULT_BOX_COLOR, Event, Fragment, NotePosition, Participant,
        ParticipantBox, SequenceDiagram, text_lines,
    },
    theme::Theme,
    ui::{
        EditorMode, EditorState, FIRST_MESSAGE_OFFSET, HEADER_HEIGHT, Selection,
        scroll::ScrollState,
    },
};

const HEADER_TOP: u16 = 1;
const ACTOR_GLYPH: &str = "☺ ";
const FRAME_PADDING: u16 = 3;
const SELF_LOOP_WIDTH: u16 = 4;
const MAX_CANVAS_WIDTH: u16 = 4096;
const PREVIEW_FRAME_TITLE: &str = "new frame";
const PREVIEW_BOX_LABEL: &str = "new box";
const DIVIDER_LABEL_ROOM: u16 = 7;
const PREVIEW_DASH: &str = "┄";

/// What the frame or box being created adds to the drawing.
enum Preview {
    /// Index of the preview inserted into the drawn diagram.
    Added(usize),
    /// Existing frames or boxes the new one would collide with.
    Collides(Vec<usize>),
    /// Frame and divider index of the divider inserted into the drawn diagram.
    Section(usize, usize),
}

/// Draws the diagram with the frame or box being created as if it already existed,
/// so layout and scrolling match the final result. The preview lives in a clone that
/// stands in for the real diagram only while drawing.
pub fn render_sequence(f: &mut Frame, outer_area: Rect, world: &mut World) {
    let editor = world.get::<EditorState>();
    if editor.frame_selection_range().is_none()
        && editor.box_selection_range().is_none()
        && editor.section_preview().is_none()
    {
        render_diagram(f, outer_area, world, None, None);
        return;
    }

    let mut preview_diagram = world.get::<SequenceDiagram>().clone();
    let frame_preview = add_frame_preview(editor, &mut preview_diagram)
        .or_else(|| add_section_preview(editor, &mut preview_diagram));
    let box_preview = add_box_preview(editor, &mut preview_diagram);

    let real = mem::replace(world.get_mut::<SequenceDiagram>(), preview_diagram);
    render_diagram(
        f,
        outer_area,
        world,
        frame_preview.as_ref(),
        box_preview.as_ref(),
    );
    *world.get_mut::<SequenceDiagram>() = real;
}

fn add_section_preview(editor: &EditorState, diagram: &mut SequenceDiagram) -> Option<Preview> {
    let at = editor.section_preview()?;
    let frame_idx = diagram.fragment_at(at)?;
    let k = diagram.add_section(frame_idx, at, editor.input_buffer.clone())?;
    Some(Preview::Section(frame_idx, k))
}

/// A range that overlaps existing frames is not previewed, those are marked instead.
fn add_frame_preview(editor: &EditorState, diagram: &mut SequenceDiagram) -> Option<Preview> {
    let range = editor.frame_selection_range()?;
    let (start, end) = (*range.start(), *range.end());
    if end >= diagram.event_count() {
        return None;
    }

    let colliding = diagram.overlapping_fragments(start, end);
    if !colliding.is_empty() {
        return Some(Preview::Collides(colliding));
    }

    let idx = diagram.fragments.partition_point(|f| f.start <= start);
    diagram.fragments.insert(
        idx,
        Fragment {
            kind: editor.frame_kind,
            label: editor.input_buffer.clone(),
            start,
            end,
            sections: Vec::new(),
        },
    );
    Some(Preview::Added(idx))
}

/// A range that overlaps existing boxes is not previewed, those are marked instead.
fn add_box_preview(editor: &EditorState, diagram: &mut SequenceDiagram) -> Option<Preview> {
    let range = editor.box_selection_range()?;
    let (start, end) = (*range.start(), *range.end());
    if end >= diagram.participant_count() {
        return None;
    }

    let colliding = diagram.overlapping_boxes(start, end);
    if !colliding.is_empty() {
        return Some(Preview::Collides(colliding));
    }

    let label = if editor.input_buffer.is_empty() {
        PREVIEW_BOX_LABEL.to_string()
    } else {
        editor.input_buffer.clone()
    };
    let idx = diagram.boxes.partition_point(|b| b.start <= start);
    diagram.boxes.insert(
        idx,
        ParticipantBox {
            label,
            color: box_color_chosen(editor).then_some(editor.box_color),
            start,
            end,
        },
    );
    Some(Preview::Added(idx))
}

/// The color is only chosen once the picker is reached.
fn box_color_chosen(editor: &EditorState) -> bool {
    editor.mode != EditorMode::SelectBoxEnd
}

fn render_diagram(
    f: &mut Frame,
    outer_area: Rect,
    world: &mut World,
    frame_preview: Option<&Preview>,
    box_preview: Option<&Preview>,
) {
    let selection = world.get::<EditorState>().selection;
    let area = outer_area.inner(Margin::new(0, 1));

    let scroll = world.get_mut::<ScrollState>();
    scroll.set_viewport(area.height);
    scroll.hit_areas.clear();
    scroll.diagram_area = area;

    let visible_event = match selection {
        Selection::Event(idx) => Some(idx),
        Selection::Fragment(idx) => world
            .get::<SequenceDiagram>()
            .fragments
            .get(idx)
            .map(|f| f.start),
        Selection::Section(frame_idx, k) => world
            .get::<SequenceDiagram>()
            .fragments
            .get(frame_idx)
            .and_then(|f| f.sections.get(k))
            .map(|(at, _)| *at),
        _ => None,
    };
    if let Some(idx) = visible_event
        && world.get::<ScrollState>().follow
    {
        let diagram = world.get::<SequenceDiagram>().clone();
        world.get_mut::<ScrollState>().ensure_visible(idx, &diagram);
    }

    let theme = world.get::<Theme>();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border);
    f.render_widget(block, outer_area);

    if area.is_empty() {
        return;
    }

    let (participants, canvas_width) = participant_layout(area, world.get::<SequenceDiagram>());
    let canvas = Rect {
        width: canvas_width,
        ..area
    };
    let mut buf = Buffer::empty(canvas);

    let lifeline_start = area.y + HEADER_HEIGHT;
    let rows = event_rows(world, lifeline_start + FIRST_MESSAGE_OFFSET);
    render_box_outlines(&mut buf, canvas, world, &participants, &rows, box_preview);
    render_participants(&mut buf, canvas, world, &participants);
    render_lifelines(&mut buf, canvas, world, &participants, lifeline_start);
    render_fragments(&mut buf, canvas, world, &participants, &rows, frame_preview);
    render_events(&mut buf, world, &participants, &rows);

    let (content_left, content_right) = content_columns(&buf);
    let max_offset = (content_right + 3)
        .saturating_sub(area.width)
        .min(canvas_width.saturating_sub(area.width));
    let follow = world.get::<ScrollState>().follow;
    if follow && let Some((left, right)) = selection_span(canvas, world, &participants) {
        let scroll = world.get_mut::<ScrollState>();
        scroll.follow_x(left, right, area.width, max_offset);

        // Don't leave blank columns hidden when the span fits without scrolling.
        if scroll.x_offset < content_left && right + 2 <= area.width {
            scroll.x_offset = 0;
        }

        // Likewise at the right end: keep a blank column before the border.
        if scroll.x_offset > 0 && right >= content_right && left > max_offset {
            scroll.x_offset = max_offset;
        }
    }
    let offset = world.get::<ScrollState>().x_offset.min(max_offset);
    world.get_mut::<ScrollState>().x_offset = offset;

    let areas = hit_areas(canvas, world, &participants, &rows);
    world.get_mut::<ScrollState>().hit_areas = areas;

    copy_visible_columns(f, &buf, area, offset);
    let hidden_left = offset >= content_left && offset > 0;
    let hidden_right = offset + area.width.saturating_sub(2) < content_right;
    render_scroll_markers(f, area, world, hidden_left, hidden_right);
    render_scrollbar(f, area, world);
}

/// Canvas columns (inclusive) the selection covers, if it has any.
fn selection_span(canvas: Rect, world: &World, positions: &[u16]) -> Option<(u16, u16)> {
    let diagram = world.get::<SequenceDiagram>();

    match world.get::<EditorState>().selection {
        Selection::Participant(i) => {
            let participant = diagram.participants.get(i)?;
            let (x, width) = header_span(canvas, positions[i], participant);
            Some((x, x + width - 1))
        }
        Selection::Box(i) => box_spans(canvas, world, positions)
            .get(i)
            .copied()
            .flatten(),
        Selection::Event(i) => {
            let event = diagram.events.get(i)?;
            Some(event_extent(event, &event_text(diagram, i), positions))
        }
        Selection::Fragment(i) | Selection::Section(i, _) => {
            let frame = diagram.fragments.get(i)?;
            let title = frame_title(frame.kind.as_mermaid_str(), &frame.label);
            let (left, width, _) = frame_bounds(canvas, diagram, frame, &title, positions)?;
            Some((left, left + width - 1))
        }
        Selection::None => None,
    }
}

/// First and last canvas column (outside the border column) that has anything drawn.
fn content_columns(buf: &Buffer) -> (u16, u16) {
    let area = buf.area;
    let is_blank = |x: u16| (area.top()..area.bottom()).all(|y| buf[(x, y)] == Cell::EMPTY);
    let columns = area.x + 1..area.right();

    let left = columns.clone().find(|&x| !is_blank(x)).unwrap_or(0);
    let right = columns.rev().find(|&x| !is_blank(x)).unwrap_or(0);

    (left, right)
}

fn render_scroll_markers(
    f: &mut Frame,
    area: Rect,
    world: &World,
    hidden_left: bool,
    hidden_right: bool,
) {
    if area.width < 3 {
        return;
    }

    let style = world.get::<Theme>().muted;
    let y = area.y + HEADER_TOP + 1;

    if hidden_left {
        f.buffer_mut().set_string(area.x, y, "‹", style);
    }
    if hidden_right {
        f.buffer_mut().set_string(area.right() - 1, y, "›", style);
    }
}

fn render_scrollbar(f: &mut Frame, area: Rect, world: &World) {
    let diagram = world.get::<SequenceDiagram>();
    let scroll = world.get::<ScrollState>();

    if !scroll.needs_scroll(diagram) {
        return;
    }

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
    let scrollbar_area = Rect {
        x: area.x,
        y: area.y + HEADER_HEIGHT,
        width: area.width,
        height: area.height.saturating_sub(HEADER_HEIGHT),
    };
    let mut scrollbar_state = scroll.scrollbar_state(diagram);
    f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
}

/// Copies the canvas columns shown inside the border, shifted left by `offset`.
fn copy_visible_columns(f: &mut Frame, canvas: &Buffer, area: Rect, offset: u16) {
    let first = area.x + 1;
    let last = area.right().saturating_sub(1);

    for y in area.y..area.bottom() {
        for x in first..last {
            let Some(cell) = canvas.cell((x + offset, y)) else {
                continue;
            };
            if let Some(target) = f.buffer_mut().cell_mut((x, y)) {
                *target = cell.clone();

                // A wide glyph in the last column would spill onto the border.
                if x + 1 == last && text_width(cell.symbol()) > 1 {
                    target.set_symbol(" ");
                }
            }
        }
    }
}

/// Lifeline x of every participant and the canvas width the diagram needs.
fn participant_layout(area: Rect, diagram: &SequenceDiagram) -> (Vec<u16>, u16) {
    let count = diagram.participants.len();
    let gaps = column_gaps(diagram, area.width);
    let mut spacing = area.width / (count.max(1) as u16 + 1);

    let total = |spacing: u16| -> u32 { gaps.iter().map(|&g| u32::from(spacing.max(g))).sum() };
    while spacing > 0 && total(spacing) > u32::from(area.width) {
        spacing -= 1;
    }

    let mut positions = Vec::with_capacity(count);
    let mut x = 0;
    for &gap in &gaps[..count] {
        x = u16::saturating_add(x, spacing.max(gap));
        positions.push(x);
    }
    let mut content_width = x.saturating_add(spacing.max(gaps[count]));

    // Wide notes and labels that the gap rules skip still need to be reachable.
    for (i, event) in diagram.events.iter().enumerate() {
        let (_, right) = event_extent(event, &event_text(diagram, i), &positions);
        content_width = content_width.max(right.saturating_add(3));
    }

    let canvas_width = area.width.max(content_width.min(MAX_CANVAS_WIDTH));
    (positions, canvas_width)
}

/// Minimum room left of every participant, plus one entry for the room right of the last.
fn column_gaps(diagram: &SequenceDiagram, max_gap: u16) -> Vec<u16> {
    let count = diagram.participants.len();
    let mut gaps = vec![0u16; count + 1];
    if count == 0 {
        return gaps;
    }

    let widths: Vec<u16> = diagram
        .participants
        .iter()
        .map(|p| text_width(&header_text(p)) + 4)
        .collect();
    let box_of = |i: usize| {
        diagram
            .boxes
            .iter()
            .position(|b| b.start <= i && i <= b.end)
    };

    gaps[0] = widths[0] / 2 + 2;
    gaps[count] = widths[count - 1] / 2 + 2;
    for i in 1..count {
        gaps[i] = u16::midpoint(widths[i - 1], widths[i]) + 2;
        if box_of(i - 1) != box_of(i) {
            gaps[i] += 3;
        }
    }
    if box_of(0).is_some() {
        gaps[0] += 2;
    }
    if box_of(count - 1).is_some() {
        gaps[count] += 2;
    }

    for (i, event) in diagram.events.iter().enumerate() {
        let text = label_width(&event_text(diagram, i));
        match event {
            Event::Message { from, to, .. } if from == to => {
                gaps[from + 1] = gaps[from + 1].max(SELF_LOOP_WIDTH + text + 1);
            }
            Event::Message { from, to, .. } if from.abs_diff(*to) == 1 => {
                let right = *from.max(to);
                gaps[right] = gaps[right].max(text + 4);
            }
            Event::Message { .. } => {}
            Event::Note {
                position,
                participant_start,
                participant_end,
                ..
            } => {
                if participant_start != participant_end {
                    continue;
                }
                let i = *participant_start;
                match position {
                    NotePosition::Right => gaps[i + 1] = gaps[i + 1].max(text + 7),
                    NotePosition::Left => gaps[i] = gaps[i].max(text + 7),
                    NotePosition::Over => {
                        let half = u16::midpoint(text, 4) + 2;
                        gaps[i] = gaps[i].max(half);
                        gaps[i + 1] = gaps[i + 1].max(half);
                    }
                }
            }
        }
    }

    // Room for the border column, and for frames drawn around edge events.
    gaps[count] += 1;
    if !diagram.fragments.is_empty() {
        gaps[0] += FRAME_PADDING;
        gaps[count] += FRAME_PADDING;
    }

    for gap in &mut gaps {
        *gap = (*gap).min(max_gap);
    }
    gaps
}

/// Left edge and width of a participant's header box.
fn header_span(area: Rect, center: u16, participant: &Participant) -> (u16, u16) {
    let width = text_width(&header_text(participant)) + 4;
    let x = center
        .saturating_sub(width / 2)
        .min(area.width.saturating_sub(width));
    (x, width)
}

fn render_participants(buf: &mut Buffer, area: Rect, world: &World, positions: &[u16]) {
    let diagram = world.get::<SequenceDiagram>();
    let editor = world.get::<EditorState>();
    let selection = editor.selection;
    let box_range = editor.box_selection_range();
    let theme = world.get::<Theme>();

    for (i, participant) in diagram.participants.iter().enumerate() {
        let style = if selection == Selection::Participant(i) {
            theme.selected
        } else if box_range.as_ref().is_some_and(|range| range.contains(&i)) {
            theme.accent
        } else {
            theme.text
        };

        let (x, width) = header_span(area, positions[i], participant);

        Paragraph::new(header_text(participant))
            .alignment(Alignment::Center)
            .style(style)
            .block(Block::default().borders(Borders::ALL).border_style(style))
            .render(
                Rect {
                    x,
                    y: area.y + HEADER_TOP,
                    width,
                    height: 3,
                },
                buf,
            );
    }
}

fn header_text(p: &Participant) -> String {
    if p.actor {
        format!("{ACTOR_GLYPH}{}", p.display_name())
    } else {
        p.display_name().to_string()
    }
}

/// Columns (inclusive) a box outline covers: a border column and a blank column
/// around the headers. Neighbouring boxes never overlap.
fn box_spans(area: Rect, world: &World, positions: &[u16]) -> Vec<Option<(u16, u16)>> {
    let diagram = world.get::<SequenceDiagram>();
    let min_left = area.x + 1;
    let max_right = area.right().saturating_sub(2);
    let mut previous_right: Option<u16> = None;
    let mut spans = Vec::new();

    for b in &diagram.boxes {
        if b.end >= positions.len() {
            spans.push(None);
            continue;
        }

        let (first_x, _) = header_span(area, positions[b.start], &diagram.participants[b.start]);
        let (last_x, last_w) = header_span(area, positions[b.end], &diagram.participants[b.end]);

        let mut left = (area.x + first_x).saturating_sub(2).max(min_left);
        if let Some(right) = previous_right {
            left = left.max(right + 1);
        }
        let right = (area.x + last_x + last_w + 1).min(max_right);

        if right < left + 1 {
            spans.push(None);
            continue;
        }

        // Room up to the neighbouring headers, leaving room for their box border.
        let lower = match b.start.checked_sub(1) {
            Some(prev) => {
                let (x, w) = header_span(area, positions[prev], &diagram.participants[prev]);
                area.x + x + w + 2
            }
            None => min_left,
        };
        let upper = match diagram.participants.get(b.end + 1) {
            Some(next) => {
                let (x, _) = header_span(area, positions[b.end + 1], next);
                (area.x + x).saturating_sub(3)
            }
            None => max_right,
        };
        let lower = lower
            .max(previous_right.map_or(min_left, |r| r + 1))
            .min(left);
        let upper = upper.min(max_right).max(right);

        let (left, right) = widen_for_label(left, right, &b.label, lower, upper);

        previous_right = Some(right);
        spans.push(Some((left, right)));
    }

    spans
}

/// Grows a span symmetrically so `╭─ label ─╮` fits, as far as the limits allow.
fn widen_for_label(left: u16, right: u16, label: &str, lower: u16, upper: u16) -> (u16, u16) {
    if label.is_empty() {
        return (left, right);
    }

    let wanted = text_width(label) + 6;
    let width = right - left + 1;
    if width >= wanted {
        return (left, right);
    }

    let extra = wanted - width;
    let new_left = left.saturating_sub(extra / 2).max(lower);
    let new_right = (right + extra - extra / 2).min(upper);
    if new_right - new_left + 1 >= wanted {
        return (new_left, new_right);
    }

    // One side ran out of room: give the rest to the other side.
    let new_left = (new_right + 1).saturating_sub(wanted).max(lower);
    let new_right = (new_left + wanted - 1).min(upper);
    (new_left, new_right)
}

fn render_box_outlines(
    buf: &mut Buffer,
    area: Rect,
    world: &World,
    positions: &[u16],
    rows: &[(usize, u16)],
    preview: Option<&Preview>,
) {
    let diagram = world.get::<SequenceDiagram>();
    let theme = world.get::<Theme>();
    let editor = world.get::<EditorState>();
    let selection = editor.selection;
    let area_bottom = area.y + area.height.saturating_sub(1);

    let event_count = diagram.event_count();
    let has_below = world.get::<ScrollState>().visible_range(diagram).end < event_count;
    let content_end = match rows.last() {
        Some(&(i, y)) => y + diagram.event_height(i) - diagram.rows_above(i),
        None => area.y + HEADER_HEIGHT + FIRST_MESSAGE_OFFSET,
    };
    let bottom = if has_below {
        area_bottom
    } else {
        content_end.min(area_bottom)
    };

    let top = area.y;
    if bottom < top + HEADER_HEIGHT {
        return;
    }

    let spans = box_spans(area, world, positions);

    for (box_idx, (b, span)) in diagram.boxes.iter().zip(spans).enumerate() {
        let Some((left, right)) = span else {
            continue;
        };

        let is_preview = matches!(preview, Some(Preview::Added(idx)) if *idx == box_idx);
        let style = match preview {
            Some(Preview::Added(idx)) if *idx == box_idx => {
                let color = box_color_chosen(editor).then_some(editor.box_color);
                color.map_or(theme.selected, |c| Style::default().fg(box_color(c)))
            }
            Some(Preview::Collides(indices)) if indices.contains(&box_idx) => theme.error,
            _ if selection == Selection::Box(box_idx) => {
                theme.selected.add_modifier(Modifier::BOLD)
            }
            _ => Style::default().fg(box_color(b.color.unwrap_or(DEFAULT_BOX_COLOR))),
        };
        let width = right - left + 1;

        Paragraph::new(box_top_edge(&b.label, width, is_preview))
            .style(style)
            .render(
                Rect {
                    x: left,
                    y: top,
                    width,
                    height: 1,
                },
                buf,
            );

        for y in top + 1..bottom {
            for x in [left, right] {
                Paragraph::new("│").style(style).render(
                    Rect {
                        x,
                        y,
                        width: 1,
                        height: 1,
                    },
                    buf,
                );
            }
        }

        let dashes = horizontal_rule(usize::from(width - 2), is_preview);
        Paragraph::new(format!("╰{dashes}╯")).style(style).render(
            Rect {
                x: left,
                y: bottom,
                width,
                height: 1,
            },
            buf,
        );
    }
}

fn box_top_edge(label: &str, width: u16, dashed: bool) -> String {
    let inner = usize::from(width - 2);
    let dash = dash_glyph(dashed);
    // "─ " before and " ─" after the label
    let label_room = width.saturating_sub(6);
    if label.is_empty() || label_room == 0 {
        return format!("╭{}╮", dash.repeat(inner));
    }

    let label = truncate_to_width(label, label_room);
    let rest = inner.saturating_sub(text_width(&label) as usize + 3);
    format!("╭{dash} {label} {}╮", dash.repeat(rest))
}

fn dash_glyph(dashed: bool) -> &'static str {
    if dashed { PREVIEW_DASH } else { "─" }
}

fn horizontal_rule(len: usize, dashed: bool) -> String {
    dash_glyph(dashed).repeat(len)
}

pub fn box_color(color: BoxColor) -> Color {
    match color {
        BoxColor::Blue => Color::Rgb(100, 150, 220),
        BoxColor::Green => Color::Rgb(80, 180, 100),
        BoxColor::Red => Color::Rgb(220, 80, 80),
        BoxColor::Yellow => Color::Rgb(200, 180, 50),
        BoxColor::Orange => Color::Rgb(220, 130, 50),
        BoxColor::Purple => Color::Rgb(170, 80, 200),
        BoxColor::Aqua => Color::Rgb(60, 190, 200),
        BoxColor::Gray => Color::Rgb(150, 150, 150),
    }
}

fn render_lifelines(
    buf: &mut Buffer,
    area: Rect,
    world: &World,
    participants: &[u16],
    lifeline_start: u16,
) {
    let diagram = world.get::<SequenceDiagram>();
    let scroll = world.get::<ScrollState>();
    let theme = world.get::<Theme>();
    let event_count = diagram.event_count();

    let visible_range = scroll.visible_range(diagram);
    let has_above = visible_range.start > 0;
    let has_below = visible_range.end < event_count;

    for &x in participants {
        let xi = x.min(area.width.saturating_sub(1));
        for y in lifeline_start..area.y + area.height {
            Paragraph::new("│").style(theme.text).render(
                Rect {
                    x: xi,
                    y,
                    width: 1,
                    height: 1,
                },
                buf,
            );
        }

        // Show scroll indicators on lifelines
        if has_above {
            Paragraph::new("⋮").style(theme.muted).render(
                Rect {
                    x: xi,
                    y: lifeline_start,
                    width: 1,
                    height: 1,
                },
                buf,
            );
        }

        if has_below {
            Paragraph::new("⋮").style(theme.muted).render(
                Rect {
                    x: xi,
                    y: area.y + area.height - 1,
                    width: 1,
                    height: 1,
                },
                buf,
            );
        }
    }
}

/// Anchor row of every visible event, leaving room for frame header and divider rows.
fn event_rows(world: &World, base_y: u16) -> Vec<(usize, u16)> {
    let diagram = world.get::<SequenceDiagram>();
    let visible_range = world.get::<ScrollState>().visible_range(diagram);

    let mut y = base_y;
    let mut rows = Vec::new();

    for i in visible_range {
        let above = diagram.rows_above(i);
        y += above;
        rows.push((i, y));
        y += diagram.event_height(i) - above;
    }

    rows
}

fn anchor_row(rows: &[(usize, u16)], idx: usize) -> Option<u16> {
    rows.iter().find(|(i, _)| *i == idx).map(|(_, y)| *y)
}

/// Row of the frame header or divider drawn above an event.
fn header_row(rows: &[(usize, u16)], idx: usize) -> Option<u16> {
    anchor_row(rows, idx).map(|y| y - 2)
}

/// Row of the frame footer drawn below an event.
fn footer_row(rows: &[(usize, u16)], diagram: &SequenceDiagram, idx: usize) -> Option<u16> {
    anchor_row(rows, idx).map(|y| y + diagram.events[idx].height() - 1)
}

/// Recomputes the hit areas for the current diagram and scroll position without drawing.
pub fn refresh_hit_areas(world: &mut World) {
    let area = world.get::<ScrollState>().diagram_area;
    if area.is_empty() {
        return;
    }

    let (participants, canvas_width) = participant_layout(area, world.get::<SequenceDiagram>());
    let canvas = Rect {
        width: canvas_width,
        ..area
    };
    let rows = event_rows(world, area.y + HEADER_HEIGHT + FIRST_MESSAGE_OFFSET);

    let areas = hit_areas(canvas, world, &participants, &rows);
    world.get_mut::<ScrollState>().hit_areas = areas;
}

/// Clickable areas in draw order, so later (topmost) items win where they overlap.
fn hit_areas(
    canvas: Rect,
    world: &World,
    participants: &[u16],
    rows: &[(usize, u16)],
) -> Vec<(Rect, Selection)> {
    let diagram = world.get::<SequenceDiagram>();
    let mut areas = Vec::new();

    let spans = box_spans(canvas, world, participants);
    for (i, span) in spans.into_iter().enumerate() {
        if let Some((left, right)) = span {
            areas.push((
                Rect::new(left, canvas.y, right - left + 1, 1),
                Selection::Box(i),
            ));
        }
    }

    for (i, participant) in diagram.participants.iter().enumerate() {
        let (x, width) = header_span(canvas, participants[i], participant);
        let area = Rect::new(x, canvas.y + HEADER_TOP, width, 3);
        areas.push((area, Selection::Participant(i)));
    }

    for (frame_idx, frame) in diagram.fragments.iter().enumerate() {
        let title = frame_title(frame.kind.as_mermaid_str(), &frame.label);
        let Some((left, width, _)) = frame_bounds(canvas, diagram, frame, &title, participants)
        else {
            continue;
        };

        let selection = Selection::Fragment(frame_idx);
        if let Some(y) = header_row(rows, frame.start) {
            areas.push((Rect::new(left, y, width, 1), selection));
        }
        if let Some(y) = footer_row(rows, diagram, frame.end) {
            areas.push((Rect::new(left, y, width, 1), selection));
        }

        for (k, (at, _)) in frame.sections.iter().enumerate() {
            if let Some(y) = header_row(rows, *at) {
                areas.push((
                    Rect::new(left, y, width, 1),
                    Selection::Section(frame_idx, k),
                ));
            }
        }
    }

    for &(i, y) in rows {
        let event = &diagram.events[i];
        let (left, right) = event_extent(event, &event_text(diagram, i), participants);
        let area = Rect::new(left, y - 1, right - left + 1, event.height());
        areas.push((area, Selection::Event(i)));
    }

    areas
}

fn render_fragments(
    buf: &mut Buffer,
    area: Rect,
    world: &World,
    participants: &[u16],
    rows: &[(usize, u16)],
    preview: Option<&Preview>,
) {
    let diagram = world.get::<SequenceDiagram>();
    let theme = world.get::<Theme>();
    let (Some(&(first_visible, _)), Some(&(last_visible, _))) = (rows.first(), rows.last()) else {
        return;
    };

    let first_row = area.y + HEADER_HEIGHT;
    let last_row = area.y + area.height - 1;
    let editor = world.get::<EditorState>();
    let selection = editor.selection;

    for (frame_idx, frame) in diagram.fragments.iter().enumerate() {
        let is_preview = matches!(preview, Some(Preview::Added(idx)) if *idx == frame_idx);
        let (border_style, title_style) = match preview {
            Some(Preview::Added(idx)) if *idx == frame_idx => (theme.selected, theme.selected),
            Some(Preview::Collides(indices)) if indices.contains(&frame_idx) => {
                (theme.error, theme.error)
            }
            _ if selection == Selection::Fragment(frame_idx) => (theme.selected, theme.selected),
            _ => (theme.muted, theme.accent),
        };

        if frame.end < first_visible || frame.start > last_visible {
            continue;
        }

        let header_y = header_row(rows, frame.start);
        let footer_y = footer_row(rows, diagram, frame.end);

        let title = if is_preview && editor.mode == EditorMode::SelectFrameEnd {
            PREVIEW_FRAME_TITLE.to_string()
        } else {
            frame_title(frame.kind.as_mermaid_str(), &frame.label)
        };
        let Some((left, width, title)) = frame_bounds(area, diagram, frame, &title, participants)
        else {
            continue;
        };
        let title_width = text_width(&title);
        let right = left + width - 1;

        let top = header_y.unwrap_or(first_row);
        let bottom = footer_y.unwrap_or(last_row);

        for y in top.max(first_row)..=bottom.min(last_row) {
            let row = Rect {
                x: left,
                y,
                width,
                height: 1,
            };

            if Some(y) == header_y {
                let dashes = usize::from(width.saturating_sub(title_width + 4).max(1));
                let line = Line::from(vec![
                    Span::styled("┌ ", border_style),
                    Span::styled(title.as_str(), title_style),
                    Span::styled(
                        format!(" {}┐", horizontal_rule(dashes, is_preview)),
                        border_style,
                    ),
                ]);
                Paragraph::new(line).render(row, buf);
            } else if Some(y) == footer_y {
                let dashes = horizontal_rule(usize::from(width.saturating_sub(2)), is_preview);
                Paragraph::new(format!("└{dashes}┘"))
                    .style(border_style)
                    .render(row, buf);
            } else {
                for x in [left, right] {
                    Paragraph::new("│")
                        .style(border_style)
                        .render(Rect { x, width: 1, ..row }, buf);
                }
            }
        }

        for (k, (at, label)) in frame.sections.iter().enumerate() {
            let Some(y) = header_row(rows, *at) else {
                continue;
            };
            if y < first_row || y > last_row {
                continue;
            }

            let is_preview_divider =
                matches!(preview, Some(Preview::Section(f, i)) if (*f, *i) == (frame_idx, k));
            let (line_style, label_style) =
                if is_preview_divider || selection == Selection::Section(frame_idx, k) {
                    (theme.selected, theme.selected)
                } else {
                    (border_style, title_style)
                };
            let row = Rect {
                x: left,
                y,
                width,
                height: 1,
            };
            let line = divider_line(label, width, line_style, label_style);
            Paragraph::new(line).render(row, buf);
        }
    }
}

/// Dashed rule across a frame: `├┄ [label] ┄┄┤`, or only dashes without a label.
fn divider_line(label: &str, width: u16, line_style: Style, label_style: Style) -> Line<'static> {
    let max_label = width.saturating_sub(DIVIDER_LABEL_ROOM);
    if label.is_empty() || max_label == 0 {
        let dashes = horizontal_rule(usize::from(width.saturating_sub(2)), true);
        return Line::from(Span::styled(format!("├{dashes}┤"), line_style));
    }

    let label = format!("[{}]", truncate_to_width(label, max_label));
    let dashes = usize::from(width.saturating_sub(text_width(&label) + 5));
    Line::from(vec![
        Span::styled("├┄ ", line_style),
        Span::styled(label, label_style),
        Span::styled(format!(" {}┤", horizontal_rule(dashes, true)), line_style),
    ])
}

/// Left x, width and (possibly truncated) title of a frame, or None if it cannot fit.
fn frame_bounds(
    area: Rect,
    diagram: &SequenceDiagram,
    frame: &Fragment,
    full_title: &str,
    participants: &[u16],
) -> Option<(u16, u16, String)> {
    let extents: Vec<(u16, u16)> = (frame.start..=frame.end)
        .map(|i| event_extent(&diagram.events[i], &event_text(diagram, i), participants))
        .collect();
    let min_x = extents.iter().map(|e| e.0).min().unwrap_or(0);
    let max_x = extents.iter().map(|e| e.1).max().unwrap_or(0);

    let min_left = area.x + 1;
    let max_right = area.right().saturating_sub(2);
    let available = (max_right + 1).saturating_sub(min_left);
    if available < 6 {
        return None;
    }

    let title = truncate_to_width(full_title, available - 5);
    let desired_left = min_x.saturating_sub(FRAME_PADDING).max(min_left);
    let width = (max_x + FRAME_PADDING + 1)
        .saturating_sub(desired_left)
        .max(text_width(&title) + 5)
        .max(divider_min_width(frame))
        .min(available);
    let left = desired_left.min(max_right + 1 - width);

    Some((left, width, title))
}

/// Width a frame needs to show its widest divider label untruncated.
fn divider_min_width(frame: &Fragment) -> u16 {
    let widest = frame.sections.iter().map(|(_, label)| text_width(label));
    widest
        .max()
        .map_or(0, |w| if w == 0 { 0 } else { w + DIVIDER_LABEL_ROOM })
}

fn truncate_to_width(text: &str, max_width: u16) -> String {
    if text_width(text) <= max_width {
        return text.to_string();
    }

    let mut width = 0;
    let mut result = String::new();

    for c in text.chars() {
        width += text_width(&c.to_string());
        if width >= max_width {
            break;
        }
        result.push(c);
    }

    format!("{}…", result.trim_end())
}

fn frame_title(kind: &str, label: &str) -> String {
    if label.is_empty() {
        kind.to_string()
    } else {
        format!("{kind} [{label}]")
    }
}

fn render_events(buf: &mut Buffer, world: &World, participants: &[u16], rows: &[(usize, u16)]) {
    let diagram = world.get::<SequenceDiagram>();
    let editor = world.get::<EditorState>();
    let theme = world.get::<Theme>();

    let frame_range = editor.frame_selection_range();

    for &(i, y) in rows {
        let event = &diagram.events[i];
        let label = event_text(diagram, i);

        let style = if editor.selection == Selection::Event(i) {
            theme.selected
        } else if frame_range.as_ref().is_some_and(|range| range.contains(&i)) {
            theme.accent
        } else {
            theme.text
        };

        match event {
            Event::Message {
                from,
                to,
                style: arrow_style,
                ..
            } => {
                render_message(
                    buf,
                    participants[*from],
                    participants[*to],
                    &label,
                    *arrow_style,
                    y,
                    style,
                );
            }
            Event::Note {
                position,
                participant_start,
                participant_end,
                ..
            } => {
                render_note(
                    buf,
                    participants,
                    *position,
                    *participant_start,
                    *participant_end,
                    &label,
                    y,
                    style,
                );
            }
        }
    }
}

fn line_glyph(arrow_style: ArrowStyle) -> char {
    if arrow_style.is_dashed() {
        '┄'
    } else {
        '─'
    }
}

fn head_glyph(arrow_style: ArrowStyle, pointing_right: bool) -> Option<char> {
    match arrow_style {
        ArrowStyle::SolidArrow | ArrowStyle::DashedArrow => {
            Some(if pointing_right { '▶' } else { '◀' })
        }
        ArrowStyle::Solid | ArrowStyle::Dashed => None,
        ArrowStyle::SolidCross | ArrowStyle::DashedCross => Some('×'),
        ArrowStyle::SolidAsync | ArrowStyle::DashedAsync => {
            Some(if pointing_right { '▷' } else { '◁' })
        }
    }
}

fn render_message(
    buf: &mut Buffer,
    from_x: u16,
    to_x: u16,
    text: &str,
    arrow_style: ArrowStyle,
    y: u16,
    style: ratatui::style::Style,
) {
    let lines = text_lines(text);
    let line_count = lines.len() as u16;

    if from_x == to_x {
        let mut area = Rect {
            x: from_x,
            y: y.saturating_sub(1),
            width: SELF_LOOP_WIDTH,
            height: 1,
        };

        let line = line_glyph(arrow_style);
        let head = head_glyph(arrow_style, false).unwrap_or(line);

        Paragraph::new(format!("{line}{line}{line}┐"))
            .style(style)
            .render(area, buf);

        for (k, label) in lines.into_iter().enumerate() {
            let row = y + k as u16;

            // Start after the lifeline column so the lifeline stays visible.
            let middle = Rect {
                x: from_x + 1,
                y: row,
                width: SELF_LOOP_WIDTH - 1,
                height: 1,
            };
            Paragraph::new("  │").style(style).render(middle, buf);

            let label_area = Rect {
                x: from_x + SELF_LOOP_WIDTH,
                y: row,
                width: text_width(label),
                height: 1,
            };
            Paragraph::new(label).style(style).render(label_area, buf);
        }

        area.y = y + line_count;
        Paragraph::new(format!("{head}{line}{line}┘"))
            .style(style)
            .render(area, buf);
    } else {
        let start = from_x.min(to_x);
        let end = from_x.max(to_x);
        let len = end - start;

        let line = line_glyph(arrow_style);
        let pointing_right = from_x < to_x;
        let mut arrow = line.to_string().repeat(len as usize);
        let head = head_glyph(arrow_style, pointing_right).unwrap_or(line);
        if pointing_right {
            arrow.push(head);
        } else {
            arrow.insert(0, head);
        }

        Paragraph::new(Line::from(arrow)).style(style).render(
            Rect {
                x: start,
                y: y + line_count - 1,
                width: len + 1,
                height: 1,
            },
            buf,
        );

        for (k, label) in lines.into_iter().enumerate() {
            let width = text_width(label);
            let text_start = message_text_start(start, len, width);

            Paragraph::new(label)
                .alignment(Alignment::Center)
                .style(style)
                .render(
                    Rect {
                        x: text_start,
                        y: (y + k as u16).saturating_sub(1),
                        width,
                        height: 1,
                    },
                    buf,
                );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_note(
    buf: &mut Buffer,
    participants: &[u16],
    position: NotePosition,
    participant_start: usize,
    participant_end: usize,
    text: &str,
    y: u16,
    style: ratatui::style::Style,
) {
    let lines = text_lines(text);
    let (x, width) = note_span(
        position,
        participants[participant_start],
        participants[participant_end],
        label_width(text),
    );

    let area = Rect {
        x,
        y: y.saturating_sub(1),
        width,
        height: lines.len() as u16,
    }
    .intersection(buf.area);

    Clear.render(area, buf);
    Paragraph::new(lines.into_iter().map(Line::from).collect::<Vec<_>>())
        .alignment(Alignment::Center)
        .style(style.reversed())
        .render(area, buf);
}

fn event_text(diagram: &SequenceDiagram, idx: usize) -> String {
    let text = diagram.events[idx].text();
    match diagram.message_number(idx) {
        Some(n) => format!("{n}. {text}"),
        None => text.to_string(),
    }
}

/// Left and right x (inclusive) of everything an event draws.
fn event_extent(event: &Event, text: &str, participants: &[u16]) -> (u16, u16) {
    match event {
        Event::Message { from, to, .. } => {
            let (from_x, to_x) = (participants[*from], participants[*to]);
            let width = label_width(text);

            if from_x == to_x {
                let right = from_x + SELF_LOOP_WIDTH + width.max(1) - 1;
                return (from_x, right.max(from_x + SELF_LOOP_WIDTH - 1));
            }

            let (start, end) = (from_x.min(to_x), from_x.max(to_x));
            let text_start = message_text_start(start, end - start, width);
            let text_end = text_start + width.max(1) - 1;
            (start.min(text_start), end.max(text_end))
        }
        Event::Note {
            position,
            participant_start,
            participant_end,
            ..
        } => {
            let (x, width) = note_span(
                *position,
                participants[*participant_start],
                participants[*participant_end],
                label_width(text),
            );
            (x, x + width - 1)
        }
    }
}

fn message_text_start(start: u16, len: u16, text_width: u16) -> u16 {
    (start + len.div_ceil(2)).saturating_sub(text_width / 2)
}

/// Left x and width of a note, given the text width.
fn note_span(position: NotePosition, start_x: u16, end_x: u16, text_width: u16) -> (u16, u16) {
    match position {
        NotePosition::Right => (start_x.saturating_add(2), text_width + 4),
        NotePosition::Left => {
            let w = text_width + 4;
            (start_x.saturating_sub(w + 1), w)
        }
        NotePosition::Over => {
            let min_x = start_x.min(end_x);
            let max_x = start_x.max(end_x);
            let span = max_x.saturating_sub(min_x);
            let w = span.max(text_width + 2) + 2;
            let x = if span > 0 {
                min_x.saturating_sub(1)
            } else {
                min_x.saturating_sub(w / 2)
            };
            (x, w)
        }
    }
}

fn text_width(text: &str) -> u16 {
    Line::from(text).width() as u16
}

fn label_width(text: &str) -> u16 {
    text_lines(text)
        .into_iter()
        .map(text_width)
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout(source: &str, width: u16) -> (Vec<u16>, u16) {
        let diagram = SequenceDiagram::from_mermaid(source).unwrap();
        participant_layout(Rect::new(0, 0, width, 20), &diagram)
    }

    fn eight_participants(messages: &str) -> String {
        let names = (1..=8)
            .map(|i| format!("participant Participant{i}\n"))
            .collect::<Vec<String>>()
            .concat();
        format!("sequenceDiagram\n{names}{messages}")
    }

    #[test]
    fn short_diagram_keeps_even_spacing() {
        let source = "sequenceDiagram\nA->>B: hi\nB->>C: yo\n";

        assert_eq!(layout(source, 80), (vec![20, 40, 60], 80));
    }

    #[test]
    fn crowded_headers_widen_the_canvas() {
        let messages = (1..8)
            .map(|i| {
                format!(
                    "Participant{i}->>Participant{}: a long message label\n",
                    i + 1
                )
            })
            .collect::<Vec<String>>()
            .concat();
        let source = eight_participants(&messages);
        let diagram = SequenceDiagram::from_mermaid(&source).unwrap();

        let (positions, width) = layout(&source, 80);
        assert!(width > 80);

        let label = text_width("a long message label");
        for pair in positions.windows(2) {
            assert!(pair[1] - pair[0] >= label + 2);
        }
        for (i, pair) in positions.windows(2).enumerate() {
            let (x0, w0) = header_span(
                Rect::new(0, 0, width, 20),
                pair[0],
                &diagram.participants[i],
            );
            let (x1, _) = header_span(
                Rect::new(0, 0, width, 20),
                pair[1],
                &diagram.participants[i + 1],
            );
            assert!(x0 + w0 < x1, "headers {i} and {} overlap", i + 1);
        }
    }

    #[test]
    fn widest_line_drives_gap_width() {
        let wide = "a much longer second line";
        let (positions, _) = layout(
            &format!("sequenceDiagram\nparticipant A\nparticipant B\nA->>B: hi<br/>{wide}\n"),
            200,
        );

        assert!(positions[1] - positions[0] > text_width(wide));
    }

    #[test]
    fn last_participant_keeps_room_for_self_message_and_note() {
        let source = eight_participants(
            "Participant8->>Participant8: loop back\nNote right of Participant8: remember this\n",
        );

        let (positions, width) = layout(&source, 80);
        let last = *positions.last().unwrap();
        let note = text_width("remember this");

        assert!(last + 2 + note + 4 < width);
        assert!(last + SELF_LOOP_WIDTH + text_width("loop back") < width);
    }

    #[test]
    fn widened_message_label_has_blank_columns_on_both_sides() {
        let source =
            "sequenceDiagram\nparticipant A\nparticipant B\nA->>B: a long label that needs room\n";
        let (positions, _) = layout(source, 40);

        let label = text_width("a long label that needs room");
        assert!(positions[1] - positions[0] >= label + 4);
    }

    #[test]
    fn label_that_fits_between_headers_does_not_widen_canvas() {
        let source =
            "sequenceDiagram\nparticipant A\nparticipant B\nA->>B: 123456789012345678901234x\n";

        assert_eq!(layout(source, 80).1, 80);
    }

    #[test]
    fn wide_glyph_in_last_visible_column_is_blanked() {
        use ratatui::{Terminal, backend::TestBackend};

        let area = Rect::new(0, 0, 10, 3);
        let mut canvas = Buffer::empty(area);
        canvas.set_string(8, 1, "用", Style::default());

        let mut terminal = Terminal::new(TestBackend::new(10, 3)).unwrap();
        terminal
            .draw(|f| copy_visible_columns(f, &canvas, area, 0))
            .unwrap();

        assert_eq!(terminal.backend().buffer()[(8, 1)].symbol(), " ");
    }
}
