//! A borderless mini player that changes shape with its native window.
//! A persistent split trades track information for playback controls; taller
//! forms make room for artwork, seeking, and the existing expandable panels.

use std::time::Instant;

use egui::{Align, Color32, Layout, Rect, Sense, Ui, UiBuilder, Vec2, pos2, vec2};

use crate::app::{App, NowPlaying};
use crate::model::Action;
use crate::player::RepeatMode;
use crate::settings::VisMode;
use crate::theme::{self, Icon, Palette};
use crate::{util, vis};

use super::widgets::{self, SliderEvent};

const PLAY_SIZE: f32 = 30.0;
const CONTROL_SPACING: f32 = 2.0;
const CONTROL_PADDING: f32 = 8.0;
/// The narrowest the controls side may become: the play button, its padding,
/// and one point of slack. The slack absorbs the f32 round trip through the
/// divider position, which otherwise rounds the play button out of its own
/// budget and leaves the controls side empty.
const CONTROLS_MIN: f32 = PLAY_SIZE + 2.0 * CONTROL_PADDING + 1.0;
/// The narrowest the information side may become: the drag dots and a cover.
const INFO_MIN: f32 = 84.0;
const OAIDV_WIDTH: f32 = 12.0;
const DETAILS_ROW_HEIGHT: f32 = 28.0;
/// How close to the bottom edge the pointer has to be for the revealed seek
/// bar to take the click instead of whatever it floats over.
const GRAB_BAND: f32 = 8.0;
/// Where the volume slider anchors itself, written by the button that opens it.
const VOLUME_BUTTON_RECT_ID: &str = "mini-volume-button-rect";
/// The diameter of a macOS traffic light.
#[cfg(target_os = "macos")]
const CLOSE_DOT: f32 = 12.0;

/// Drop priority: the first entry survives the narrowest controls side. The
/// details row picks the list up where the main row ran out of width.
const LADDER: [Control; 13] = [
    Control::Play,
    Control::Next,
    Control::Previous,
    Control::Add,
    Control::Repeat,
    Control::Volume,
    Control::Shuffle,
    Control::Lyrics,
    Control::Equalizer,
    Control::Playlist,
    Control::Connect,
    Control::Like,
    Control::FullWindow,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Form {
    Tall,
    Stacked,
    Micro,
    Compact,
    Wide,
}

fn form(size: Vec2) -> Form {
    if size.y >= 200.0 {
        Form::Tall
    } else if size.y >= 116.0 {
        Form::Stacked
    } else if size.x < 300.0 {
        Form::Micro
    } else if size.x >= 520.0 {
        Form::Wide
    } else {
        Form::Compact
    }
}

pub fn show(app: &mut App, ui: &mut Ui) {
    // The mini player keeps its own keyboard focus, so the shortcuts have to
    // run from here too: Cmd+W is the borderless window's only close, and
    // Cmd+Shift+M and Cmd+Shift+K are how a window this small reaches the
    // full window and MilkDrop when no button fits.
    super::keys::handle(app, ui.ctx());
    let palette = app.palette;
    let now = app.now_playing();
    let tint = app
        .settings
        .mini_player_tint_background
        .then(|| {
            app.tint_for(
                now.as_ref()
                    .and_then(|now| now.art_url.as_deref().or(now.art_small.as_deref())),
            )
        })
        .flatten();
    let fill = tint.map_or(palette.window, |tint| {
        super::blend(palette.window, tint, 0.45)
    });
    let window_rect = ui.max_rect();
    // The connect button is far down the ladder, so anchor the popup to the
    // window's own corner first; the button overwrites this when it is drawn.
    ui.ctx().data_mut(|data| {
        data.insert_temp(
            egui::Id::new(super::devices::BUTTON_RECT_ID),
            Rect::from_min_size(
                pos2(window_rect.right() - 8.0, window_rect.bottom() - 8.0),
                Vec2::ZERO,
            ),
        )
    });
    let lyrics_t =
        ui.ctx()
            .animate_bool_with_time(egui::Id::new("mini-lyrics"), app.show_lyrics_panel, 0.22);
    egui::Frame::new()
        .fill(fill)
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            let content = ui.available_rect_before_wrap();
            let shape = form(content.size());
            ui.set_min_size(content.size());
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.scope(|ui| {
                if app.show_mini_player_settings {
                    ui.disable();
                }
                egui::ScrollArea::both()
                    .id_salt("mini-sections")
                    .auto_shrink([false, false])
                    .min_scrolled_height(0.0)
                    .min_scrolled_width(0.0)
                    .show(ui, |ui| {
                        let panels = lyrics_t > 0.001
                            || app.show_equalizer_window
                            || app.show_playlist_window;
                        let height = if panels {
                            content.height().min(match shape {
                                Form::Tall => 200.0,
                                Form::Stacked => 116.0,
                                _ => 80.0,
                            })
                        } else {
                            content.height()
                        };
                        let (rect, _) = ui.allocate_exact_size(
                            vec2(ui.available_width(), height),
                            Sense::hover(),
                        );
                        let mut bar = child(ui, rect, "mini-form", Layout::top_down(Align::Min));
                        match shape {
                            Form::Tall => tall(app, &mut bar, now.as_ref(), rect, fill),
                            Form::Stacked => stacked(app, &mut bar, now.as_ref(), rect),
                            _ => single_row(app, &mut bar, now.as_ref(), rect, shape),
                        }
                        lyrics_section(app, ui, lyrics_t);
                        if app.show_equalizer_window {
                            ui.separator();
                            super::equalizer_window::show(app, ui);
                        }
                        if app.show_playlist_window {
                            ui.separator();
                            super::playlist_window::show(app, ui);
                        }
                    });
            });
            if shape != Form::Tall && !app.show_mini_player_settings {
                seek_overlay(app, ui, window_rect, now.as_ref());
            }
            if !app.show_mini_player_settings {
                super::devices::popup(app, ui.ctx());
                volume_popup(app, ui.ctx(), window_rect, now.as_ref());
            }
            if app.show_mini_player_settings {
                settings_overlay(app, ui, content);
            }
        });
    // A screenshot run draws the mini player inside the main window. That
    // window's geometry is not the mini player's and must not reach disk.
    if app.capturing() {
        return;
    }
    // Both readings are already in egui points, the same unit the viewport
    // builder takes: egui-winit multiplies the builder by the zoom factor on
    // the way out and divides the window by it on the way back, so the round
    // trip is exact and compensating for the zoom here would inflate the
    // window a little more on every launch.
    //
    // Clamp transitional full-screen readings before they can reach disk.
    if let Some(rect) = ui.ctx().input(|input| input.viewport().inner_rect) {
        let size = [
            rect.width().clamp(240.0, 1200.0),
            rect.height().clamp(56.0, 460.0),
        ];
        if (app.settings.mini_player_size[0] - size[0]).abs() > 1.0
            || (app.settings.mini_player_size[1] - size[1]).abs() > 1.0
        {
            app.settings.mini_player_size = size;
            app.actions.push(Action::SettingsChanged);
        }
    }
    if let Some(rect) = ui.ctx().input(|input| input.viewport().outer_rect) {
        let pos = [rect.min.x, rect.min.y];
        let moved = app
            .settings
            .mini_player_pos
            .is_none_or(|saved| (saved[0] - pos[0]).abs() > 1.0 || (saved[1] - pos[1]).abs() > 1.0);
        if moved {
            app.settings.mini_player_pos = Some(pos);
            app.actions.push(Action::SettingsChanged);
        }
    }
}

fn child(ui: &mut Ui, rect: Rect, id: &str, layout: Layout) -> Ui {
    let mut child = ui.new_child(UiBuilder::new().id_salt(id).max_rect(rect).layout(layout));
    child.set_clip_rect(rect.intersect(ui.clip_rect()));
    child
}

fn tall(app: &mut App, ui: &mut Ui, now: Option<&NowPlaying>, rect: Rect, fill: Color32) {
    let strip = Rect::from_min_size(rect.min, vec2(rect.width(), 26.0));
    top_strip(app, ui, strip);
    let controls_height = if app.settings.mini_player_show_controls {
        40.0
    } else {
        0.0
    };
    let title_rect = Rect::from_min_max(pos2(rect.left(), rect.bottom() - 38.0), rect.max);
    let seek_rect = Rect::from_min_max(
        pos2(rect.left(), title_rect.top() - 24.0),
        pos2(rect.right(), title_rect.top()),
    );
    let banner = Rect::from_min_max(
        pos2(rect.left(), strip.bottom() + 2.0),
        pos2(rect.right(), seek_rect.top() - controls_height - 4.0),
    );
    ui.painter()
        .rect_filled(banner, 6.0, super::blend(fill, app.palette.accent, 0.12));
    let cover = Rect::from_center_size(
        banner.center(),
        Vec2::splat(banner.height().min(banner.width())),
    );
    widgets::paint_cover(
        ui,
        &app.palette,
        now.and_then(|now| now.art_url.as_deref().or(now.art_small.as_deref())),
        cover,
        6.0,
        Icon::Music,
        Some(app.backend.art()),
    );
    if app.settings.mini_player_show_controls {
        let transport = Rect::from_min_max(
            pos2(rect.left(), seek_rect.top() - controls_height),
            pos2(rect.right(), seek_rect.top()),
        );
        controls(app, ui, now, transport, true);
        seek_row(app, ui, seek_rect, now, true);
    }
    track_info(app, ui, now, title_rect, false, false, true);
}

