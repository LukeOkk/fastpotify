//! The words of the playing track, in a side panel that follows the song.

use egui::{Align, Frame, Layout, Margin, Sense};

use crate::app::App;
use crate::model::{Action, Loadable};
use crate::theme::{self, Icon};

use super::widgets;

use crate::lyrics::TRANSLATE_LANGUAGES;

const LINE_SIZE: f32 = 19.0;
const LINE_GAP: f32 = 10.0;
/// How long a line takes to light up or fade.
const LIGHT_UP_SECONDS: f32 = 0.22;

fn blend(from: egui::Color32, to: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    egui::Color32::from(egui::Rgba::from(from) * (1.0 - t) + egui::Rgba::from(to) * t)
}

pub fn side_panel(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let panel = egui::Panel::right("lyrics-panel")
        .resizable(true)
        .default_size(app.settings.lyrics_width)
        .size_range(theme::SIDE_PANEL_MIN_WIDTH..=640.0)
        .show_separator_line(false)
        .frame(
            Frame::new()
                .fill(palette.panel)
                .inner_margin(Margin::symmetric(12, 12)),
        );
    let response = panel.show(ui, |ui| {
        let window_controls = super::window_controls_reservation(
            ui.ctx(),
            app.show_queue_panel,
            app.show_lyrics_panel,
            ui.available_width(),
        );
        ui.add_space(window_controls.lyrics_top);
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            theme::text(ui, "Lyrics", theme::bold(18.0), palette.text);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if theme::icon_button(ui, Icon::X, 18.0, palette.secondary, palette.text, "Close")
                    .clicked()
                {
                    app.actions.push(Action::ToggleLyricsPanel);
                }
                let loaded = matches!(&app.lyrics, Loadable::Loaded(Some(_)));
                if loaded
                    && !app.lyrics_following
                    && theme::pill_button(ui, &palette, "Follow", false).clicked()
                {
                    app.lyrics_following = true;
                    app.lyrics_line_shown = None;
                }
                if loaded
                    && theme::icon_button(
                        ui,
                        Icon::Globe,
                        18.0,
                        if app.settings.lyrics_translate_enabled {
                            palette.accent
                        } else {
                            palette.secondary
                        },
                        palette.text,
                        "Translate lyrics",
                    )
                    .clicked()
                {
                    app.settings.lyrics_translate_enabled = !app.settings.lyrics_translate_enabled;
                    app.actions.push(Action::SettingsChanged);
                    if app.settings.lyrics_translate_enabled {
                        app.maybe_translate_lyrics();
                    }
                }
            });
        });
        if app.settings.lyrics_translate_enabled {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add_space(4.0);
                theme::text(ui, "Translate to", theme::regular(12.5), palette.secondary);
                let current = app.settings.lyrics_translate_language.clone();
                let current_label = current
                    .as_deref()
                    .and_then(|code| {
                        TRANSLATE_LANGUAGES
                            .iter()
                            .find(|(iso, _)| *iso == code)
                            .map(|(_, name)| *name)
                    })
                    .unwrap_or("Automatic (system language)");
                egui::ComboBox::new("lyrics-translate-language", "")
                    .selected_text(current_label)
                    .show_ui(ui, |ui| {
                        let mut changed = false;
                        if ui
                            .selectable_label(current.is_none(), "Automatic (system language)")
                            .clicked()
                            && current.is_some()
                        {
                            app.settings.lyrics_translate_language = None;
                            changed = true;
                        }
                        for (code, name) in TRANSLATE_LANGUAGES {
                            let selected = current.as_deref() == Some(*code);
                            if ui.selectable_label(selected, *name).clicked() && !selected {
                                app.settings.lyrics_translate_language = Some((*code).to_string());
                                changed = true;
                            }
                        }
                        if changed {
                            app.actions.push(Action::SettingsChanged);
                            app.maybe_translate_lyrics();
                        }
                    });
            });
        }
        ui.add_space(8.0);
        contents(app, ui);
    });
    let current_width = response.response.rect.width();
    if (app.settings.lyrics_width - current_width).abs() > 1.0 {
        app.settings.lyrics_width = current_width;
        app.actions.push(Action::SettingsChanged);
    }
}

