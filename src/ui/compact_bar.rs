//! A small, borderless, resizable "now playing" bar: cover, title/artist,
//! a small reactive level meter, and transport controls in a single row.
//! A simpler alternative to the Winamp mini player (`src/ui/winamp/`),
//! which stays a fixed-size, pixel-skinned window -- this one is a plain
//! egui layout the user can resize.

use std::time::Instant;

use egui::{Color32, Vec2};

use crate::app::{App, NowPlaying};
use crate::model::Action;
use crate::player::RepeatMode;
use crate::theme::{self, Icon};
use crate::vis;

use super::widgets;

/// Reserved for the transport controls (repeat, next, play, previous,
/// shuffle, connect, like, exit-compact -- each an `icon_button`'s edge
/// plus the default item spacing between them), so the title/artist column
/// knows how much room it actually has and can truncate instead of
/// overlapping the level meter or the controls themselves.
const CONTROLS_WIDTH: f32 = 300.0;
const METER_WIDTH: f32 = 64.0;
const METER_BARS: usize = 14;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    egui::Frame::new()
        .fill(palette.window)
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            let now = app.now_playing();
            ui.horizontal_centered(|ui| {
                let cover_size = ui.available_height().min(56.0);
                let (cover_rect, _) =
                    ui.allocate_exact_size(Vec2::splat(cover_size), egui::Sense::hover());
                widgets::paint_cover(
                    ui,
                    &palette,
                    now.as_ref()
                        .and_then(|now| now.art_small.as_deref().or(now.art_url.as_deref())),
                    cover_rect,
                    6.0,
                    Icon::Music,
                    Some(app.backend.art()),
                );

                ui.add_space(10.0);
                let info_width = (ui.available_width() - METER_WIDTH - CONTROLS_WIDTH - 20.0)
                    .max(60.0);
                ui.scope(|ui| {
                    ui.set_max_width(info_width);
                    ui.vertical(|ui| {
                        ui.add_space((ui.available_height() - 34.0).max(0.0) / 2.0);
                        let title = now
                            .as_ref()
                            .map(|now| now.title.as_str())
                            .unwrap_or("Nothing playing");
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(title)
                                    .font(theme::medium(14.0))
                                    .color(palette.text),
                            )
                            .truncate(),
                        );
                        let artist = now.as_ref().map(|now| now.subtitle.as_str()).unwrap_or("");
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(artist)
                                    .font(theme::regular(12.0))
                                    .color(palette.secondary),
                            )
                            .truncate(),
                        );
                    });
                });

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);
                level_meter(app, ui, now.as_ref());

                ui.with_layout(
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| transport(app, ui, now.as_ref()),
                );
            });
        });
    // Clamped: on the frame this window opens, or the one where sign-in
    // forces it closed again, egui can report a stale or transitional
    // `inner_rect` (once literally the outgoing main window's full-screen
    // size). Ignoring anything outside a sane bar size keeps a corrupt
    // reading from ever reaching disk.
    let size = ui.ctx().input(|input| input.viewport().inner_rect);
    if let Some(rect) = size {
        let size = [rect.width().clamp(260.0, 900.0), rect.height().clamp(56.0, 220.0)];
        if (app.settings.compact_bar_size[0] - size[0]).abs() > 1.0
            || (app.settings.compact_bar_size[1] - size[1]).abs() > 1.0
        {
            app.settings.compact_bar_size = size;
            app.actions.push(Action::SettingsChanged);
        }
    }
}

