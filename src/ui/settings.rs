//! The Settings page.

use egui::{Align, CornerRadius, Frame, Layout, Margin, Stroke, Vec2};

use crate::api::models::pick_image;
use crate::app::App;
use crate::model::{Action, Dialog};
use crate::settings::{AccentColor, ThemeChoice};
use crate::theme::{self, Icon, Palette};
use crate::tr;

use super::widgets;

const PLAYBACK_DIRTY_ID: &str = "playback-settings-dirty";
pub(crate) const PERSONAL_APP_FOCUS_ID: &str = "focus-personal-app-setup";

fn section(
    ui: &mut egui::Ui,
    palette: &Palette,
    title: &str,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    ui.add_space(10.0);
    theme::text(ui, title, theme::bold(18.0), palette.text);
    ui.add_space(8.0);
    Frame::new()
        .fill(
            palette
                .surface
                .gamma_multiply(if palette.dark { 0.7 } else { 1.0 }),
        )
        .stroke(Stroke::new(1.0, palette.outline))
        .corner_radius(CornerRadius::same(theme::RADIUS + 2))
        .inner_margin(Margin::symmetric(20, 16))
        .show(ui, |ui| {
            ui.set_width(ui.available_width().min(760.0));
            add_contents(ui);
        });
    ui.add_space(8.0);
}

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let locale = app.locale;
    ui.add_space(8.0);
    theme::text(ui, tr!(locale, "Settings"), theme::bold(28.0), palette.text);
    ui.add_space(4.0);
    let dirty_id = egui::Id::new(PLAYBACK_DIRTY_ID);
    let mut playback_dirty = ui
        .data(|data| data.get_temp::<bool>(dirty_id))
        .unwrap_or(false);
    let mut changed = false;

    section(ui, &palette, &tr!(locale, "Account"), |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 14.0;
            let avatar = app
                .user
                .as_ref()
                .and_then(|user| pick_image(&user.images, 64).map(str::to_string));
            widgets::cover(ui, &palette, avatar.as_deref(), 56.0, 28.0, Icon::User);
            ui.vertical(|ui| {
                let name = app
                    .user
                    .as_ref()
                    .map(|user| user.name().to_string())
                    .unwrap_or_default();
                theme::text(ui, name, theme::semibold(16.0), palette.text);
                let product = app
                    .user
                    .as_ref()
                    .and_then(|user| user.product.clone())
                    .map(|product| match product.as_str() {
                        "premium" => tr!(locale, "Spotify Premium").into_owned(),
                        "free" | "open" => {
                            tr!(locale, "Spotify Free, local playback needs Premium").into_owned()
                        }
                        other => other.to_string(),
                    })
                    .unwrap_or_default();
                theme::text(ui, product, theme::regular(13.0), palette.secondary);
                if let Some(username) = app.local.connected.then(|| app.local.username.clone())
                    && !username.is_empty()
                {
                    theme::text(
                        ui,
                        format!("Connected as {username}"),
                        theme::regular(12.0),
                        palette.dim,
                    );
                }
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if theme::pill_button(ui, &palette, &tr!(locale, "Sign out"), false).clicked() {
                    app.actions.push(Action::SignOut);
                }
            });
        });
        ui.add_space(10.0);
        let mut client_id = app.settings.web_client_id.clone().unwrap_or_default();
        widgets::setting_row(
            ui,
            &palette,
            &tr!(locale, "Personal Spotify app"),
            "Use a personal Development Mode app for a separate API quota. The shared app stays active.",
            |ui| {
                let response = Frame::new()
                    .fill(palette.surface)
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(Margin::symmetric(10, 6))
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut client_id)
                                .id(egui::Id::new("personal-web-client-id"))
                                .hint_text(egui::RichText::new(tr!(locale, "Client ID").into_owned()).color(palette.dim))
                                .font(theme::regular(13.0))
                                .frame(egui::Frame::NONE)
                                .desired_width(200.0),
                        )
                    })
                    .inner;
                if ui
                    .data_mut(|data| data.remove_temp::<bool>(egui::Id::new(PERSONAL_APP_FOCUS_ID)))
                    .unwrap_or(false)
                {
                    response.scroll_to_me(Some(Align::Center));
                    response.request_focus();
                }
                if response.changed() {
                    let trimmed = client_id.trim().to_string();
                    app.settings.web_client_id = (!trimmed.is_empty()).then_some(trimmed);
                    changed = true;
                }
            },
        );
        widgets::setting_row(
            ui,
            &palette,
            &tr!(locale, "Create an app"),
            &tr!(locale, "Create one for free in Spotify's developer dashboard."),
            |ui| {
                if theme::pill_button(ui, &palette, &tr!(locale, "Setup guide"), false).clicked() {
                    app.actions.push(Action::OpenUrl(
                        "https://fastpotify.rocks/make-it-even-faster/".into(),
                    ));
                }
            },
        );
        let wanted = app
            .settings
            .web_client_id
            .as_deref()
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .map(str::to_string);
        let in_use = wanted
            .as_deref()
            .is_some_and(|wanted| app.web_app.as_deref() == Some(wanted));
        if in_use {
            widgets::setting_row(
                ui,
                &palette,
                &tr!(locale, "Personal app ready"),
                &tr!(locale, "Supported requests use your app. Other requests use the shared app."),
                |ui| {
                    if theme::pill_button(ui, &palette, &tr!(locale, "Remove"), false).clicked() {
                        app.settings.web_client_id = None;
                        app.actions.push(Action::ConfigurePersonalWebApp);
                    }
                },
            );
        } else if wanted.is_some() {
            widgets::setting_row(
                ui,
                &palette,
                &tr!(locale, "Authorize your personal app"),
                &tr!(locale, "Spotify opens in your browser to verify the account."),
                |ui| {
                    if theme::pill_button(ui, &palette, &tr!(locale, "Authorize"), true).clicked() {
                        app.actions.push(Action::ConfigurePersonalWebApp);
                    }
                },
            );
        } else if app.web_app.is_some() {
            widgets::setting_row(
                ui,
                &palette,
                &tr!(locale, "Remove personal app"),
                &tr!(locale, "Shared access remains signed in."),
                |ui| {
                    if theme::pill_button(ui, &palette, &tr!(locale, "Remove"), false).clicked() {
                        app.actions.push(Action::ConfigurePersonalWebApp);
                    }
                },
            );
        }
    });

    section(ui, &palette, &tr!(locale, "Playback on this computer"), |ui| {
        let (status, detail, action) = match &app.local_playback {
            crate::backend::LocalPlayback::Ready { .. } => (
                tr!(locale, "Ready").into_owned(),
                tr!(locale, "This computer is a Spotify Connect device.").into_owned(),
                None,
            ),
            crate::backend::LocalPlayback::Authorizing => (
                tr!(locale, "Setting up").into_owned(),
                tr!(locale, "Finish authorizing in your browser.").into_owned(),
                None,
            ),
            crate::backend::LocalPlayback::Connecting => (
                tr!(locale, "Connecting").into_owned(),
                tr!(locale, "Connecting to Spotify…").into_owned(),
                None,
            ),
            crate::backend::LocalPlayback::Failed(message) => {
                (
                    tr!(locale, "Unavailable").into_owned(),
                    message.clone(),
                    Some(tr!(locale, "Try again").into_owned()),
                )
            }
            crate::backend::LocalPlayback::Unavailable => (
                tr!(locale, "Not set up").into_owned(),
                tr!(locale, "Requires Spotify Premium and a one-time browser sign-in.").into_owned(),
                Some(tr!(locale, "Enable playback here").into_owned()),
            ),
        };
        let status = tr!(locale, "Status: {status}").replace("{status}", &status);
        widgets::setting_row(ui, &palette, &status, &detail, |ui| {
            if let Some(label) = action {
                if theme::pill_button(ui, &palette, label.as_str(), true).clicked() {
                    app.actions.push(Action::EnablePlayback);
                }
            } else if app.local_ready
                && theme::soft_button(ui, &palette, Some(Icon::Refresh), &tr!(locale, "Reconnect"), false)
                    .clicked()
            {
                app.actions.push(Action::RestartEngine);
            }
        });
        widgets::setting_row(
            ui,
            &palette,
            &tr!(locale, "Device name"),
            &tr!(locale, "How this computer appears in Spotify Connect."),
            |ui| {
                let response = Frame::new()
                    .fill(palette.surface)
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(Margin::symmetric(10, 6))
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut app.settings.device_name)
                                .font(theme::regular(14.0))
                                .frame(egui::Frame::NONE)
                                .desired_width(200.0),
                        )
                    })
                    .inner;
                if response.changed() {
                    changed = true;
                    playback_dirty = true;
                }
            },
        );
        widgets::setting_row(
            ui,
            &palette,
            &tr!(locale, "Audio quality"),
            &tr!(locale, "Higher bitrates use more data and cache space."),
            |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    for (kbps, label) in [
                        (320u16, &tr!(locale, "Very high · 320 kbps")),
                        (160, &tr!(locale, "High · 160 kbps")),
                        (96, &tr!(locale, "Normal · 96 kbps")),
                    ] {
                        if theme::soft_button(
                            ui,
                            &palette,
                            None,
                            label,
                            app.settings.bitrate == kbps,
                        )
                        .clicked()
                            && app.settings.bitrate != kbps
                        {
                            app.settings.bitrate = kbps;
                            changed = true;
                            playback_dirty = true;
                        }
                    }
                });
            },
        );
        widgets::setting_row(
            ui,
            &palette,
            &tr!(locale, "Normalize volume"),
            &tr!(locale, "Keep loud and quiet tracks at a similar level."),
            |ui| {
                if widgets::switch(
                    ui,
                    &palette,
                    &tr!(locale, "Normalize volume"),
                    &mut app.settings.normalisation,
                )
                .changed()
                {
                    changed = true;
                    playback_dirty = true;
                }
            },
        );
        widgets::setting_row(
            ui,
            &palette,
            &tr!(locale, "Autoplay"),
            &tr!(locale, "Keep playing similar songs when your music ends."),
            |ui| {
                if widgets::switch(ui, &palette, &tr!(locale, "Autoplay"), &mut app.settings.autoplay).changed() {
                    changed = true;
                    playback_dirty = true;
                }
            },
        );
        widgets::setting_row(
            ui,
            &palette,
            &tr!(locale, "Gapless playback"),
            &tr!(locale, "Play tracks without silence between them."),
            |ui| {
                if widgets::switch(ui, &palette, &tr!(locale, "Gapless playback"), &mut app.settings.gapless)
                    .changed()
                {
                    changed = true;
                    playback_dirty = true;
                }
            },
        );
        widgets::setting_row(
            ui,
            &palette,
            &tr!(locale, "Keep music playing when the window closes"),
            super::keys::platform_shortcut(
                &tr!(locale, "Fastpotify hides to the system tray. Quit from the tray menu or with Ctrl+Q."),
                &tr!(locale, "Fastpotify hides to the system tray. Quit from the tray menu or with Cmd+Q."),
            ),
            |ui| {
                if widgets::switch(
                    ui,
                    &palette,
                    &tr!(locale, "Keep music playing when the window closes"),
                    &mut app.settings.keep_playing_in_background,
                )
                .changed()
                {
                    changed = true;
                }
            },
        );
        widgets::setting_row(
            ui,
            &palette,
            &tr!(locale, "Automatic update checks"),
            &tr!(locale, "Checks GitHub once a day. No personal data is sent."),
            |ui| {
                if widgets::switch(
                    ui,
                    &palette,
                    &tr!(locale, "Automatic update checks"),
                    &mut app.settings.check_for_updates,
                )
                .changed()
                {
                    changed = true;
                }
            },
        );
        if cfg!(target_os = "linux") {
            widgets::setting_row(
                ui,
                &palette,
                &tr!(locale, "Audio output"),
                &tr!(locale, "PulseAudio also covers PipeWire. Rodio talks to ALSA directly."),
                |ui| {
                    let current = app
                        .settings
                        .platform_backend()
                        .unwrap_or_else(|| "rodio".into());
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        for backend in ["rodio", "pulseaudio"] {
                            let label = if backend == "pulseaudio" {
                                &tr!(locale, "PulseAudio / PipeWire")
                            } else {
                                &tr!(locale, "ALSA (rodio)")
                            };
                            if theme::soft_button(ui, &palette, None, label, current == backend)
                                .clicked()
                                && current != backend
                            {
                                app.settings.audio_backend = Some(backend.to_string());
                                changed = true;
                                playback_dirty = true;
                            }
                        }
                    });
                },
            );
        }
        #[cfg(windows)]
        widgets::setting_row(
            ui,
            &palette,
            &tr!(locale, "Output buffer"),
            "More buffering can prevent clicks on busy computers. Less buffering makes controls respond sooner.",
            |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    let current = app.settings.audio_buffer_ms;
                    for ms in [50u32, 100, 200] {
                        let label = format!("{ms} ms");
                        if theme::soft_button(ui, &palette, None, &label, current == ms).clicked()
                            && current != ms
                        {
                            app.settings.audio_buffer_ms = ms;
                            changed = true;
                            playback_dirty = true;
                        }
                    }
                });
            },
        );
        widgets::setting_row(
            ui,
            &palette,
            &tr!(locale, "Audio cache"),
            &tr!(locale, "Save downloaded audio for later playback."),
            |ui| {
                // The control area lays out right-to-left: add the rightmost item first.
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    if widgets::switch(ui, &palette, &tr!(locale, "Audio cache"), &mut app.settings.audio_cache)
                        .changed()
                    {
                        changed = true;
                        playback_dirty = true;
                    }
                    if app.settings.audio_cache {
                        ui.add_space(6.0);
                        for (mb, label) in [(4096u64, "4 GB"), (1024, "1 GB"), (512, "512 MB")] {
                            if theme::soft_button(
                                ui,
                                &palette,
                                None,
                                label,
                                app.settings.audio_cache_mb == mb,
                            )
                            .clicked()
                                && app.settings.audio_cache_mb != mb
                            {
                                app.settings.audio_cache_mb = mb;
                                changed = true;
                                playback_dirty = true;
                            }
                        }
                    }
                });
            },
        );
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if playback_dirty {
                if theme::pill_button(ui, &palette, &tr!(locale, "Apply and restart playback"), true).clicked() {
                    app.actions.push(Action::RestartEngine);
                    playback_dirty = false;
                }
                theme::subtle(
                    ui,
                    &palette,
                    &tr!(locale, "Restart local playback to apply these settings."),
                );
            } else {
                theme::subtle(ui, &palette, &tr!(locale, "Playback settings applied."));
            }
        });
    });

    section(ui, &palette, &tr!(locale, "Appearance"), |ui| {
        widgets::setting_row(
            ui,
            &palette,
            &tr!(locale, "Language"),
            &tr!(locale, "Automatic follows your system's language."),
            |ui| {
                // Every language is written in itself, so a listener looking
                // at an interface they cannot read still finds their own.
                let system = crate::i18n::Locale::from_system();
                let automatic = format!("Automatic ({})", system.native_name());
                let chosen = app.settings.locale_choice();
                let label = chosen.map_or(automatic.as_str(), |locale| locale.native_name());
                egui::ComboBox::new("settings-language", "")
                    .selected_text(label)
                    .show_ui(ui, |ui| {
                        if ui.selectable_label(chosen.is_none(), &automatic).clicked()
                            && chosen.is_some()
                        {
                            app.set_language(None);
                            changed = true;
                        }
                        for locale in crate::i18n::LOCALES {
                            let selected = chosen == Some(*locale);
                            if ui
                                .selectable_label(selected, locale.native_name())
                                .clicked()
                                && !selected
                            {
                                app.set_language(Some(*locale));
                                changed = true;
                            }
                        }
                    });
            },
        );
        widgets::setting_row(ui, &palette, &tr!(locale, "Theme"), "", |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                for choice in ThemeChoice::ALL {
                    if theme::soft_button(
                        ui,
                        &palette,
                        None,
                        choice.label(),
                        app.settings.theme == choice,
                    )
                    .clicked()
                        && app.settings.theme != choice
                    {
                        app.settings.theme = choice;
                        changed = true;
                    }
                }
            });
        });
        widgets::setting_row(ui, &palette, &tr!(locale, "Accent color"), "", |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                for choice in AccentColor::ALL {
                    let mut button = theme::soft_button(
                        ui,
                        &palette,
                        None,
                        choice.label(),
                        app.settings.accent_color == choice,
                    );
                    if let Some(hint) = choice.hint() {
                        button = button.on_hover_text(hint);
                    }
                    if button.clicked() && app.settings.accent_color != choice {
                        app.settings.accent_color = choice;
                        changed = true;
                    }
                }
            });
        });
        widgets::setting_row(
            ui,
            &palette,
            &tr!(locale, "Colour from album art"),
            &tr!(locale, "Use the current cover's colour on pages and the player bar."),
            |ui| {
                if widgets::switch(
                    ui,
                    &palette,
                    &tr!(locale, "Colour from album art"),
                    &mut app.settings.accent_from_art,
                )
                .changed()
                {
                    changed = true;
                }
            },
        );
        widgets::setting_row(
            ui,
            &palette,
            &tr!(locale, "Compact library sidebar"),
            &tr!(locale, "Show names without covers in the sidebar."),
            |ui| {
                if widgets::switch(
                    ui,
                    &palette,
                    &tr!(locale, "Compact library sidebar"),
                    &mut app.settings.sidebar_compact,
                )
                .changed()
                {
                    changed = true;
                }
            },
        );
        widgets::setting_row(
            ui,
            &palette,
            &tr!(locale, "Compact track list"),
            &tr!(locale, "Show each track on one line without a cover."),
            |ui| {
                if widgets::switch(
                    ui,
                    &palette,
                    &tr!(locale, "Compact track list"),
                    &mut app.settings.tracklist_compact,
                )
                .changed()
                {
                    changed = true;
                }
            },
        );
        widgets::setting_row(
            ui,
            &palette,
            &tr!(locale, "Interface zoom"),
            super::keys::platform_shortcut(
                &tr!(locale, "Ctrl+Plus and Ctrl+Minus work anywhere; Ctrl+0 resets."),
                &tr!(locale, "Cmd+Plus and Cmd+Minus work anywhere; Cmd+0 resets."),
            ),
            |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    let mut zoom = app.settings.zoom;
                    if theme::soft_button(ui, &palette, None, "+", false).clicked() {
                        zoom = (zoom + 0.1).min(2.5);
                    }
                    theme::text(
                        ui,
                        format!("{:.0}%", zoom * 100.0),
                        theme::medium(13.5),
                        palette.text,
                    );
                    if theme::soft_button(ui, &palette, None, "-", false).clicked() {
                        zoom = (zoom - 0.1).max(0.5);
                    }
                    if (zoom - app.settings.zoom).abs() > 0.001 {
                        app.settings.zoom = zoom;
                        ui.ctx().set_zoom_factor(zoom);
                        app.mark_settings_dirty();
                    }
                });
            },
        );
    });

    section(ui, &palette, &tr!(locale, "Lyrics translation"), |ui| {
        widgets::setting_row(
            ui,
            &palette,
            &tr!(locale, "Translate lyrics"),
            "Via a LibreTranslate-compatible server. The official one needs an API key below (get one at libretranslate.com); a self-hosted server usually does not.",
            |ui| {
                if widgets::switch(
                    ui,
                    &palette,
                    &tr!(locale, "Translate lyrics"),
                    &mut app.settings.lyrics_translate_enabled,
                )
                .changed()
                {
                    changed = true;
                    if app.settings.lyrics_translate_enabled {
                        app.maybe_translate_lyrics();
                    }
                }
            },
        );
        if app.settings.lyrics_translate_enabled {
            widgets::setting_row(
                ui,
                &palette,
                &tr!(locale, "Language"),
                &tr!(locale, "Automatic follows your system's language."),
                |ui| {
                    let current = app.settings.lyrics_translate_language.clone();
                    // Language names are given in their own language, so only
                    // the "follow the system" line is translated.
                    let automatic = tr!(locale, "Automatic (system language)");
                    let current_label = current
                        .as_deref()
                        .and_then(|code| {
                            crate::lyrics::TRANSLATE_LANGUAGES
                                .iter()
                                .find(|(iso, _)| *iso == code)
                                .map(|(_, name)| *name)
                        })
                        .unwrap_or(&automatic);
                    egui::ComboBox::new("settings-lyrics-translate-language", "")
                        .selected_text(current_label)
                        .show_ui(ui, |ui| {
                            let mut picked = false;
                            if ui
                                .selectable_label(
                                    current.is_none(),
                                    tr!(locale, "Automatic (system language)").into_owned(),
                                )
                                .clicked()
                                && current.is_some()
                            {
                                app.settings.lyrics_translate_language = None;
                                picked = true;
                            }
                            for (code, name) in crate::lyrics::TRANSLATE_LANGUAGES {
                                let selected = current.as_deref() == Some(*code);
                                if ui.selectable_label(selected, *name).clicked() && !selected {
                                    app.settings.lyrics_translate_language =
                                        Some((*code).to_string());
                                    picked = true;
                                }
                            }
                            if picked {
                                changed = true;
                                app.maybe_translate_lyrics();
                            }
                        });
                },
            );
            widgets::setting_row(
                ui,
                &palette,
                &tr!(locale, "Translation server"),
                &tr!(locale, "Empty uses the official libretranslate.com, which requires the API key below."),
                |ui| {
                    let mut url = app
                        .settings
                        .lyrics_translate_api_url
                        .clone()
                        .unwrap_or_default();
                    let response = Frame::new()
                        .fill(palette.surface)
                        .corner_radius(CornerRadius::same(6))
                        .inner_margin(Margin::symmetric(10, 6))
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut url)
                                    .hint_text(
                                        egui::RichText::new("https://libretranslate.com/translate")
                                            .color(palette.dim),
                                    )
                                    .font(theme::regular(13.0))
                                    .frame(egui::Frame::NONE)
                                    .desired_width(260.0),
                            )
                        })
                        .inner;
                    if response.changed() {
                        let trimmed = url.trim().to_string();
                        app.settings.lyrics_translate_api_url =
                            (!trimmed.is_empty()).then_some(trimmed);
                        changed = true;
                    }
                },
            );
            widgets::setting_row(
                ui,
                &palette,
                &tr!(locale, "API key"),
                &tr!(locale, "Only needed for the official libretranslate.com server."),
                |ui| {
                    let mut key = app
                        .settings
                        .lyrics_translate_api_key
                        .clone()
                        .unwrap_or_default();
                    let response = Frame::new()
                        .fill(palette.surface)
                        .corner_radius(CornerRadius::same(6))
                        .inner_margin(Margin::symmetric(10, 6))
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut key)
                                    .password(true)
                                    .hint_text(egui::RichText::new(tr!(locale, "API key").into_owned()).color(palette.dim))
                                    .font(theme::regular(13.0))
                                    .frame(egui::Frame::NONE)
                                    .desired_width(200.0),
                            )
                        })
                        .inner;
                    if response.changed() {
                        let trimmed = key.trim().to_string();
                        app.settings.lyrics_translate_api_key =
                            (!trimmed.is_empty()).then_some(trimmed);
                        changed = true;
                    }
                },
            );
        }
    });

    section(ui, &palette, &tr!(locale, "Mini player"), |ui| {
        widgets::setting_row(
            ui,
            &palette,
            &tr!(locale, "Mini player"),
            &tr!(locale, "A small, resizable now-playing bar with cover, controls, and lyrics."),
            |ui| {
                if theme::pill_button(ui, &palette, &tr!(locale, "Switch to it"), false).clicked() {
                    app.actions.push(Action::ToggleMiniPlayer);
                }
            },
        );
        widgets::setting_row(
            ui,
            &palette,
            &tr!(locale, "Always on top"),
            &tr!(locale, "Keep the mini player above everything else."),
            |ui| {
                let mut on_top = app.settings.winamp_on_top;
                if widgets::switch(ui, &palette, &tr!(locale, "Always on top"), &mut on_top).changed() {
                    app.actions.push(Action::ToggleWinampOnTop);
                }
            },
        );
        if app.windows_controls_visible() {
            widgets::setting_row(
                ui,
                &palette,
                &tr!(locale, "Show in taskbar"),
                "Keep a taskbar button for the mini player. The tray icon stays available when hidden.",
                |ui| {
                    let mut visible = app.settings.winamp_show_taskbar;
                    let response =
                        widgets::switch(ui, &palette, &tr!(locale, "Show mini player in taskbar"), &mut visible);
                    if response.changed() {
                        app.actions.push(Action::SetWinampTaskbar(visible));
                    }
                    #[cfg(any(test, feature = "demo"))]
                    if app.demo_windows_controls {
                        let id = egui::Id::new("demo-mini-player-taskbar-focus");
                        if !ui.data(|data| data.get_temp::<bool>(id)).unwrap_or(false) {
                            response.scroll_to_me(Some(Align::Center));
                            ui.data_mut(|data| data.insert_temp(id, true));
                        }
                    }
                },
            );
        }
    });

    section(ui, &palette, &tr!(locale, "MilkDrop"), |ui| {
        widgets::setting_row(
            ui,
            &palette,
            &tr!(locale, "MilkDrop window"),
            super::keys::platform_shortcut(
                "A projectM visualiser for local playback. Open it here, from the top bar, with Ctrl+Shift+K, or from the mini player's V menu. Press ? or F1 for its shortcuts.",
                "A projectM visualiser for local playback. Open it here, from the top bar, with Cmd+Shift+K, or from the mini player's V menu. Press ? or F1 for its shortcuts.",
            ),
            |ui| {
                let mut open = app.settings.milkdrop_open;
                if widgets::switch(ui, &palette, &tr!(locale, "MilkDrop window"), &mut open).changed() {
                    app.actions.push(Action::ToggleWinampMilkdrop);
                }
            },
        );
        let folder = app.dirs.milkdrop_dir();
        app.winamp.presets.refresh(&folder);
        let count = app.winamp.presets.count();
        let downloading = app.winamp.presets.downloading();
        widgets::setting_row(
            ui,
            &palette,
            &tr!(locale, "Presets"),
            &format!(
                "{} in {}. Add .milk files here. Fastpotify downloads presets when MilkDrop first opens with an empty folder.",
                match count {
                    0 => tr!(locale, "None yet").into_owned(),
                    1 => tr!(locale, "One preset").into_owned(),
                    n => format!("{n} presets"),
                },
                folder.display(),
            ),
            |_ui| {},
        );
        // Three buttons are wider than a row's control slot; they get a
        // line of their own under the words.
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            for (index, pack) in crate::milkdrop::PACKS.iter().enumerate() {
                let label = match downloading {
                    Some(name) if name == pack.name => tr!(locale, "Fetching...").into_owned(),
                    _ => format!("Get {}", pack.name),
                };
                if theme::soft_button(ui, &palette, Some(Icon::Globe), &label, false)
                    .on_hover_text(pack.note)
                    .clicked()
                    && downloading.is_none()
                {
                    app.actions.push(Action::DownloadMilkdropPack(index));
                }
            }
            if theme::soft_button(ui, &palette, Some(Icon::ExternalLink), &tr!(locale, "Open folder"), false)
                .clicked()
            {
                app.actions.push(Action::OpenMilkdropFolder);
            }
        });
        ui.add_space(10.0);
        widgets::setting_row(
            ui,
            &palette,
            &tr!(locale, "Time per preset"),
            &tr!(locale, "How long each preset plays before the next fades in."),
            |ui| {
                let mut seconds = app.settings.milkdrop_seconds.clamp(2, 300);
                let slider = egui::Slider::new(&mut seconds, 2..=300)
                    .logarithmic(true)
                    .suffix(" s");
                if ui.add(slider).changed() {
                    app.actions.push(Action::SetMilkdropSeconds(seconds));
                }
            },
        );
        let screen_hz = app.settings.milkdrop_screen_hz;
        widgets::setting_row(
            ui,
            &palette,
            &tr!(locale, "Frame rate"),
            &match screen_hz {
                0 => tr!(locale, "Lower rates use fewer resources. Uncapped draws as fast as possible.")
                    .into_owned(),
                hz => format!(
                    "Your screen refreshes at {hz} Hz. Higher rates do not add visible frames. Uncapped draws as fast as possible."
                ),
            },
            |ui| {
                let fps = app.settings.milkdrop_fps;
                // The dial stops at the rates worth having and passes
                // through nothing in between, the way a gear lever does.
                let stops = crate::milkdrop::fps_stops(screen_hz, fps);
                let last = stops.len().saturating_sub(1);
                let mut at = stops.iter().position(|rate| *rate == fps).unwrap_or(1);
                let labels: Vec<String> = stops
                    .iter()
                    .map(|rate| crate::milkdrop::fps_label(*rate, screen_hz))
                    .collect();
                let shown = labels.clone();
                let typed = stops.clone();
                let slider = egui::Slider::new(&mut at, 0..=last)
                    .step_by(1.0)
                    .custom_formatter(move |value, _| {
                        shown
                            .get((value.round().max(0.0) as usize).min(shown.len() - 1))
                            .cloned()
                            .unwrap_or_default()
                    })
                    .custom_parser(move |text| {
                        // A rate typed in lands on the nearest stop, since
                        // the stops are all this dial can hold.
                        let text = text.trim().to_lowercase();
                        if text.starts_with("un") {
                            return Some(typed.len().saturating_sub(1) as f64);
                        }
                        let wanted: u32 = text
                            .trim_end_matches("fps")
                            .trim()
                            .split(',')
                            .next()?
                            .trim()
                            .parse()
                            .ok()?;
                        typed
                            .iter()
                            .enumerate()
                            .filter(|(_, rate)| **rate > 0)
                            .min_by_key(|(_, rate)| rate.abs_diff(wanted))
                            .map(|(index, _)| index as f64)
                    });
                if ui.add(slider).changed()
                    && let Some(rate) = stops.get(at)
                {
                    app.actions.push(Action::SetMilkdropFps(*rate));
                }
            },
        );
        widgets::setting_row(
            ui,
            &palette,
            &tr!(locale, "Resolution"),
            &tr!(locale, "Half and Quarter use fewer resources and scale the image back up."),
            |ui| {
                let current = app.settings.milkdrop_scale.max(1);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    for (scale, label) in [(1u32, &tr!(locale, "Full")), (2, &tr!(locale, "Half")), (4, &tr!(locale, "Quarter"))] {
                        if theme::soft_button(ui, &palette, None, label, scale == current).clicked()
                            && scale != current
                        {
                            app.actions.push(Action::SetMilkdropScale(scale));
                        }
                    }
                });
            },
        );
    });

    section(ui, &palette, &tr!(locale, "Equalizer"), |ui| {
        widgets::setting_row(
            ui,
            &palette,
            &tr!(locale, "Equalizer"),
            "A ten-band equalizer for playback on this computer. It does not affect other devices.",
            |ui| {
                let mut on = app.settings.eq_on;
                if widgets::switch(ui, &palette, &tr!(locale, "Equalizer"), &mut on).changed() {
                    app.actions.push(Action::ToggleEq);
                }
            },
        );
        let names: Vec<(usize, &str)> = crate::eq::PRESETS
            .iter()
            .enumerate()
            .map(|(index, preset)| (index, preset.name))
            .collect();
        let current = crate::eq::PRESETS
            .iter()
            .position(|preset| preset.bands_db == app.settings.eq_bands_db)
            .unwrap_or(usize::MAX);
        if let Some(picked) = widgets::chips(ui, &palette, &names, current) {
            app.actions.push(Action::ApplyEqPreset(picked));
        }
        ui.add_space(10.0);
        eq_curve(ui, &palette, &crate::app::eq_settings(&app.settings));
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 14.0;
            let on = app.settings.eq_on;
            let mut preamp = app.settings.eq_preamp_db;
            if eq_slider(ui, &palette, &tr!(locale, "Pre"), &mut preamp, on) {
                app.actions.push(Action::SetEqPreamp(preamp));
            }
            for (band, hz) in crate::eq::BANDS.iter().enumerate() {
                let mut gain = app.settings.eq_bands_db[band];
                if eq_slider(ui, &palette, &hertz(*hz), &mut gain, on) {
                    app.actions.push(Action::SetEqBand(band, gain));
                }
            }
        });
    });

    section(ui, &palette, &tr!(locale, "Storage"), |ui| {
        widgets::setting_row(
            ui,
            &palette,
            &tr!(locale, "Artwork cache"),
            &format!("Stored in {}", app.dirs.art_cache_dir().display()),
            |ui| {
                if theme::soft_button(ui, &palette, Some(Icon::Trash), &tr!(locale, "Clear artwork"), false)
                    .clicked()
                {
                    app.actions.push(Action::ClearArtCache);
                }
            },
        );
        widgets::setting_row(
            ui,
            &palette,
            &tr!(locale, "Audio cache"),
            &format!("Stored in {}", app.dirs.audio_cache_dir().display()),
            |_| {},
        );
        widgets::setting_row(
            ui,
            &palette,
            &tr!(locale, "Play history"),
            &format!(
                "Tracks played here are stored in {}. This file is never uploaded.",
                app.dirs.history_file().display()
            ),
            |ui| {
                if theme::soft_button(ui, &palette, Some(Icon::Trash), &tr!(locale, "Clear history"), false)
                    .clicked()
                {
                    app.actions.push(Action::ClearPlayHistory);
                }
            },
        );
        widgets::setting_row(
            ui,
            &palette,
            &tr!(locale, "Sign-in"),
            &tr!(locale, "Sign-ins are saved in the system credential store when available."),
            |_| {},
        );
    });

    section(ui, &palette, &tr!(locale, "About"), |ui| {
        ui.horizontal(|ui| {
            let (logo, _) = ui.allocate_exact_size(Vec2::splat(40.0), egui::Sense::hover());
            theme::logo(ui, logo.center(), 40.0, palette.accent, palette.on_accent);
            ui.vertical(|ui| {
                theme::text(
                    ui,
                    format!("Fastpotify {}", env!("CARGO_PKG_VERSION")),
                    theme::semibold(15.0),
                    palette.text,
                );
                theme::text(
                    ui,
                    tr!(locale, "Built with Rust, egui, and librespot. Not affiliated with Spotify."),
                    theme::regular(13.0),
                    palette.secondary,
                );
            });
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let check_label = if app.update_checking {
                &tr!(locale, "Checking…")
            } else {
                &tr!(locale, "Check for updates")
            };
            if theme::soft_button(ui, &palette, Some(Icon::Refresh), check_label, false).clicked()
                && !app.update_checking
            {
                app.actions.push(Action::CheckForUpdates);
            }
            if theme::soft_button(ui, &palette, Some(Icon::Info), &tr!(locale, "Keyboard shortcuts"), false)
                .clicked()
            {
                app.actions.push(Action::ShowDialog(Dialog::Shortcuts));
            }
            if theme::soft_button(ui, &palette, Some(Icon::ExternalLink), &tr!(locale, "Source code"), false)
                .clicked()
            {
                ui.ctx()
                    .open_url(egui::OpenUrl::new_tab(env!("CARGO_PKG_REPOSITORY")));
            }
        });
    });

    ui.data_mut(|data| data.insert_temp(dirty_id, playback_dirty));
    if changed {
        app.actions.push(Action::SettingsChanged);
    }
}