fn stacked(app: &mut App, ui: &mut Ui, now: Option<&NowPlaying>, rect: Rect) {
    top_strip(
        app,
        ui,
        Rect::from_min_size(rect.min, vec2(rect.width(), 26.0)),
    );
    let controls_height = if app.settings.mini_player_show_controls {
        38.0
    } else {
        0.0
    };
    let info = Rect::from_min_max(
        pos2(rect.left(), rect.top() + 28.0),
        pos2(rect.right(), rect.bottom() - controls_height - 4.0),
    );
    track_info(app, ui, now, info, true, false, true);
    if app.settings.mini_player_show_controls {
        let transport = Rect::from_min_max(pos2(rect.left(), info.bottom() + 2.0), rect.max);
        controls(app, ui, now, transport, true);
    }
}

fn top_strip(app: &mut App, ui: &mut Ui, rect: Rect) {
    // Before the drag handle, or the handle would swallow its clicks.
    #[cfg(target_os = "macos")]
    close_dot(
        app,
        ui,
        pos2(
            rect.left() + CLOSE_DOT / 2.0 + 1.0,
            rect.top() + CLOSE_DOT / 2.0 + 1.0,
        ),
    );
    let handle = Rect::from_center_size(rect.center(), vec2((rect.width() - 56.0).max(0.0), 24.0));
    super::titlebar_drag(ui, handle);
    drag_dots(
        ui,
        Rect::from_center_size(handle.center(), vec2(20.0, 10.0)),
        &app.palette,
    );
    let gear = Rect::from_min_size(pos2(rect.right() - 26.0, rect.top()), Vec2::splat(26.0));
    let mut gear_ui = child(ui, gear, "mini-gear", Layout::left_to_right(Align::Center));
    if theme::icon_button(
        &mut gear_ui,
        Icon::Settings,
        14.0,
        app.palette.secondary,
        app.palette.text,
        "Mini player settings",
    )
    .clicked()
    {
        app.show_mini_player_settings = !app.show_mini_player_settings;
    }
}

/// macOS puts a window's close control in its top-left corner. This one is
/// borderless, so it draws its own traffic light rather than going without.
#[cfg(target_os = "macos")]
fn close_dot(app: &mut App, ui: &mut Ui, center: egui::Pos2) {
    let hit = Rect::from_center_size(center, Vec2::splat(CLOSE_DOT + 4.0));
    let response = ui
        .interact(hit, ui.id().with("mini-close"), Sense::click())
        .on_hover_text("Close the mini player");
    let hovered = response.hovered();
    let fill = if hovered {
        Color32::from_rgb(255, 95, 87)
    } else {
        Color32::from_rgb(226, 86, 79)
    };
    ui.painter().circle_filled(center, CLOSE_DOT / 2.0, fill);
    if hovered {
        // The glyph the platform reveals when the pointer is over the lights.
        let arm = 2.6;
        let stroke = egui::Stroke::new(1.2, Color32::from_rgb(115, 22, 16));
        ui.painter()
            .line_segment([center + vec2(-arm, -arm), center + vec2(arm, arm)], stroke);
        ui.painter()
            .line_segment([center + vec2(arm, -arm), center + vec2(-arm, arm)], stroke);
    }
    if response.clicked() {
        app.actions.push(Action::CloseMiniPlayer);
    }
}

fn drag_dots(ui: &Ui, rect: Rect, palette: &Palette) {
    let (cols, rows) = if rect.width() > rect.height() {
        (4, 2)
    } else {
        (2, 3)
    };
    for row in 0..rows {
        for col in 0..cols {
            let offset = vec2(
                col as f32 - (cols - 1) as f32 / 2.0,
                row as f32 - (rows - 1) as f32 / 2.0,
            ) * 5.0;
            ui.painter()
                .circle_filled(rect.center() + offset, 1.0, palette.secondary);
        }
    }
}

/// How far the divider may travel, in points of information width. Clamping in
/// points rather than in the persisted fraction keeps the controls side exactly
/// as wide as it measures: a fraction re-multiplied by the width lands a
/// rounding step short and empties the control ladder.
fn split_bounds(width: f32) -> (f32, f32) {
    let max = (width - CONTROLS_MIN).max(0.0);
    (INFO_MIN.min(max), max)
}

fn split_info_width(width: f32, fraction: f32) -> f32 {
    let width = width.max(1.0);
    let (min, max) = split_bounds(width);
    let fraction = if fraction.is_finite() { fraction } else { 0.5 };
    (width * fraction).clamp(min, max)
}

fn split_handle(app: &mut App, ui: &mut Ui, rect: Rect) -> f32 {
    let mut info = split_info_width(rect.width(), app.settings.mini_player_split);
    let hit = Rect::from_center_size(
        pos2(rect.left() + info, rect.center().y),
        vec2(7.0, rect.height()),
    );
    let response = ui
        .interact(hit, ui.id().with("mini-split"), Sense::drag())
        .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
    if response.dragged()
        && let Some(pointer) = response.interact_pointer_pos()
    {
        let width = rect.width().max(1.0);
        info = split_info_width(width, (pointer.x - rect.left()) / width);
        let fraction = info / width;
        let (min, max) = split_bounds(width);
        // A window too narrow to hold both sides pins the divider; saving that
        // would overwrite the user's choice with a value they cannot see.
        if max > min
            && (!app.settings.mini_player_split.is_finite()
                || (fraction - app.settings.mini_player_split).abs() > 0.001)
        {
            app.settings.mini_player_split = fraction;
            app.actions.push(Action::SettingsChanged);
        }
    }
    let x = rect.left() + info;
    let color = if response.hovered() || response.dragged() {
        app.palette.text
    } else {
        app.palette.outline
    };
    ui.painter().line_segment(
        [pos2(x, rect.top() + 4.0), pos2(x, rect.bottom() - 4.0)],
        egui::Stroke::new(1.0, color),
    );
    info
}

fn pill_size(ui: &Ui, label: &str) -> Vec2 {
    ui.painter()
        .layout_no_wrap(label.to_owned(), theme::semibold(13.0), Color32::WHITE)
        .size()
        + vec2(36.0, 16.0)
}

fn single_row(app: &mut App, ui: &mut Ui, now: Option<&NowPlaying>, rect: Rect, shape: Form) {
    let show_controls = app.settings.mini_player_show_controls;
    let details_height = pill_size(ui, "EQ").y.max(DETAILS_ROW_HEIGHT);
    // The main row needs the play button and its padding; anything above that
    // is slack, so the default window keeps its details row after a resize or a
    // zoom change rather than losing it to an exact-equality threshold.
    let details = show_controls && shape != Form::Micro && rect.height() >= 40.0 + details_height;
    let bar = Rect::from_min_size(
        rect.min,
        vec2(
            rect.width(),
            rect.height() - if details { details_height } else { 0.0 },
        ),
    );
    let info_width = if show_controls {
        split_handle(app, ui, bar)
    } else {
        bar.width()
    };
    // macOS keeps the close control in the corner, so the grip starts below it.
    #[cfg(target_os = "macos")]
    let dots_top = bar.top() + CLOSE_DOT + 2.0;
    #[cfg(not(target_os = "macos"))]
    let dots_top = bar.top();
    // A high zoom factor can leave less height than the dot takes.
    let dots = Rect::from_min_max(
        pos2(bar.left(), dots_top.min(bar.bottom())),
        pos2(bar.left() + 16.0, bar.bottom()),
    );
    #[cfg(target_os = "macos")]
    close_dot(
        app,
        ui,
        pos2(bar.left() + CLOSE_DOT / 2.0, bar.top() + CLOSE_DOT / 2.0),
    );
    super::titlebar_drag(ui, dots);
    drag_dots(ui, dots, &app.palette);
    // Right-clicking the drag handle keeps settings reachable without taking control space.
    if ui.is_enabled()
        && ui.input(|input| {
            input.pointer.secondary_clicked()
                && input
                    .pointer
                    .interact_pos()
                    .is_some_and(|pos| dots.contains(pos))
        })
    {
        app.show_mini_player_settings = true;
    }
    ui.interact(dots, ui.id().with("mini-dots-hint"), Sense::hover())
        .on_hover_text("Drag to move. Right-click for mini player settings.");
    let mut info = Rect::from_min_max(
        pos2(dots.right() + 4.0, bar.top()),
        pos2(bar.left() + info_width - 6.0, bar.bottom()),
    );
    let slack = info.width() - 120.0;
    // Turning the visualiser off silences the meter here too, and with it the
    // 60Hz repaint it asks for.
    if show_controls && shape != Form::Micro && slack >= 40.0 && app.settings.vis != VisMode::Off {
        let column = if shape == Form::Wide {
            OAIDV_WIDTH + 4.0
        } else {
            0.0
        };
        let meter_width = (slack - column).clamp(30.0, 76.0);
        let meter = Rect::from_center_size(
            pos2(info.right() - meter_width / 2.0, info.center().y),
            vec2(meter_width, info.height().min(30.0)),
        );
        level_meter(app, ui, now, meter);
        info.max.x = meter.left() - 6.0;
        if shape == Form::Wide {
            let lamps = Rect::from_min_max(pos2(info.right() - OAIDV_WIDTH, info.top()), info.max);
            let mut lamps_ui = child(
                ui,
                lamps,
                "mini-oaidv",
                Layout::left_to_right(Align::Center),
            );
            oaidv_column(&mut lamps_ui, &app.palette);
            info.max.x = lamps.left() - 4.0;
        }
    }
    track_info(app, ui, now, info, true, shape == Form::Micro, false);
    let drawn = if show_controls {
        let controls_rect = Rect::from_min_max(pos2(bar.left() + info_width, bar.top()), bar.max);
        controls(app, ui, now, controls_rect, false)
    } else {
        Vec::new()
    };
    if details {
        let details_rect = Rect::from_min_max(pos2(rect.left(), bar.bottom()), rect.max);
        let mut details_ui = child(
            ui,
            details_rect,
            "mini-details",
            Layout::left_to_right(Align::Center),
        );
        details_row(app, &mut details_ui, now, &drawn);
    }
}

