//! The compact bar's own playlist section: the real, live playback queue
//! (`src/ui/queue.rs`) inside an expanding section of the mini player,
//! under a classic Winamp-style button row.
//!
//! `src/ui/winamp/playlist.rs` is a pixel-sprite renderer tied to the retro
//! skin's bitmap coordinates and is not reused here; only the *meaning* of
//! its ADD/REM/SEL/MISC/LIST/OPT labels is kept, each wired to whatever real
//! action `queue.rs` and `App` actually expose. Honest breakdown of what
//! each button does, since half of them have no real backing feature here:
//!
//! - ADD: real. Re-queues the currently playing track via the same
//!   `Action::AddToQueue` a track's own context menu uses. There is no
//!   track browser inside this small section to add an arbitrary song from,
//!   so the current track is the only sensible target in this context.
//! - REM: real, but coarser than classic Winamp's per-row delete. `queue.rs`
//!   has no concept of a selected row at all, so this clears every manually
//!   queued row via the same `Action::ClearQueue` the queue page's own
//!   trash button uses (disabled, with a tooltip, when there is nothing to
//!   clear).
//! - LIST: real. Switches the list between the live Queue and Recently
//!   played tabs via the same `Action::SetQueueTab` the queue side panel's
//!   chips use -- a repurposing of Winamp's "playlist list options" toward
//!   the closest real equivalent this app has.
//! - SEL, MISC, OPT: decorative placeholders. Winamp used these to open
//!   selection and options context menus this app has no equivalent of;
//!   they render as disabled buttons with a tooltip saying so rather than
//!   fabricating a selection system or an options menu with nothing behind
//!   it.

use egui::{Align, Layout};

use crate::app::App;
use crate::model::{Action, QueueTab};
use crate::theme::{self, Icon};

/// Caps how tall the reused queue contents can grow inside the mini player,
/// so one very long queue cannot pull the whole compact window off-screen.
const LIST_MAX_HEIGHT: f32 = 260.0;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        theme::text(ui, "Playlist", theme::regular(12.5), palette.secondary);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if theme::icon_button(
                ui,
                Icon::X,
                14.0,
                palette.secondary,
                palette.text,
                "Close playlist",
            )
            .clicked()
            {
                app.actions.push(Action::TogglePlaylistWindow);
            }
        });
    });
    ui.add_space(4.0);

    // Lazy load recents when the tab becomes visible, same as the side panel.
    if app.queue_tab == QueueTab::Recents
        && !app.recents.loading
        && !app.recents.complete
        && app.recents.items.is_empty()
        && app.recents.error.is_none()
    {
        app.actions.push(Action::LoadMoreRecents);
    }
    egui::ScrollArea::vertical()
        .id_salt("compact-playlist-scroll")
        .max_height(LIST_MAX_HEIGHT)
        .auto_shrink([false, true])
        .show(ui, |ui| match app.queue_tab {
            QueueTab::Queue => super::queue::contents(app, ui, true),
            QueueTab::Recents => super::queue::recents_contents(app, ui),
        });

    ui.add_space(4.0);
    button_row(app, ui);
    ui.add_space(4.0);
}

fn button_row(app: &mut App, ui: &mut egui::Ui) {
    // Read everything the two closures need from `app` up front: `Sides`
    // hands out two independent closures at once, and a method call like
    // `now_playing_item()` (unlike a plain field read) borrows all of
    // `*app`, so neither closure can hold `app` itself while the other
    // runs. Both just report what was clicked; `app` is mutated afterwards.
    let current_track = app.now_playing_item();
    let can_remove = app.can_clear_queue();
    let queue_tab = app.queue_tab;

    let mut add_clicked = false;
    let mut remove_clicked = false;
    let mut list_clicked = false;
    egui::Sides::new().show(
        ui,
        |ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            // ADD: real -- re-queues the current track (see module docs).
            add_clicked = ui
                .add_enabled(current_track.is_some(), egui::Button::new("ADD"))
                .on_hover_text("Add the current track to the queue")
                .on_disabled_hover_text("Nothing is playing to add")
                .clicked();
            // REM: real -- clears the manually queued rows (see module docs).
            remove_clicked = ui
                .add_enabled(can_remove, egui::Button::new("REM"))
                .on_hover_text("Clear the songs you queued manually")
                .on_disabled_hover_text("Nothing manually queued to remove")
                .clicked();
            // SEL, MISC: decorative -- no selection model or options menu exists.
            ui.add_enabled(false, egui::Button::new("SEL"))
                .on_disabled_hover_text("No row selection exists to act on (decorative)");
            ui.add_enabled(false, egui::Button::new("MISC"))
                .on_disabled_hover_text("No misc menu exists here (decorative)");
        },
        |ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            // OPT: decorative -- no options menu exists.
            ui.add_enabled(false, egui::Button::new("OPT"))
                .on_disabled_hover_text("No options menu exists here (decorative)");
            // LIST: real -- switches the live Queue/Recents tab.
            let label = match queue_tab {
                QueueTab::Queue => "LIST: QUEUE",
                QueueTab::Recents => "LIST: RECENT",
            };
            list_clicked = ui
                .button(label)
                .on_hover_text("Switch between the queue and recently played")
                .clicked();
        },
    );

    if add_clicked && let Some(item) = current_track {
        app.actions.push(Action::AddToQueue {
            uri: item.uri().to_string(),
            label: item.name().to_string(),
        });
    }
    if remove_clicked {
        app.actions.push(Action::ClearQueue);
    }
    if list_clicked {
        let next = match queue_tab {
            QueueTab::Queue => QueueTab::Recents,
            QueueTab::Recents => QueueTab::Queue,
        };
        app.actions.push(Action::SetQueueTab(next));
    }
}