/// A band's frequency the short way: 60, 170, 1K, 16K.
fn hertz(hz: f32) -> String {
    if hz >= 1000.0 {
        format!("{}K", (hz / 1000.0).round() as u32)
    } else {
        format!("{}", hz.round() as u32)
    }
}

/// One vertical slider in the app's own style: the track filled from
/// 0 dB, the handle in the middle when flat, a double-click to put it
/// back there. Returns whether it moved.
fn eq_slider(ui: &mut egui::Ui, palette: &Palette, label: &str, value: &mut f32, on: bool) -> bool {
    use egui::{Rect, Stroke, pos2, vec2};
    let range = crate::eq::RANGE_DB;
    ui.vertical(|ui| {
        let (rect, response) =
            ui.allocate_exact_size(vec2(30.0, 118.0), egui::Sense::click_and_drag());
        let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
        let track = Rect::from_center_size(rect.center(), vec2(4.0, rect.height() - 20.0));
        let y_of = |db: f32| track.bottom() - (db + range) / (2.0 * range) * track.height();
        let mut changed = false;
        if response.double_clicked() {
            *value = 0.0;
            changed = true;
        } else if (response.dragged() || response.clicked())
            && let Some(pos) = response.interact_pointer_pos()
        {
            let db = (track.bottom() - pos.y) / track.height() * 2.0 * range - range;
            let db = (db.clamp(-range, range) * 10.0).round() / 10.0;
            if db != *value {
                *value = db;
                changed = true;
            }
        }
        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            painter.rect_filled(track, 2.0, palette.surface_active);
            let fill = if on { palette.accent } else { palette.dim };
            let (top, bottom) = (y_of(value.max(0.0)), y_of(value.min(0.0)));
            painter.rect_filled(
                Rect::from_min_max(pos2(track.left(), top), pos2(track.right(), bottom)),
                2.0,
                fill,
            );
            painter.hline(
                (track.left() - 3.0)..=(track.right() + 3.0),
                y_of(0.0),
                Stroke::new(1.0, palette.dim),
            );
            let handle = pos2(track.center().x, y_of(*value));
            painter.circle_filled(handle, 7.0, palette.text);
            if response.hovered() || response.dragged() {
                painter.text(
                    pos2(track.center().x, rect.top() + 2.0),
                    egui::Align2::CENTER_TOP,
                    format!("{value:+.1}"),
                    theme::regular(11.0),
                    palette.secondary,
                );
            }
        }
        theme::text(ui, label, theme::regular(11.5), palette.secondary);
        changed
    })
    .inner
}