fn track_info(
    app: &mut App,
    ui: &mut Ui,
    now: Option<&NowPlaying>,
    mut rect: Rect,
    cover: bool,
    title_only: bool,
    add: bool,
) {
    if add && app.settings.mini_player_show_controls {
        let button = Rect::from_min_size(
            pos2(rect.right() - 26.0, rect.center().y - 13.0),
            Vec2::splat(26.0),
        );
        let mut add_ui = child(ui, button, "mini-add", Layout::left_to_right(Align::Center));
        add_to_queue(app, &mut add_ui, now);
        rect.max.x = button.left() - 6.0;
    }
    if cover {
        let edge = rect
            .height()
            .min(if title_only { 30.0 } else { 52.0 })
            .min((rect.width() - 12.0).max(0.0));
        let cover_rect = Rect::from_min_size(
            pos2(rect.left(), rect.center().y - edge / 2.0),
            Vec2::splat(edge),
        );
        widgets::paint_cover(
            ui,
            &app.palette,
            now.and_then(|now| now.art_small.as_deref().or(now.art_url.as_deref())),
            cover_rect,
            6.0,
            Icon::Music,
            Some(app.backend.art()),
        );
        rect.min.x = (cover_rect.right() + 8.0).min(rect.right());
    }
    let height = if title_only { 18.0 } else { 34.0 };
    let labels = Rect::from_center_size(rect.center(), vec2(rect.width().max(0.0), height));
    let mut labels_ui = child(ui, labels, "mini-track-text", Layout::top_down(Align::Min));
    labels_ui.spacing_mut().item_spacing.y = 0.0;
    labels_ui.add(
        egui::Label::new(
            egui::RichText::new(now.map_or("Nothing playing", |now| now.title.as_str()))
                .font(theme::medium(14.0))
                .color(app.palette.text),
        )
        .truncate(),
    );
    if !title_only {
        labels_ui.add(
            egui::Label::new(
                egui::RichText::new(now.map_or("", |now| now.subtitle.as_str()))
                    .font(theme::regular(12.0))
                    .color(app.palette.secondary),
            )
            .truncate(),
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Control {
    Like,
    Connect,
    Shuffle,
    Lyrics,
    Equalizer,
    Playlist,
    Volume,
    Previous,
    Play,
    Next,
    Add,
    Repeat,
    FullWindow,
}

#[derive(Clone, Copy)]
struct Candidate {
    control: Control,
    width: f32,
}

fn control_width(ui: &Ui, control: Control) -> f32 {
    match control {
        Control::Play => PLAY_SIZE,
        Control::Equalizer => pill_size(ui, "EQ").x,
        Control::Playlist => pill_size(ui, "PL").x,
        Control::Volume => 13.0 + 12.0,
        _ => 14.0 + 12.0,
    }
}

fn control_ladder(ui: &Ui, width: f32, skip: Option<Control>) -> Vec<Candidate> {
    let mut candidates: Vec<_> = LADDER
        .into_iter()
        .filter(|control| Some(*control) != skip)
        .map(|control| Candidate {
            control,
            width: control_width(ui, control),
        })
        .collect();
    let mut used = 0.0;
    let count = candidates
        .iter()
        .take_while(|candidate| {
            let next = used + if used > 0.0 { CONTROL_SPACING } else { 0.0 } + candidate.width;
            if next > width {
                return false;
            }
            used = next;
            true
        })
        .count();
    candidates.truncate(count);
    candidates
}

/// Draws as many controls as `rect` measures room for, and reports which ones
/// those were so the details row can take over from there.
fn controls(
    app: &mut App,
    ui: &mut Ui,
    now: Option<&NowPlaying>,
    rect: Rect,
    centered: bool,
) -> Vec<Control> {
    // The tall and stacked forms carry their own add-to-queue button beside the
    // title, so leaving it out here spends the width on one more control.
    let mut kept = control_ladder(
        ui,
        (rect.width() - 2.0 * CONTROL_PADDING).max(0.0),
        centered.then_some(Control::Add),
    );
    let drawn: Vec<_> = kept.iter().map(|candidate| candidate.control).collect();
    let width = kept.iter().map(|candidate| candidate.width).sum::<f32>()
        + CONTROL_SPACING * kept.len().saturating_sub(1) as f32;
    kept.sort_by_key(|candidate| candidate.control);
    let left = if centered {
        rect.center().x - width / 2.0
    } else {
        rect.right() - CONTROL_PADDING - width
    };
    let mut row = child(
        ui,
        Rect::from_min_size(pos2(left, rect.top()), vec2(width, rect.height())),
        "mini-controls",
        Layout::left_to_right(Align::Center),
    );
    row.spacing_mut().item_spacing.x = CONTROL_SPACING;
    for candidate in kept {
        row.push_id(candidate.control, |ui| {
            control(app, ui, now, candidate.control)
        });
    }
    drawn
}

fn control(app: &mut App, ui: &mut Ui, now: Option<&NowPlaying>, control: Control) {
    let palette = app.palette;
    let enabled = now.is_some_and(|now| now.can_control) || app.is_connected();
    if control == Control::Add {
        add_to_queue(app, ui, now);
        return;
    }
    if control == Control::Play {
        let playing = now.is_some_and(|now| now.playing);
        let response = ui
            .add_enabled_ui(enabled, |ui| {
                theme::circle_button(
                    ui,
                    if playing {
                        Icon::PauseFilled
                    } else {
                        Icon::PlayFilled
                    },
                    PLAY_SIZE,
                    palette.text,
                    palette.text,
                    palette.window,
                    if playing { "Pause" } else { "Play" },
                )
            })
            .inner;
        if response
            .on_disabled_hover_text("Playback control unavailable")
            .clicked()
        {
            app.actions.push(Action::TogglePlay);
        }
        return;
    }
    if matches!(control, Control::Equalizer | Control::Playlist) {
        let (label, on, action) = if control == Control::Equalizer {
            (
                "EQ",
                app.show_equalizer_window,
                Action::ToggleEqualizerWindow,
            )
        } else {
            ("PL", app.show_playlist_window, Action::TogglePlaylistWindow)
        };
        if theme::pill_button(ui, &palette, label, on).clicked() {
            app.actions.push(action);
        }
        return;
    }
    let (icon, active, label, action, enabled) = match control {
        Control::Next => (Icon::SkipForward, false, "Next", Action::Next, enabled),
        Control::Previous => (Icon::SkipBack, false, "Previous", Action::Previous, enabled),
        Control::Repeat => {
            let repeat = now.map(|now| now.repeat).unwrap_or_default();
            (
                if repeat == RepeatMode::Track {
                    Icon::Repeat1
                } else {
                    Icon::Repeat
                },
                repeat != RepeatMode::Off,
                "Repeat",
                Action::CycleRepeat,
                enabled,
            )
        }
        Control::Volume => {
            let (icon, _) = volume_icon(app, now);
            (
                icon,
                app.show_volume_popup,
                "Volume",
                Action::ToggleVolumePopup,
                true,
            )
        }
        Control::Shuffle => (
            Icon::Shuffle,
            now.is_some_and(|now| now.shuffle),
            "Shuffle",
            Action::ToggleShuffle,
            enabled,
        ),
        Control::Lyrics => (
            Icon::Mic,
            app.show_lyrics_panel,
            "Lyrics",
            Action::ToggleLyricsPanel,
            true,
        ),
        Control::Connect => (
            Icon::Speaker,
            now.is_some_and(|now| !now.local),
            "Connect to a device",
            Action::ToggleDevicesPopup,
            true,
        ),
        Control::Like => {
            let saved = now.is_some_and(|now| app.is_saved(&now.uri).unwrap_or(false));
            (
                Icon::CircleCheck,
                saved,
                if saved {
                    "Remove from Liked Songs"
                } else {
                    "Save to Liked Songs"
                },
                Action::ToggleSaved(now.map_or_else(String::new, |now| now.uri.clone())),
                now.is_some_and(|now| !now.is_episode),
            )
        }
        Control::FullWindow => (
            Icon::ExternalLink,
            false,
            "Open the full window",
            Action::ToggleMiniPlayer,
            true,
        ),
        _ => unreachable!(),
    };
    let response = ui
        .add_enabled_ui(enabled, |ui| {
            theme::icon_button(
                ui,
                icon,
                if control == Control::Volume {
                    13.0
                } else {
                    14.0
                },
                if active {
                    palette.accent
                } else {
                    palette.secondary
                },
                palette.text,
                label,
            )
        })
        .inner;
    if control == Control::Connect {
        ui.ctx().data_mut(|data| {
            data.insert_temp(egui::Id::new(super::devices::BUTTON_RECT_ID), response.rect)
        });
        if response.clicked() {
            // Both windows can be on screen and they share one popup flag;
            // the list belongs to whichever window was asked for it.
            app.devices_popup_host = ui.ctx().viewport_id();
        }
    }
    let response = if control == Control::Volume {
        ui.ctx()
            .data_mut(|data| data.insert_temp(egui::Id::new(VOLUME_BUTTON_RECT_ID), response.rect));
        // The slider this button now opens took the place of the mute it used
        // to be, so mute keeps a one-click way in of its own.
        if response.secondary_clicked() {
            app.actions.push(Action::ToggleMute);
        }
        response.on_hover_text(if volume_icon(app, now).1 {
            "Right-click to unmute"
        } else {
            "Right-click to mute"
        })
    } else {
        response
    };
    if response
        .on_disabled_hover_text(if control == Control::Like {
            "Only a playing song can be saved to Liked Songs"
        } else {
            "Playback control unavailable"
        })
        .clicked()
    {
        app.actions.push(action);
    }
}

fn add_to_queue(app: &mut App, ui: &mut Ui, now: Option<&NowPlaying>) {
    let palette = app.palette;
    let response = ui
        .add_enabled_ui(now.is_some(), |ui| {
            theme::icon_button(
                ui,
                Icon::ListPlus,
                14.0,
                palette.secondary,
                palette.text,
                "Add the current track to the queue",
            )
        })
        .inner;
    if response
        .on_disabled_hover_text("Nothing is playing to add")
        .clicked()
        && let Some(now) = now
    {
        app.actions.push(Action::AddToQueue {
            uri: now.uri.clone(),
            label: now.title.clone(),
        });
    }
}

/// The volume slider the controls row has no width to hold inline. It floats
/// over the rows below the button, the way the devices list already does.
fn volume_popup(app: &mut App, ctx: &egui::Context, screen: Rect, now: Option<&NowPlaying>) {
    if !app.show_volume_popup {
        return;
    }
    let palette = app.palette;
    let button = ctx
        .data(|data| data.get_temp::<Rect>(egui::Id::new(VOLUME_BUTTON_RECT_ID)))
        .unwrap_or_else(|| Rect::from_min_size(pos2(8.0, 8.0), Vec2::ZERO));
    let width = 168.0;
    let volume = now
        .map(|now| now.volume_percent)
        .unwrap_or_else(|| crate::app::volume_to_percent(app.local.volume));
    let shown = app
        .volume_preview
        .map_or(volume, |fraction| (fraction * 100.0).round() as u8);
    // egui's own constraint re-centres the area rather than nudging it, which
    // unpins it from the button, so keep it inside the window by hand: slide
    // along x, and flip above the button when there is no room below.
    let height = 40.0;
    let below = button.bottom() + 4.0;
    let position = pos2(
        (button.center().x - width / 2.0).clamp(
            screen.left() + 4.0,
            (screen.right() - width - 4.0).max(screen.left() + 4.0),
        ),
        if below + height <= screen.bottom() {
            below
        } else {
            (button.top() - height - 4.0).max(screen.top() + 4.0)
        },
    );
    let area = egui::Area::new(egui::Id::new("mini-volume-popup"))
        .order(egui::Order::Foreground)
        .constrain(false)
        .fixed_pos(position)
        .show(ctx, |ui| {
            widgets::menu_frame(&palette).show(ui, |ui| {
                ui.set_width(width);
                ui.horizontal(|ui| {
                    let (icon, muted) = volume_icon(app, now);
                    if theme::icon_button(
                        ui,
                        icon,
                        14.0,
                        palette.secondary,
                        palette.text,
                        if muted { "Unmute" } else { "Mute" },
                    )
                    .clicked()
                    {
                        app.actions.push(Action::ToggleMute);
                    }
                    // Local volume is cheap to apply continuously; a remote
                    // device only hears about it on release. The same split
                    // `player_bar.rs` makes for its own slider.
                    match widgets::thin_slider(
                        ui,
                        &palette,
                        egui::Id::new("mini-volume-slider"),
                        "Volume (%)",
                        f32::from(shown) / 100.0,
                        ui.available_width(),
                        Some(0.05),
                    ) {
                        SliderEvent::Dragging(value) => {
                            app.volume_preview = Some(value);
                            if now.is_none_or(|now| now.local) {
                                app.actions
                                    .push(Action::PreviewVolume((value * 100.0).round() as u8));
                            }
                        }
                        SliderEvent::Committed(value) => {
                            app.volume_preview = None;
                            app.actions
                                .push(Action::SetVolume((value * 100.0).round() as u8));
                        }
                        SliderEvent::None => {}
                    }
                });
            });
        })
        .response;
    // Dismiss on a click outside, but not on the button itself: that already
    // toggles, and closing here too would reopen it on the same click.
    let dismissed = ctx.input(|input| {
        input.key_pressed(egui::Key::Escape)
            || (input.pointer.any_click()
                && input.pointer.interact_pos().is_some_and(|pos| {
                    !area.rect.contains(pos) && !button.contains(pos)
                }))
    });
    if dismissed {
        app.show_volume_popup = false;
    }
}

fn volume_icon(app: &App, now: Option<&NowPlaying>) -> (Icon, bool) {
    let volume = now
        .map(|now| now.volume_percent)
        .unwrap_or_else(|| crate::app::volume_to_percent(app.local.volume));
    let shown = app
        .volume_preview
        .map_or(volume, |fraction| (fraction * 100.0).round() as u8);
    (
        match shown {
            0 => Icon::VolumeX,
            1..=33 => Icon::Volume,
            34..=66 => Icon::Volume1,
            _ => Icon::Volume2,
        },
        shown == 0,
    )
}

#[derive(Clone)]
struct MeterState {
    heights: [f32; vis::BARS],
    peaks: [f32; vis::BARS],
    last: Instant,
}

fn smooth(value: f32, target: f32, dt: f32, fall_rate: f32) -> f32 {
    let rate = if target > value { 30.0 } else { fall_rate };
    value + (target - value) * (1.0 - (-dt * rate).exp())
}

// Local playback only: a remote Spotify Connect device supplies no audio
// stream to Fastpotify, so its meter stays flat rather than faking movement.
fn level_meter(app: &mut App, ui: &mut Ui, now: Option<&NowPlaying>, rect: Rect) {
    if !ui.is_rect_visible(rect) {
        return;
    }
    let local = now.is_some_and(|now| now.local);
    let sounding = now.is_some_and(|now| (now.playing || now.loading) && now.local);
    let samples = if sounding {
        app.winamp.tap.window(vis::FFT_SAMPLES, vis::LAG)
    } else {
        vec![0.0; vis::FFT_SAMPLES]
    };
    let time = Instant::now();
    let bars = app.winamp.analyser.step(&samples, time);
    let moving = sounding || !app.winamp.analyser.settled();
    let id = egui::Id::new("mini-meter-smoothing");
    let mut state = ui
        .ctx()
        .data_mut(|data| data.get_temp::<MeterState>(id))
        .unwrap_or(MeterState {
            heights: [0.0; vis::BARS],
            peaks: [0.0; vis::BARS],
            last: time,
        });
    let dt = time.saturating_duration_since(state.last).as_secs_f32();
    state.last = time;
    for (index, bar) in bars.iter().enumerate() {
        state.heights[index] = if local {
            smooth(state.heights[index], f32::from(bar.height), dt, 12.0)
        } else {
            0.0
        };
        state.peaks[index] = if local {
            smooth(
                state.peaks[index],
                f32::from(bar.peak.unwrap_or(0)),
                dt,
                6.0,
            )
        } else {
            0.0
        };
    }
    let count = (((rect.width() + 2.0) / 4.0) as usize).clamp(8, vis::BARS);
    let gap = 2.0;
    let width = (rect.width() - gap * (count - 1) as f32) / count as f32;
    let palette = app.palette;
    for index in 0..count {
        let source = index * vis::BARS / count;
        let level = state.heights[source] / f32::from(vis::ROWS);
        let height = (rect.height() * level).max(2.0);
        let x = rect.left() + index as f32 * (width + gap);
        let bar = Rect::from_min_size(pos2(x, rect.bottom() - height), vec2(width, height));
        ui.painter().rect_filled(
            bar,
            1.0,
            if level > 0.01 {
                palette.accent
            } else {
                palette.accent.gamma_multiply(0.24)
            },
        );
        if state.peaks[source] > 0.1 {
            // A window shorter than the cap itself would invert the bounds.
            let y = (rect.bottom() - rect.height() * state.peaks[source] / f32::from(vis::ROWS))
                .clamp(rect.top(), (rect.bottom() - 2.0).max(rect.top()));
            ui.painter().rect_filled(
                Rect::from_min_size(pos2(x, y), vec2(width, 2.0)),
                0.0,
                super::blend(palette.accent, Color32::WHITE, 0.45),
            );
        }
    }
    let smoothing = state
        .heights
        .iter()
        .chain(&state.peaks)
        .any(|value| *value > 0.1);
    ui.ctx().data_mut(|data| data.insert_temp(id, state));
    // The mini player is an immediate child viewport, so eframe can only
    // repaint it by repainting the main window with it. A meter nobody can
    // see is not worth redrawing the whole app thirty times a second for.
    let hidden = ui
        .ctx()
        .input(|input| input.viewport().occluded.unwrap_or(false));
    // egui subtracts a predicted frame; two frames avoid a busy loop with vsync off.
    if (moving || smoothing) && !hidden {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_micros(16_667) * 2);
    }
}

/// Decorative only: the app has no state corresponding to O/A/I/D/V lamps, and
/// the tooltip says so rather than letting them read as live indicators.
fn oaidv_column(ui: &mut Ui, palette: &Palette) {
    const LETTERS: [&str; 5] = ["O", "A", "I", "D", "V"];
    let (rect, response) = ui.allocate_exact_size(
        vec2(OAIDV_WIDTH, ui.available_height().min(52.0)),
        Sense::hover(),
    );
    response.on_hover_text("Decorative, from Winamp's layout");
    if !ui.is_rect_visible(rect) {
        return;
    }
    let step = rect.height() / LETTERS.len() as f32;
    for (index, letter) in LETTERS.iter().enumerate() {
        ui.painter().text(
            pos2(rect.center().x, rect.top() + step * (index as f32 + 0.5)),
            egui::Align2::CENTER_CENTER,
            letter,
            theme::regular(9.0),
            palette.dim,
        );
    }
}

fn text_width(ui: &Ui, text: &str, font: egui::FontId) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_owned(), font, Color32::WHITE)
        .size()
        .x
}