fn contents(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let Some(now) = app.now_playing() else {
        widgets::empty_state(
            ui,
            &palette,
            Icon::Mic,
            "Nothing playing",
            "Play a song to see its lyrics.",
        );
        return;
    };
    let lyrics = match &app.lyrics {
        Loadable::NotLoaded | Loadable::Loading => {
            widgets::loading_row(ui, &palette);
            return;
        }
        Loadable::Failed(error) => {
            let message = format!("Couldn't fetch the lyrics: {error}");
            ui.add_space(8.0);
            theme::text(ui, message, theme::regular(13.0), palette.secondary);
            ui.add_space(8.0);
            if theme::pill_button(ui, &palette, "Try again", false).clicked() {
                app.request_lyrics();
            }
            return;
        }
        Loadable::Loaded(None) => {
            widgets::empty_state(
                ui,
                &palette,
                Icon::Mic,
                "No lyrics",
                "No lyrics found for this track.",
            );
            return;
        }
        Loadable::Loaded(Some(lyrics)) if lyrics.instrumental => {
            widgets::empty_state(
                ui,
                &palette,
                Icon::Music,
                "Instrumental",
                "No timed lyrics for this track.",
            );
            return;
        }
        Loadable::Loaded(Some(lyrics)) => lyrics.clone(),
    };
    // Translated text swaps in by index; original lines still drive timing
    // and the active-line highlight, since translation never changes line
    // count or `at_ms`.
    let translated_lines = if app.settings.lyrics_translate_enabled {
        match &app.lyrics_translated {
            Loadable::Loaded(lines) if lines.len() == lyrics.lines.len() => Some(lines.clone()),
            _ => None,
        }
    } else {
        None
    };

    let active = lyrics.active_line(now.position_ms);
    let follow = app.lyrics_following && app.lyrics_line_shown != Some(active);
    // The line being sung is bold and in the accent colour; every other
    // line is quiet, regular text, the same before and after it has been
    // sung. A line takes 220 ms to light up or fade, as in omarchy-lyrics.
    let quiet = palette.text.gamma_multiply(0.45);
    let scroll = egui::ScrollArea::vertical()
        .id_salt("lyrics-scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // Before the first line there is nothing to highlight, so the
            // panel sits at the top rather than wherever it was left.
            if follow && lyrics.synced && active.is_none() {
                let top = ui.cursor().min;
                ui.scroll_to_rect(
                    egui::Rect::from_min_size(top, egui::vec2(1.0, 1.0)),
                    Some(Align::Min),
                );
            }
            ui.add_space(12.0);
            for (index, line) in lyrics.lines.iter().enumerate() {
                let is_active = active == Some(index);
                let lit = ui.ctx().animate_bool_with_time(
                    egui::Id::new("lyric-line").with(index),
                    is_active,
                    LIGHT_UP_SECONDS,
                );
                let color = blend(quiet, palette.accent, lit);
                let font = if lit > 0.5 {
                    theme::bold(LINE_SIZE)
                } else {
                    theme::regular(LINE_SIZE)
                };
                let display_text = translated_lines
                    .as_ref()
                    .map_or(line.text.as_str(), |lines| lines[index].text.as_str());
                // A timed line with no words is the band playing on.
                let text = if display_text.is_empty() && lyrics.synced {
                    "\u{266a}"
                } else {
                    display_text
                };
                let sense = if lyrics.synced {
                    Sense::click()
                } else {
                    Sense::hover()
                };
                let response = if crate::bidi::is_rtl(text) {
                    let galley = crate::bidi::layout(
                        ui.painter(),
                        text,
                        font,
                        color,
                        ui.available_width(),
                        usize::MAX,
                        None,
                    );
                    ui.add(egui::Label::new(galley).sense(sense))
                } else {
                    ui.add(
                        egui::Label::new(egui::RichText::new(text).font(font).color(color))
                            .sense(sense),
                    )
                };
                let rect = response.rect;
                if lyrics.synced {
                    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
                    if response.clicked()
                        && let Some(at_ms) = line.at_ms
                    {
                        app.actions.push(Action::Seek(at_ms));
                        app.lyrics_following = true;
                    }
                }
                if is_active && follow {
                    ui.scroll_to_rect(rect, Some(Align::Center));
                }
                ui.add_space(LINE_GAP);
            }
            // Words without timing can only be followed by the clock: sit
            // at the part of the text the song is probably at.
            if app.lyrics_following && !lyrics.synced && now.duration_ms > 0 {
                let fraction =
                    (f64::from(now.position_ms) / f64::from(now.duration_ms)).clamp(0.0, 1.0);
                let content = ui.min_rect();
                let y = content.top() + content.height() * fraction as f32;
                ui.scroll_to_rect(
                    egui::Rect::from_min_max(
                        egui::pos2(content.left(), y),
                        egui::pos2(content.right(), y + 1.0),
                    ),
                    Some(Align::Center),
                );
            }
            ui.add_space(60.0);
        });
    // Scrolling by hand means the reader wants to look elsewhere; the
    // Follow button in the header picks the song back up.
    if ui.rect_contains_pointer(scroll.inner_rect)
        && ui.input(|input| input.smooth_scroll_delta.y != 0.0)
    {
        app.lyrics_following = false;
    }
    app.lyrics_line_shown = Some(active);
}
