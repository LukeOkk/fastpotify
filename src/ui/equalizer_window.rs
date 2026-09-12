//! The compact bar's own equalizer section: a real, plain-egui ten-band
//! equalizer bound to the same [`crate::eq`] settings the classic Winamp
//! skin's equalizer window (`src/ui/winamp/equalizer.rs`) already drives.
//!
//! Like that window, this one never touches `app.winamp.eq` (the mutex the
//! player thread reads) directly: every control here reads a snapshot via
//! [`crate::app::eq_settings`] and writes back through the same `Action`s
//! the skinned equalizer uses (`ToggleEq`, `SetEqBand`, `SetEqPreamp`,
//! `ApplyEqPreset`), which apply to `app.settings` and then push the fresh
//! settings into the shared mutex for the audio thread. That is the
//! existing, already-correct way this app keeps the UI and the player in
//! sync, so this window reuses it rather than re-deriving its own locking.

use egui::{Align2, FontId};

use crate::app::App;
use crate::eq::{self, RANGE_DB};
use crate::model::Action;
use crate::theme::{self, Palette};

/// The length of each vertical slider's travel.
const SLIDER_HEIGHT: f32 = 96.0;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let settings = crate::app::eq_settings(&app.settings);

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        if theme::pill_button(ui, &palette, "ON", settings.on).clicked() {
            app.actions.push(Action::ToggleEq);
        }
        // Winamp's AUTO loaded a per-song preset, which Spotify has no
        // equivalent for; the real, honest mapping here is the app's own
        // loudness normalisation setting -- also a mutex-free, honest
        // choice already used by the skinned equalizer's own AUTO button.
        let auto = app.settings.normalisation;
        if theme::pill_button(ui, &palette, "AUTO", auto)
            .on_hover_text("Loudness normalisation (Settings > Playback). Takes effect on the next restart of local playback.")
            .clicked()
        {
            app.settings.normalisation = !auto;
            app.actions.push(Action::SettingsChanged);
        }

        ui.add_space(6.0);
        theme::text(ui, "PRESETS", theme::regular(11.0), palette.secondary);
        let current_label = eq::PRESETS
            .iter()
            .find(|preset| preset.bands_db == app.settings.eq_bands_db)
            .map(|preset| preset.name)
            .unwrap_or("Custom");
        egui::ComboBox::new("compact-eq-presets", "")
            .selected_text(current_label)
            .show_ui(ui, |ui| {
                for (index, preset) in eq::PRESETS.iter().enumerate() {
                    if index == eq::WINAMP_PRESET_COUNT {
                        ui.separator();
                    }
                    let chosen = preset.bands_db == app.settings.eq_bands_db;
                    if ui.selectable_label(chosen, preset.name).clicked() {
                        app.actions.push(Action::ApplyEqPreset(index));
                    }
                }
            });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if theme::icon_button(
                ui,
                theme::Icon::X,
                14.0,
                palette.secondary,
                palette.text,
                "Close equalizer",
            )
            .clicked()
            {
                app.actions.push(Action::ToggleEqualizerWindow);
            }
        });
    });

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        db_scale(ui, &palette);
        if let Some(value) = vertical_band(ui, &palette, "PREAMP", settings.preamp_db, gain_slider)
        {
            app.actions.push(Action::SetEqPreamp(value));
        }

        ui.separator();

        for (index, hz) in eq::BANDS.iter().enumerate() {
            let label = band_label(*hz);
            let gain = settings.bands_db[index];
            if let Some(value) = vertical_band(ui, &palette, &label, gain, gain_slider) {
                app.actions.push(Action::SetEqBand(index, value));
            }
        }
    });
    ui.add_space(4.0);
}

/// One vertical slider with its label underneath, decibels above it while
/// dragging. Returns the new value on a change.
fn vertical_band(
    ui: &mut egui::Ui,
    palette: &Palette,
    label: &str,
    value: f32,
    slider: impl FnOnce(&mut egui::Ui, &Palette, &mut f32) -> bool,
) -> Option<f32> {
    let mut value = value;
    let mut changed = false;
    ui.vertical(|ui| {
        ui.set_width(26.0);
        ui.vertical_centered(|ui| {
            changed = slider(ui, palette, &mut value);
            ui.add_space(2.0);
            theme::text(ui, label, theme::regular(9.5), palette.dim);
        });
    });
    changed.then_some(value)
}

/// A vertical decibel slider, twelve either way, with no built-in numeric
/// readout (the label underneath is the frequency, not the value).
fn gain_slider(ui: &mut egui::Ui, _palette: &Palette, value: &mut f32) -> bool {
    ui.spacing_mut().slider_width = SLIDER_HEIGHT;
    let response = ui.add(
        egui::Slider::new(value, -RANGE_DB..=RANGE_DB)
            .vertical()
            .show_value(false),
    );
    response.changed()
}

/// The frequencies as Winamp's own equalizer labelled them: below 1 kHz in
/// hertz, at and above it in kilohertz.
fn band_label(hz: f32) -> String {
    if hz >= 1000.0 {
        format!("{:.0}K", hz / 1000.0)
    } else {
        format!("{hz:.0}")
    }
}

/// A small +12/0/-12 dB reference column, level with the sliders beside it.
fn db_scale(ui: &mut egui::Ui, palette: &Palette) {
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(28.0, SLIDER_HEIGHT), egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let painter = ui.painter();
    let font = FontId::monospace(8.5);
    for (label, t) in [("+12", 0.0_f32), ("0", 0.5), ("-12", 1.0)] {
        let y = rect.top() + t * rect.height();
        painter.text(
            egui::pos2(rect.left(), y),
            Align2::LEFT_CENTER,
            label,
            font.clone(),
            palette.dim,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn band_labels_match_winamp_formatting() {
        assert_eq!(band_label(60.0), "60");
        assert_eq!(band_label(170.0), "170");
        assert_eq!(band_label(310.0), "310");
        assert_eq!(band_label(600.0), "600");
        assert_eq!(band_label(1000.0), "1K");
        assert_eq!(band_label(3000.0), "3K");
        assert_eq!(band_label(6000.0), "6K");
        assert_eq!(band_label(12000.0), "12K");
        assert_eq!(band_label(14000.0), "14K");
        assert_eq!(band_label(16000.0), "16K");
    }
}