/// The bitrate is the one chosen for this computer and the sample rate is the
/// decoder's, so neither describes a device across the room and neither is
/// shown for one. Nothing playing means nothing to measure.
fn rates(app: &App, now: Option<&NowPlaying>) -> Option<[String; 2]> {
    now.filter(|now| now.local && (now.playing || now.loading || now.position_ms > 0))?;
    Some([
        format!("{} KBPS", app.settings.bitrate),
        format!("{} KHZ", librespot_playback::SAMPLE_RATE / 1000),
    ])
}

fn details_row(app: &mut App, ui: &mut Ui, now: Option<&NowPlaying>, drawn: &[Control]) {
    let palette = app.palette;
    ui.spacing_mut().item_spacing.x = 6.0;
    let small = theme::regular(10.0);
    let mono = app.settings.mono;
    let rates = rates(app, now);
    let trailing = ["MONO", "STEREO"]
        .iter()
        .map(|label| text_width(ui, label, small.clone()) + 6.0)
        .sum::<f32>()
        + rates
            .iter()
            .flatten()
            .map(|text| text_width(ui, text, small.clone()) + 6.0)
            .sum::<f32>();
    // Whatever the main row's ladder ran out of width for continues here, so no
    // button is drawn twice and none is left unreachable at the default size.
    let mut budget = ui.available_width() - trailing;
    for control in LADDER
        .into_iter()
        .filter(|control| !drawn.contains(control))
    {
        let width = control_width(ui, control) + 6.0;
        if width > budget {
            break;
        }
        budget -= width;
        ui.push_id(control, |ui| self::control(app, ui, now, control));
    }
    for (label, active) in [("MONO", mono), ("STEREO", !mono)] {
        if theme::link(
            ui,
            label,
            small.clone(),
            if active { palette.accent } else { palette.dim },
        )
        .on_hover_text(if label == "MONO" {
            "Play in mono"
        } else {
            "Play in stereo"
        })
        .clicked()
            && !active
        {
            app.actions.push(Action::ToggleMono);
        }
    }
    if let Some([kbps, khz]) = rates {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            theme::text(ui, kbps, small.clone(), palette.dim);
            theme::text(ui, khz, small.clone(), palette.dim);
        });
    }
}