/// The equalizer's response over the audible range, the bands marked on
/// it: the shape says what a row of numbers cannot.
fn eq_curve(ui: &mut egui::Ui, palette: &Palette, settings: &crate::eq::EqSettings) {
    use egui::{Shape, Stroke, pos2, vec2};
    let width = ui.available_width().min(720.0);
    let (rect, _) = ui.allocate_exact_size(vec2(width, 120.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, theme::RADIUS as f32, palette.surface);
    let plot = rect.shrink2(vec2(10.0, 12.0));
    let (low, high) = (20f32.log10(), 20_000f32.log10());
    let x_of = |hz: f32| plot.left() + (hz.log10() - low) / (high - low) * plot.width();
    let y_of = |db: f32| {
        plot.center().y
            - db.clamp(-crate::eq::RANGE_DB, crate::eq::RANGE_DB) / crate::eq::RANGE_DB
                * plot.height()
                / 2.0
    };
    for db in [-12.0, -6.0, 0.0, 6.0, 12.0] {
        let color = if db == 0.0 {
            palette.dim
        } else {
            palette.outline
        };
        painter.hline(plot.x_range(), y_of(db), Stroke::new(1.0, color));
    }
    for hz in crate::eq::BANDS {
        painter.vline(x_of(hz), plot.y_range(), Stroke::new(1.0, palette.outline));
    }
    let curve = settings.curve();
    let points: Vec<egui::Pos2> = (0..=240)
        .map(|step| {
            let t = step as f32 / 240.0;
            let hz = 10f32.powf(low + t * (high - low));
            pos2(plot.left() + t * plot.width(), y_of(curve.db_at(hz)))
        })
        .collect();
    let color = if settings.on {
        palette.accent
    } else {
        palette.dim
    };
    painter.add(Shape::line(points, Stroke::new(2.0, color)));
    for (hz, db) in crate::eq::BANDS.iter().zip(settings.bands_db) {
        painter.circle_filled(pos2(x_of(*hz), y_of(db + settings.preamp_db)), 3.0, color);
    }
}