/// A small bar-style level meter, tinted with the app's accent colour.
/// Reactive only for local playback -- Fastpotify has no access to the
/// audio stream of a track playing on a remote Spotify Connect device, so
/// it sits flat there rather than fake it.
fn level_meter(app: &mut App, ui: &mut egui::Ui, now: Option<&NowPlaying>) {
    let palette = app.palette;
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(METER_WIDTH, ui.available_height().min(28.0)), egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let sounding = now.is_some_and(|now| (now.playing || now.loading) && now.local);
    let samples = if sounding {
        app.winamp.tap.window(vis::FFT_SAMPLES, vis::LAG)
    } else {
        vec![0.0; vis::FFT_SAMPLES]
    };
    let bars = app.winamp.analyser.step(&samples, Instant::now());
    let bar_gap = 2.0;
    let bar_width = (rect.width() - bar_gap * (METER_BARS as f32 - 1.0)) / METER_BARS as f32;
    let painter = ui.painter();
    for i in 0..METER_BARS {
        // 19 real bars downsampled to the meter's own bar count.
        let source = bars[i * vis::BARS / METER_BARS];
        let level = f32::from(source.height) / f32::from(vis::ROWS);
        let height = (rect.height() * level).max(2.0);
        let x = rect.left() + i as f32 * (bar_width + bar_gap);
        let bar_rect = egui::Rect::from_min_size(
            egui::pos2(x, rect.bottom() - height),
            Vec2::new(bar_width, height),
        );
        let color = if level > 0.05 {
            palette.accent
        } else {
            Color32::from_rgba_unmultiplied(
                palette.accent.r(),
                palette.accent.g(),
                palette.accent.b(),
                60,
            )
        };
        painter.rect_filled(bar_rect, 1.0, color);
    }
}

fn transport(app: &mut App, ui: &mut egui::Ui, now: Option<&NowPlaying>) {
    let palette = app.palette;
    let enabled = now.is_some_and(|now| now.can_control) || app.is_connected();
    let playing = now.is_some_and(|now| now.playing);
    let shuffle = now.is_some_and(|now| now.shuffle);
    let repeat = now.map(|now| now.repeat).unwrap_or_default();
    let dim = if enabled {
        palette.secondary
    } else {
        palette.dim
    };

    // `right_to_left`: each widget is placed starting from the right, so
    // the *first* one added ends up rightmost. Add in reverse of the
    // left-to-right order Spotify's own compact bar uses (like, connect,
    // shuffle, previous, play, next, repeat, exit-compact).
    if theme::icon_button(
        ui,
        Icon::ExternalLink,
        15.0,
        palette.secondary,
        palette.text,
        "Open the full window",
    )
    .clicked()
    {
        app.actions.push(Action::ToggleCompactBarWindow);
    }
    ui.add_space(4.0);

    let (repeat_icon, repeat_color) = match repeat {
        RepeatMode::Off => (Icon::Repeat, dim),
        RepeatMode::Context => (Icon::Repeat, palette.accent),
        RepeatMode::Track => (Icon::Repeat1, palette.accent),
    };
    if theme::icon_button(ui, repeat_icon, 15.0, repeat_color, palette.text, "Repeat").clicked() {
        app.actions.push(Action::CycleRepeat);
    }
    if theme::icon_button(ui, Icon::SkipForward, 16.0, dim, palette.text, "Next").clicked() {
        app.actions.push(Action::Next);
    }
    let icon = if playing {
        Icon::PauseFilled
    } else {
        Icon::PlayFilled
    };
    if theme::circle_button(ui, icon, 30.0, palette.text, palette.text, palette.window, "Play")
        .clicked()
    {
        app.actions.push(Action::TogglePlay);
    }
    if theme::icon_button(ui, Icon::SkipBack, 16.0, dim, palette.text, "Previous").clicked() {
        app.actions.push(Action::Previous);
    }
    let shuffle_color = if shuffle { palette.accent } else { dim };
    if theme::icon_button(ui, Icon::Shuffle, 15.0, shuffle_color, palette.text, "Shuffle")
        .clicked()
    {
        app.actions.push(Action::ToggleShuffle);
    }

    let remote = now.is_some_and(|now| !now.local);
    let devices = theme::icon_button(
        ui,
        Icon::Speaker,
        15.0,
        if remote { palette.accent } else { dim },
        palette.text,
        "Connect to a device",
    );
    ui.ctx().data_mut(|data| {
        data.insert_temp(
            egui::Id::new(super::devices::BUTTON_RECT_ID),
            devices.rect,
        )
    });
    if devices.clicked() {
        app.actions.push(Action::ToggleDevicesPopup);
    }

    if let Some(now) = now {
        if !now.is_episode {
            let saved = app.is_saved(&now.uri).unwrap_or(false);
            let color = if saved { palette.accent } else { palette.dim };
            let uri = now.uri.clone();
            if theme::icon_button(
                ui,
                Icon::CircleCheck,
                15.0,
                color,
                palette.text,
                "Save to Liked Songs",
            )
            .clicked()
            {
                app.actions.push(Action::ToggleSaved(uri));
            }
        }
    }
    super::devices::popup(app, ui.ctx());
}