fn seek_row(app: &mut App, ui: &mut Ui, mut rect: Rect, now: Option<&NowPlaying>, times: bool) {
    let palette = app.palette;
    let (position, duration) = now
        .map(|now| (now.position_ms, now.duration_ms))
        .unwrap_or((0, 0));
    let fraction = if duration > 0 {
        position as f32 / duration as f32
    } else {
        0.0
    };
    let shown = app.seek_preview.unwrap_or(fraction);
    if times {
        ui.painter().text(
            pos2(rect.left(), rect.center().y),
            egui::Align2::LEFT_CENTER,
            util::format_duration_ms((shown * duration as f32) as u32),
            theme::regular(11.0),
            palette.secondary,
        );
        ui.painter().text(
            pos2(rect.right(), rect.center().y),
            egui::Align2::RIGHT_CENTER,
            util::format_duration_ms(duration),
            theme::regular(11.0),
            palette.secondary,
        );
        rect = rect.shrink2(vec2(44.0, 0.0));
    }
    let slider_rect = Rect::from_center_size(rect.center(), vec2(rect.width(), 16.0));
    let mut slider_ui = child(
        ui,
        slider_rect,
        "mini-seek-row",
        Layout::left_to_right(Align::Center),
    );
    if duration == 0 || !now.is_some_and(|now| now.can_control) {
        slider_ui.disable();
    }
    let event = widgets::thin_slider(
        &mut slider_ui,
        &palette,
        egui::Id::new("mini-seek"),
        "Playback position (%)",
        fraction,
        rect.width(),
        None,
    );
    let shown = match &event {
        SliderEvent::Dragging(value) | SliderEvent::Committed(value) => *value,
        SliderEvent::None => shown,
    };
    match event {
        SliderEvent::Dragging(value) => app.seek_preview = Some(value),
        SliderEvent::Committed(value) => {
            app.seek_preview = None;
            if duration > 0 {
                app.actions
                    .push(Action::Seek((value * duration as f32) as u32));
            }
        }
        SliderEvent::None => {}
    }
    slider_ui.painter().circle_filled(
        pos2(
            rect.left() + shown.clamp(0.0, 1.0) * rect.width(),
            rect.center().y,
        ),
        3.0,
        palette.text,
    );
}

fn seek_overlay(app: &mut App, ui: &mut Ui, rect: Rect, now: Option<&NowPlaying>) {
    if !app.settings.mini_player_show_controls {
        return;
    }
    let hover = ui.input(|input| {
        input
            .pointer
            .hover_pos()
            .is_some_and(|pos| rect.contains(pos))
    });
    let t = ui.ctx().animate_bool_with_time(
        egui::Id::new("mini-seek-hover"),
        hover || app.seek_preview.is_some(),
        0.16,
    );
    if t <= 0.001 {
        return;
    }
    // Seated on the bottom edge, not across it: the slider is a fixed 16 points
    // tall, and anything below the edge is clipped away rather than grabbable.
    let bottom = Rect::from_center_size(
        pos2(rect.center().x, rect.bottom() - 8.0),
        vec2(rect.width(), 16.0),
    );
    let mut overlay = ui.new_child(
        UiBuilder::new()
            .id_salt("mini-bottom-seek")
            .max_rect(bottom),
    );
    overlay.set_clip_rect(rect);
    overlay.set_opacity(t);
    // Only the bottom edge grabs the pointer. The slider's own rect is a fixed
    // 16 points tall and is drawn last, so letting it interact everywhere would
    // shadow the details row's pills sitting in its upper half.
    let on_edge = ui.input(|input| {
        input
            .pointer
            .hover_pos()
            .is_some_and(|pos| pos.y >= rect.bottom() - GRAB_BAND)
    });
    if on_edge || app.seek_preview.is_some() {
        seek_row(app, &mut overlay, bottom, now, false);
    } else {
        seek_line(&overlay, &app.palette, bottom, now);
    }
}

/// The revealed line before the pointer reaches the edge: painted to match
/// `thin_slider` at rest, but with no interaction of its own.
fn seek_line(ui: &Ui, palette: &Palette, rect: Rect, now: Option<&NowPlaying>) {
    if !ui.is_rect_visible(rect) {
        return;
    }
    let (position, duration) = now
        .map(|now| (now.position_ms, now.duration_ms))
        .unwrap_or((0, 0));
    let fraction = if duration > 0 {
        (position as f32 / duration as f32).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let bar = Rect::from_center_size(rect.center(), vec2(rect.width(), 4.0));
    let track = if palette.dark {
        Color32::from_white_alpha(50)
    } else {
        Color32::from_black_alpha(40)
    };
    let painter = ui.painter();
    painter.rect_filled(bar, 2.0, track);
    let filled = Rect::from_min_max(
        bar.min,
        pos2(bar.left() + bar.width() * fraction, bar.max.y),
    );
    painter.rect_filled(filled, 2.0, palette.text);
    painter.circle_filled(pos2(filled.right(), bar.center().y), 3.0, palette.text);
}

fn settings_overlay(app: &mut App, ui: &mut Ui, rect: Rect) {
    let palette = app.palette;
    let mut panel = ui.new_child(
        UiBuilder::new()
            .id_salt("mini-settings-panel")
            .max_rect(rect)
            .layer_id(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("mini-settings-layer"),
            )),
    );
    panel.set_clip_rect(rect);
    panel.painter().rect_filled(rect, 8.0, palette.window);
    panel.interact(rect, panel.id().with("background"), Sense::click());
    panel.spacing_mut().item_spacing.y = 0.0;
    egui::ScrollArea::vertical()
        .id_salt("mini-settings-scroll")
        .auto_shrink([false, false])
        .min_scrolled_height(0.0)
        .show(&mut panel, |ui| {
            ui.horizontal(|ui| {
                let done_width = pill_size(ui, "Done").x;
                let title_width =
                    (ui.available_width() - done_width - ui.spacing().item_spacing.x).max(0.0);
                ui.add_sized(
                    vec2(title_width, 32.0),
                    egui::Label::new(
                        egui::RichText::new("Mini player settings")
                            .font(theme::semibold(14.0))
                            .color(palette.text),
                    )
                    .truncate(),
                );
                if theme::pill_button(ui, &palette, "Done", true).clicked() {
                    app.show_mini_player_settings = false;
                }
            });
            for (label, background) in [("Background colour", true), ("Playback row", false)] {
                let mut on = if background {
                    app.settings.mini_player_tint_background
                } else {
                    app.settings.mini_player_show_controls
                };
                let changed = ui
                    .horizontal(|ui| {
                        let response = widgets::switch(ui, &palette, label, &mut on);
                        theme::text(ui, label, theme::regular(12.0), palette.text);
                        response.changed()
                    })
                    .inner;
                if changed {
                    if background {
                        app.settings.mini_player_tint_background = on;
                    } else {
                        app.settings.mini_player_show_controls = on;
                    }
                    app.actions.push(Action::SettingsChanged);
                }
            }
            // The way back is last in the control ladder, so the narrowest
            // windows drop it. This panel is reachable at every size.
            ui.add_space(2.0);
            if theme::pill_button(ui, &palette, "Open the full window", false).clicked() {
                app.show_mini_player_settings = false;
                app.actions.push(Action::ToggleMiniPlayer);
            }
        });
}

fn lyrics_section(app: &mut App, ui: &mut Ui, t: f32) {
    if t <= 0.001 {
        return;
    }
    let target_height = 180.0;
    let (visible, _) = ui.allocate_exact_size(
        vec2(ui.available_width(), t * target_height),
        Sense::hover(),
    );
    let full = Rect::from_min_size(visible.min, vec2(visible.width(), target_height));
    let mut lyrics = child(
        ui,
        full,
        "mini-lyrics-section",
        Layout::top_down(Align::Min),
    );
    lyrics.set_clip_rect(visible.intersect(ui.clip_rect()));
    lyrics.separator();
    super::lyrics::header(app, &mut lyrics);
    lyrics.add_space(4.0);
    super::lyrics::contents(app, &mut lyrics);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppOptions;
    use crate::paths::AppDirs;
    use crate::settings::Settings;
    use egui::accesskit::{Action as AccessibleAction, ActionData, NodeId, Role, TreeUpdate};

    fn test_app(name: &str) -> (egui::Context, App) {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let root =
            std::env::temp_dir().join(format!("fastpotify-mini-{name}-{}", std::process::id()));
        let mut app = App::new(
            &crate::backend::Waker::default(),
            AppDirs {
                config: root.join("config"),
                state: root.join("state"),
                cache: root.join("cache"),
            },
            Settings::default(),
            AppOptions {
                media_controls: false,
                restore_sign_in: false,
                tray: false,
            },
        );
        app.attach(&ctx);
        crate::demo::populate(&mut app);
        app.settings.mini_player_tint_background = false;
        (ctx, app)
    }

    fn frame(
        ctx: &egui::Context,
        app: &mut App,
        size: Vec2,
        events: Vec<egui::Event>,
    ) -> TreeUpdate {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, size)),
                events,
                ..Default::default()
            },
            |ui| show(app, ui),
        );
        output.textures_delta.clear();
        output
            .platform_output
            .accesskit_update
            .expect("accessibility tree")
    }

    /// A frame that also reports a native window rect, so the size the mini
    /// player saves can be observed.
    fn sized_frame(ctx: &egui::Context, app: &mut App, size: Vec2) {
        let mut viewports = egui::ViewportIdMap::default();
        viewports.insert(
            egui::ViewportId::ROOT,
            egui::ViewportInfo {
                inner_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, size)),
                ..Default::default()
            },
        );
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, size)),
                viewports,
                ..Default::default()
            },
            |ui| show(app, ui),
        );
        output.textures_delta.clear();
    }

    fn node(tree: &TreeUpdate, label: &str, role: Role) -> NodeId {
        tree.nodes
            .iter()
            .find(|(_, node)| node.label() == Some(label) && node.role() == role)
            .unwrap_or_else(|| panic!("missing {label:?} ({role:?})"))
            .0
    }

    fn event(target: NodeId, action: AccessibleAction, data: Option<ActionData>) -> egui::Event {
        egui::Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
            target_tree: egui::accesskit::TreeId::ROOT,
            target_node: target,
            action,
            data,
        })
    }

    #[test]
    fn forms_follow_height_before_width() {
        for (size, expected) in [
            (vec2(240.0, 200.0), Form::Tall),
            (vec2(900.0, 199.0), Form::Stacked),
            (vec2(240.0, 116.0), Form::Stacked),
            (vec2(299.0, 115.0), Form::Micro),
            (vec2(300.0, 80.0), Form::Compact),
            (vec2(519.0, 80.0), Form::Compact),
            (vec2(520.0, 80.0), Form::Wide),
        ] {
            assert_eq!(form(size), expected);
        }
    }

    #[test]
    fn dragging_trades_pixels_for_a_fitting_prefix_including_eq_and_pl() {
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let mut output = ctx.run_ui(Default::default(), |ui| {
            for width in [220.0, 440.0, 1180.0] {
                assert!((split_info_width(width, 0.0) - INFO_MIN).abs() < 0.001);
                assert!((width - split_info_width(width, 1.0) - CONTROLS_MIN).abs() < 0.001);
            }
            let mut previous = Vec::new();
            for width in 30..600 {
                let kept = control_ladder(ui, width as f32, None);
                let used = kept.iter().map(|candidate| candidate.width).sum::<f32>()
                    + CONTROL_SPACING * kept.len().saturating_sub(1) as f32;
                assert!(used <= width as f32);
                let controls: Vec<_> = kept.iter().map(|candidate| candidate.control).collect();
                assert!(controls.starts_with(&previous));
                previous = controls;
            }
            assert_eq!(
                control_ladder(ui, 58.0, None)
                    .iter()
                    .map(|candidate| candidate.control)
                    .collect::<Vec<_>>(),
                vec![Control::Play, Control::Next]
            );
            let width = 440.0 - split_info_width(440.0, 0.0) - 2.0 * CONTROL_PADDING;
            assert!(
                control_ladder(ui, width, None)
                    .iter()
                    .any(|candidate| candidate.control == Control::Playlist)
            );
        });
        output.textures_delta.clear();
    }

    /// The divider's right-hand stop is "everything but the play button", so
    /// the play button has to survive it at every width the window can reach.
    #[test]
    fn the_rightmost_split_always_leaves_room_for_the_play_button() {
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let mut output = ctx.run_ui(Default::default(), |ui| {
            for window in 240..=1200 {
                let width = window as f32 - 20.0;
                let bar = Rect::from_min_size(pos2(3.5, 0.0), vec2(width, 56.0));
                let info = split_info_width(bar.width(), 1.0);
                let controls = Rect::from_min_max(pos2(bar.left() + info, bar.top()), bar.max);
                let budget = (controls.width() - 2.0 * CONTROL_PADDING).max(0.0);
                let kept = control_ladder(ui, budget, None);
                assert_eq!(
                    kept.first().map(|candidate| candidate.control),
                    Some(Control::Play),
                    "window {window}: budget {budget}"
                );
            }
        });
        output.textures_delta.clear();
    }

    /// A meter shorter than its own peak cap must not invert a clamp.
    #[test]
    fn a_meter_thinner_than_its_peak_cap_does_not_panic() {
        let (ctx, mut app) = test_app("thin-meter");
        let mut now = app.now_playing().expect("demo track");
        now.local = true;
        now.playing = true;
        let id = egui::Id::new("mini-meter-smoothing");
        ctx.data_mut(|data| {
            data.insert_temp(
                id,
                MeterState {
                    heights: [15.0; vis::BARS],
                    peaks: [16.0; vis::BARS],
                    last: Instant::now() - std::time::Duration::from_millis(16),
                },
            )
        });
        for height in [0.0, 1.0, 1.9, 30.0] {
            let mut output = ctx.run_ui(Default::default(), |ui| {
                level_meter(
                    &mut app,
                    ui,
                    Some(&now),
                    Rect::from_min_size(pos2(10.0, 10.0), vec2(76.0, height)),
                );
            });
            output.textures_delta.clear();
        }
        app.backend.shutdown();
    }

    /// Every ladder entry is reachable at the default size: what the main row
    /// cannot fit continues in the details row, and nothing is drawn twice.
    #[test]
    fn the_details_row_takes_over_where_the_main_row_runs_out() {
        let (ctx, mut app) = test_app("overflow");
        let size = vec2(460.0, 96.0);
        frame(&ctx, &mut app, size, vec![]);
        let tree = frame(&ctx, &mut app, size, vec![]);
        for label in [
            "EQ",
            "PL",
            "Lyrics",
            "Connect to a device",
            "Open the full window",
        ] {
            let count = tree
                .nodes
                .iter()
                .filter(|(_, node)| node.label() == Some(label) && node.role() == Role::Button)
                .count();
            assert_eq!(
                count, 1,
                "{label} appears {count} times at the default size"
            );
        }
        for (_, node) in &tree.nodes {
            if node.role() == Role::Button
                && let Some(bounds) = node.bounds()
            {
                assert!(
                    bounds.x1 <= f64::from(size.x) && bounds.y1 <= f64::from(size.y),
                    "{:?}: {bounds:?}",
                    node.label()
                );
            }
        }
        app.backend.shutdown();
    }

    /// The details row still shows EQ and PL after the window loses a point of
    /// height, which is what the old exact-equality threshold could not do.
    #[test]
    fn the_details_row_survives_a_small_resize_below_the_default() {
        let (ctx, mut app) = test_app("details-slack");
        for height in [88.0, 95.0, 96.0] {
            let size = vec2(460.0, height);
            frame(&ctx, &mut app, size, vec![]);
            let tree = frame(&ctx, &mut app, size, vec![]);
            for label in ["EQ", "PL"] {
                node(&tree, label, Role::Button);
            }
        }
        app.backend.shutdown();
    }

    /// The bitrate is a download preference and the sample rate is the local
    /// decoder's, so a remote device must show neither.
    #[test]
    fn the_rates_describe_local_playback_only() {
        let (_ctx, mut app) = test_app("rates");
        let mut now = app.now_playing().expect("demo track");
        now.local = true;
        now.playing = true;
        assert!(rates(&app, Some(&now)).is_some());
        now.local = false;
        assert!(rates(&app, Some(&now)).is_none());
        now.local = true;
        now.playing = false;
        now.loading = false;
        now.position_ms = 0;
        assert!(rates(&app, Some(&now)).is_none());
        assert!(rates(&app, None).is_none());
        app.backend.shutdown();
    }

    /// Turning the visualiser off stops the meter, and with it its 60Hz frames.
    #[test]
    fn the_visualiser_setting_silences_the_mini_meter() {
        let (ctx, mut app) = test_app("vis-off");
        let size = vec2(760.0, 96.0);
        app.settings.vis = VisMode::Off;
        frame(&ctx, &mut app, size, vec![]);
        let before =
            ctx.data(|data| data.get_temp::<MeterState>(egui::Id::new("mini-meter-smoothing")));
        assert!(
            before.is_none(),
            "the meter must not run with the visualiser off"
        );
        app.settings.vis = VisMode::Bars;
        frame(&ctx, &mut app, size, vec![]);
        assert!(
            ctx.data(|data| data.get_temp::<MeterState>(egui::Id::new("mini-meter-smoothing")))
                .is_some()
        );
        app.backend.shutdown();
    }

    #[test]
    fn default_details_are_visible_and_dispatch_real_actions() {
        let (ctx, mut app) = test_app("details");
        let size = vec2(460.0, 96.0);
        for palette in [Palette::dark(), Palette::light()] {
            app.palette = palette;
            theme::apply(&ctx, &palette);
            frame(&ctx, &mut app, size, vec![]);
            let tree = frame(&ctx, &mut app, size, vec![]);
            for label in ["EQ", "PL"] {
                let id = node(&tree, label, Role::Button);
                let bounds = tree
                    .nodes
                    .iter()
                    .find(|(node_id, _)| *node_id == id)
                    .unwrap()
                    .1
                    .bounds()
                    .expect("button bounds");
                assert!(bounds.y0 >= 8.0 && bounds.y1 <= 88.0, "{label}: {bounds:?}");
                app.actions.clear();
                frame(
                    &ctx,
                    &mut app,
                    size,
                    vec![event(id, AccessibleAction::Click, None)],
                );
                assert!(app.actions.iter().any(|action| matches!(
                    (label, action),
                    ("EQ", Action::ToggleEqualizerWindow) | ("PL", Action::TogglePlaylistWindow)
                )));
            }
        }
        app.backend.shutdown();
    }

    #[test]
    fn settings_persist_hide_controls_and_block_underlying_buttons() {
        let (ctx, mut app) = test_app("settings");
        let size = vec2(460.0, 96.0);
        frame(&ctx, &mut app, size, vec![]);
        let tree = frame(&ctx, &mut app, size, vec![]);
        let pause = node(&tree, "Pause", Role::Button);
        app.show_mini_player_settings = true;
        frame(&ctx, &mut app, size, vec![]);
        app.actions.clear();
        let tree = frame(
            &ctx,
            &mut app,
            size,
            vec![event(pause, AccessibleAction::Click, None)],
        );
        assert!(
            !app.actions
                .iter()
                .any(|action| matches!(action, Action::TogglePlay))
        );
        for label in ["Background colour", "Playback row"] {
            let id = node(&tree, label, Role::CheckBox);
            app.actions.clear();
            frame(
                &ctx,
                &mut app,
                size,
                vec![event(id, AccessibleAction::Click, None)],
            );
            assert!(
                app.actions
                    .iter()
                    .any(|action| matches!(action, Action::SettingsChanged))
            );
        }
        assert!(app.settings.mini_player_tint_background);
        assert!(!app.settings.mini_player_show_controls);
        let done = node(&tree, "Done", Role::Button);
        frame(
            &ctx,
            &mut app,
            size,
            vec![event(done, AccessibleAction::Click, None)],
        );
        assert!(!app.show_mini_player_settings);
        let tree = frame(&ctx, &mut app, size, vec![]);
        assert!(
            !tree
                .nodes
                .iter()
                .any(|(_, node)| matches!(node.label(), Some("Pause" | "Next" | "EQ" | "PL")))
        );
        app.backend.shutdown();
    }

    #[test]
    fn tall_seek_and_add_use_the_current_track() {
        let (ctx, mut app) = test_app("seek-add");
        let size = vec2(460.0, 360.0);
        frame(&ctx, &mut app, size, vec![]);
        let tree = frame(&ctx, &mut app, size, vec![]);
        let now = app.now_playing().expect("demo track");
        let seek = node(&tree, "Playback position (%)", Role::Slider);
        app.actions.clear();
        frame(
            &ctx,
            &mut app,
            size,
            vec![event(
                seek,
                AccessibleAction::SetValue,
                Some(ActionData::NumericValue(37.5)),
            )],
        );
        assert!(app.seek_preview.is_none());
        assert!(app.actions.iter().any(|action| matches!(action, Action::Seek(ms) if *ms == (now.duration_ms as f32 * 0.375) as u32)));
        let add = node(&tree, "Add the current track to the queue", Role::Button);
        app.actions.clear();
        frame(
            &ctx,
            &mut app,
            size,
            vec![event(add, AccessibleAction::Click, None)],
        );
        assert!(app.actions.iter().any(|action| matches!(action, Action::AddToQueue { uri, label } if uri == &now.uri && label == &now.title)));
        app.backend.shutdown();
    }

    #[test]
    fn smoothing_uses_elapsed_time_with_faster_attack_and_slower_peak_decay() {
        for (start, target, fall) in [(0.0, 15.0, 12.0), (15.0, 0.0, 12.0), (16.0, 0.0, 6.0)] {
            let whole = smooth(start, target, 0.1, fall);
            let divided = (0..10).fold(start, |value, _| smooth(value, target, 0.01, fall));
            assert!((whole - divided).abs() < 0.0001);
        }
        assert!(smooth(0.0, 15.0, 0.03, 12.0) > 15.0 - smooth(15.0, 0.0, 0.03, 12.0));
        assert!(smooth(15.0, 0.0, 0.03, 6.0) > smooth(15.0, 0.0, 0.03, 12.0));
    }

    #[test]
    fn all_forms_keep_emitted_controls_inside_the_window() {
        let (ctx, mut app) = test_app("forms-fit");
        for palette in [Palette::dark(), Palette::light()] {
            app.palette = palette;
            theme::apply(&ctx, &palette);
            for size in [
                vec2(240.0, 56.0),
                vec2(319.0, 72.0),
                vec2(460.0, 96.0),
                vec2(760.0, 96.0),
                vec2(300.0, 132.0),
                vec2(460.0, 216.0),
                vec2(1200.0, 460.0),
            ] {
                for split in [0.0, 0.5, 1.0] {
                    app.settings.mini_player_split = split;
                    frame(&ctx, &mut app, size, vec![]);
                    let tree = frame(&ctx, &mut app, size, vec![]);
                    node(&tree, "Pause", Role::Button);
                    for (_, node) in &tree.nodes {
                        if node.role() == Role::Button
                            && let Some(bounds) = node.bounds()
                        {
                            assert!(
                                bounds.x0 >= 0.0
                                    && bounds.x1 <= f64::from(size.x)
                                    && bounds.y0 >= 0.0
                                    && bounds.y1 <= f64::from(size.y),
                                "{:?} at {size:?}, split {split}: {bounds:?}",
                                node.label()
                            );
                        }
                    }
                    let gear = tree
                        .nodes
                        .iter()
                        .any(|(_, node)| node.label() == Some("Mini player settings"));
                    assert_eq!(gear, size.y - 16.0 >= 116.0);
                    let seek = tree
                        .nodes
                        .iter()
                        .any(|(_, node)| node.role() == Role::Slider);
                    assert_eq!(seek, size.y - 16.0 >= 200.0);
                }
            }
        }
        app.backend.shutdown();
    }

    #[test]
    fn pointer_drag_persists_the_split_without_repeating_unchanged_writes() {
        let (ctx, mut app) = test_app("split-drag");
        let size = vec2(460.0, 96.0);
        frame(&ctx, &mut app, size, vec![]);
        frame(&ctx, &mut app, size, vec![]);
        let start = pos2(230.0, 30.0);
        let end = pos2(50.0, 30.0);
        frame(
            &ctx,
            &mut app,
            size,
            vec![
                egui::Event::PointerMoved(start),
                egui::Event::PointerButton {
                    pos: start,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        app.actions.clear();
        frame(&ctx, &mut app, size, vec![egui::Event::PointerMoved(end)]);
        assert!((app.settings.mini_player_split - 84.0 / 440.0).abs() < 0.001);
        assert_eq!(
            app.actions
                .iter()
                .filter(|action| matches!(action, Action::SettingsChanged))
                .count(),
            1
        );
        app.actions.clear();
        frame(&ctx, &mut app, size, vec![]);
        assert!(
            !app.actions
                .iter()
                .any(|action| matches!(action, Action::SettingsChanged))
        );
        app.backend.shutdown();
    }

    #[test]
    fn the_bottom_seek_bar_appears_only_under_the_pointer_and_really_seeks() {
        let (ctx, mut app) = test_app("hover-seek");
        let size = vec2(460.0, 96.0);
        let now = app.now_playing().expect("demo track");
        frame(&ctx, &mut app, size, vec![]);
        let tree = frame(&ctx, &mut app, size, vec![]);
        assert!(
            !tree
                .nodes
                .iter()
                .any(|(_, node)| node.role() == Role::Slider),
            "the seek bar must stay hidden while the pointer is away"
        );
        let inside = pos2(size.x / 2.0, size.y - 4.0);
        let mut seek = None;
        for _ in 0..12 {
            let tree = frame(
                &ctx,
                &mut app,
                size,
                vec![egui::Event::PointerMoved(inside)],
            );
            if let Some((id, _)) = tree
                .nodes
                .iter()
                .find(|(_, node)| node.role() == Role::Slider)
            {
                seek = Some(*id);
                break;
            }
        }
        let seek = seek.expect("hovering the mini player reveals the seek bar");
        let tree = frame(
            &ctx,
            &mut app,
            size,
            vec![egui::Event::PointerMoved(inside)],
        );
        let bounds = tree
            .nodes
            .iter()
            .find(|(id, _)| *id == seek)
            .and_then(|(_, node)| node.bounds())
            .expect("seek bar bounds");
        assert!(
            bounds.y1 <= f64::from(size.y) && bounds.y0 >= 0.0,
            "the grab area must sit inside the window: {bounds:?}"
        );
        app.actions.clear();
        frame(
            &ctx,
            &mut app,
            size,
            vec![
                egui::Event::PointerMoved(inside),
                event(
                    seek,
                    AccessibleAction::SetValue,
                    Some(ActionData::NumericValue(25.0)),
                ),
            ],
        );
        assert!(app.actions.iter().any(
            |action| matches!(action, Action::Seek(ms) if *ms == (now.duration_ms as f32 * 0.25) as u32)
        ));
        app.backend.shutdown();
    }

    /// The viewport builder takes the same points the viewport reports back,
    /// whatever the zoom factor, so the saved size has to be the reading
    /// itself. Scaling it by the zoom grew the window on every launch until
    /// it hit the ceiling below, and shrank it to the floor under 1.0.
    #[test]
    fn the_saved_size_survives_a_zoom_factor_across_launches() {
        for zoom in [0.8, 1.0, 1.25] {
            let (ctx, mut app) = test_app("zoom-size");
            let size = vec2(460.0, 96.0);
            ctx.set_zoom_factor(zoom);
            sized_frame(&ctx, &mut app, size);
            assert_eq!(app.settings.mini_player_size, [size.x, size.y]);
            // What was saved is what the next launch asks for, so a window
            // that opens at that size must leave the setting alone.
            sized_frame(&ctx, &mut app, size);
            assert_eq!(app.settings.mini_player_size, [size.x, size.y]);
            app.backend.shutdown();
        }
    }

    /// The mini player is the only window on screen while it is open, so the
    /// shortcuts that leave it have to be handled from inside it.
    #[test]
    fn the_shortcuts_that_leave_the_mini_player_work_from_inside_it() {
        let (ctx, mut app) = test_app("keys");
        let size = vec2(280.0, 72.0);
        frame(&ctx, &mut app, size, vec![]);
        for (modifiers, key, expected) in [
            (
                if cfg!(target_os = "macos") {
                    egui::Modifiers::COMMAND | egui::Modifiers::SHIFT
                } else {
                    egui::Modifiers::COMMAND
                },
                egui::Key::M,
                Action::ToggleMiniPlayer,
            ),
            (
                egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                egui::Key::K,
                Action::ToggleWinampMilkdrop,
            ),
            (egui::Modifiers::COMMAND, egui::Key::W, Action::CloseWindow),
        ] {
            app.actions.clear();
            frame(
                &ctx,
                &mut app,
                size,
                vec![egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers,
                }],
            );
            assert!(
                app.actions
                    .iter()
                    .any(|action| std::mem::discriminant(action)
                        == std::mem::discriminant(&expected)),
                "{expected:?} never reached the mini player"
            );
        }
        app.backend.shutdown();
    }

    /// A window too narrow for the connect button can still open the panel that
    /// leaves the mini player.
    #[test]
    fn the_settings_panel_can_leave_the_mini_player_at_any_size() {
        let (ctx, mut app) = test_app("escape");
        let size = vec2(280.0, 72.0);
        app.show_mini_player_settings = true;
        frame(&ctx, &mut app, size, vec![]);
        let tree = frame(&ctx, &mut app, size, vec![]);
        let id = node(&tree, "Open the full window", Role::Button);
        app.actions.clear();
        frame(
            &ctx,
            &mut app,
            size,
            vec![event(id, AccessibleAction::Click, None)],
        );
        assert!(
            app.actions
                .iter()
                .any(|action| matches!(action, Action::ToggleMiniPlayer))
        );
        assert!(!app.show_mini_player_settings);
        app.backend.shutdown();
    }

    #[test]
    fn remote_playback_clears_a_previously_moving_meter() {
        let (ctx, mut app) = test_app("remote-meter");
        let mut now = app.now_playing().expect("demo track");
        now.local = false;
        now.playing = true;
        let id = egui::Id::new("mini-meter-smoothing");
        ctx.data_mut(|data| {
            data.insert_temp(
                id,
                MeterState {
                    heights: [15.0; vis::BARS],
                    peaks: [16.0; vis::BARS],
                    last: Instant::now() - std::time::Duration::from_millis(16),
                },
            )
        });
        let mut output = ctx.run_ui(Default::default(), |ui| {
            level_meter(
                &mut app,
                ui,
                Some(&now),
                Rect::from_min_size(pos2(10.0, 10.0), vec2(76.0, 30.0)),
            );
        });
        output.textures_delta.clear();
        let state = ctx
            .data(|data| data.get_temp::<MeterState>(id))
            .expect("meter state");
        assert_eq!(state.heights, [0.0; vis::BARS]);
        assert_eq!(state.peaks, [0.0; vis::BARS]);
        app.backend.shutdown();
    }

    #[test]
    fn the_volume_button_opens_a_slider_that_floats_over_the_rows_below() {
        let (ctx, mut app) = test_app("volume-popup");
        let size = vec2(900.0, 120.0);
        let tree = frame(&ctx, &mut app, size, Vec::new());
        assert!(
            !tree
                .nodes
                .iter()
                .any(|(_, node)| node.label() == Some("Volume (%)")),
            "the slider must stay closed until the button is pressed"
        );

        let button = node(&tree, "Volume", Role::Button);
        frame(
            &ctx,
            &mut app,
            size,
            vec![event(button, AccessibleAction::Click, None)],
        );
        assert!(
            app.actions
                .iter()
                .any(|action| matches!(action, Action::ToggleVolumePopup)),
            "pressing the volume button must ask to open the slider, got {:?}",
            app.actions
        );

        app.actions.clear();
        app.show_volume_popup = true;
        let tree = frame(&ctx, &mut app, size, Vec::new());
        let slider = tree
            .nodes
            .iter()
            .find(|(_, node)| node.label() == Some("Volume (%)"))
            .expect("the volume slider is rendered while the popup is open");
        // It has to sit over the rows below the button, not push them aside.
        let bounds = slider.1.bounds().expect("slider bounds");
        let anchor = ctx
            .data(|data| data.get_temp::<Rect>(egui::Id::new(VOLUME_BUTTON_RECT_ID)))
            .expect("the button records where the slider anchors");
        let area = ctx
            .memory(|memory| memory.area_rect(egui::Id::new("mini-volume-popup")))
            .expect("the popup area is laid out");
        assert!(
            area.bottom() <= size.y && area.right() <= size.x && area.left() >= 0.0,
            "the popup at {area:?} must stay inside the {size:?} window"
        );
        assert!(
            bounds.y0 >= f64::from(anchor.bottom()),
            "slider at {bounds:?} should hang below the button at {anchor:?}"
        );

        // A window too short to hold the slider below the button has to flip it
        // above rather than hang it off the bottom edge.
        for short in [vec2(900.0, 72.0), vec2(320.0, 96.0)] {
            frame(&ctx, &mut app, short, Vec::new());
            if let Some(area) =
                ctx.memory(|memory| memory.area_rect(egui::Id::new("mini-volume-popup")))
            {
                assert!(
                    area.bottom() <= short.y && area.right() <= short.x && area.left() >= 0.0,
                    "the popup at {area:?} must stay inside the {short:?} window"
                );
            }
        }

        app.backend.shutdown();
    }
}
