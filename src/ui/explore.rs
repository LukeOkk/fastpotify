//! Browse categories: the colour-block grid and one category's playlists.

use egui::{Align, Color32, CornerRadius, Rect, Sense, Stroke, UiBuilder, Vec2, pos2, vec2};

use crate::api::models::{Category, pick_image};
use crate::app::App;
use crate::model::{Action, Loadable, Page};
use crate::theme::{self, Icon};

use super::widgets;

/// The narrowest a card may be before the grid drops a column.
const MIN_CARD_WIDTH: f32 = 180.0;
/// Card height as a fraction of its width: wider than tall, like Spotify's.
const CARD_ASPECT: f32 = 0.60;
/// How far the corner cover leans, in radians.
const COVER_TILT: f32 = 0.44;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    ui.add_space(8.0);
    theme::text(ui, "Explore", theme::bold(28.0), palette.text);
    ui.add_space(14.0);
    match &app.explore {
        Loadable::NotLoaded | Loadable::Loading => widgets::loading_row(ui, &palette),
        Loadable::Failed(message) => {
            let message = message.clone();
            widgets::error_row(ui, app, &message, Some(Page::Explore));
        }
        Loadable::Loaded(categories) if categories.is_empty() => widgets::empty_state(
            ui,
            &palette,
            Icon::Compass,
            "Nothing to browse",
            "Spotify didn't return any categories for this account.",
        ),
        Loadable::Loaded(categories) => {
            let categories = categories.clone();
            grid(app, ui, &categories);
        }
    }
}

pub fn category(app: &mut App, ui: &mut egui::Ui, id: &str) {
    let palette = app.palette;
    let (name, retired) = app
        .category_pages
        .get(id)
        .map(|page| (page.name.clone(), page.retired))
        .unwrap_or_default();
    let heading = if name.is_empty() { id } else { &name };
    ui.add_space(8.0);
    theme::text(ui, heading, theme::bold(28.0), palette.text);
    if retired {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            theme::icon(ui, Icon::Info, 15.0, palette.secondary);
            ui.add_space(2.0);
            theme::text(
                ui,
                format!(
                    "Spotify retired this category's playlist feed in November 2024, so these are search results for \u{201c}{heading}\u{201d}."
                ),
                theme::regular(13.0),
                palette.secondary,
            );
        });
    }
    ui.add_space(14.0);
    let state = app.category_pages.get(id);
    match state.map(|page| &page.playlists) {
        None | Some(Loadable::NotLoaded) | Some(Loadable::Loading) => {
            widgets::loading_row(ui, &palette)
        }
        Some(Loadable::Failed(message)) => {
            let message = message.clone();
            widgets::error_row(ui, app, &message, Some(Page::Category(id.to_string())));
        }
        Some(Loadable::Loaded(playlists)) if playlists.is_empty() => widgets::empty_state(
            ui,
            &palette,
            Icon::ListMusic,
            "No playlists here",
            "Spotify has nothing to show for this category right now.",
        ),
        Some(Loadable::Loaded(playlists)) => {
            let playlists = playlists.clone();
            let card_height = widgets::card_row_height(ui);
            widgets::virtual_wrapped_cards(ui, playlists.len(), card_height, |ui, index| {
                let playlist = &playlists[index];
                let subtitle = playlist
                    .description
                    .as_deref()
                    .map(crate::util::strip_html)
                    .filter(|description| !description.is_empty())
                    .unwrap_or_else(|| format!("By {}", playlist.owner_name()));
                let card = widgets::card(
                    ui,
                    app,
                    pick_image(&playlist.images, 300),
                    &playlist.name,
                    &subtitle,
                    false,
                    true,
                );
                if card.play {
                    app.actions.push(Action::PlayContext {
                        uri: playlist.uri.clone(),
                        offset_uri: None,
                        offset_index: None,
                    });
                }
                if card.clicked {
                    app.actions
                        .push(Action::Open(Page::Playlist(playlist.id.clone())));
                }
                egui::Popup::context_menu(&card.response)
                    .id(ui.make_persistent_id(("category-playlist-menu", &playlist.uri)))
                    .frame(widgets::menu_frame(&palette))
                    .show(|ui| {
                        let owned = app.user_id().is_some_and(|id| playlist.owned_by(id));
                        widgets::context_menu_items(
                            ui,
                            app,
                            &playlist.uri,
                            &playlist.name,
                            owned.then_some(playlist),
                        );
                    });
            });
        }
    }
}

fn grid(app: &mut App, ui: &mut egui::Ui, categories: &[Category]) {
    let gap = widgets::CARD_GAP;
    let available = ui.available_width().max(MIN_CARD_WIDTH);
    let columns = ((available + gap) / (MIN_CARD_WIDTH + gap))
        .floor()
        .max(1.0) as usize;
    let card_width = (available - gap * (columns as f32 - 1.0)) / columns as f32;
    let card_height = (card_width * CARD_ASPECT).clamp(96.0, 150.0);
    let rows = categories.len().div_ceil(columns);

    // The row height below already carries the gap, so the enclosing layout
    // must not add its own on top or the virtual rows drift out of place.
    let previous_spacing = ui.spacing().item_spacing;
    ui.spacing_mut().item_spacing.y = 0.0;
    let grid_id = ui.unique_id().with("explore-grid");
    widgets::virtual_rows(ui, rows, card_height + gap, |ui, row| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(gap, gap);
            let start = row * columns;
            let end = (start + columns).min(categories.len());
            for category in &categories[start..end] {
                ui.scope_builder(UiBuilder::new().id(grid_id.with(&category.id)), |ui| {
                    let (rect, response) =
                        ui.allocate_exact_size(vec2(card_width, card_height), Sense::click());
                    let response = card(ui, app, category, rect, response);
                    if response.clicked() {
                        app.actions.push(Action::OpenCategory {
                            id: category.id.clone(),
                            name: category.name.clone(),
                        });
                    }
                });
            }
        });
        ui.allocate_space(vec2(available, gap));
    });
    ui.spacing_mut().item_spacing = previous_spacing;
}

fn card(
    ui: &mut egui::Ui,
    app: &mut App,
    category: &Category,
    rect: Rect,
    response: egui::Response,
) -> egui::Response {
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), &category.name)
    });
    if response.gained_focus() {
        response.scroll_to_me(None);
    }
    if ui.is_rect_visible(rect) {
        let base = card_color(&category.id);
        let fill = if ui.rect_contains_pointer(rect) {
            super::blend(base, Color32::WHITE, 0.12)
        } else {
            base
        };
        ui.painter()
            .rect_filled(rect, CornerRadius::same(theme::RADIUS), fill);

        let label = label_color(base);
        // The cover only reaches this far up the card, so the name can run
        // wider than the gap it leaves at the bottom.
        let text_width = (rect.width() * 0.76).max(40.0);
        let galley =
            widgets::ellipsized(ui, &category.name, theme::bold(16.0), label, text_width, 2);
        let text_rect =
            Rect::from_min_size(rect.min + vec2(16.0, 13.0), vec2(text_width, rect.height()));
        let position = match galley.job.halign {
            Align::RIGHT => pos2(text_rect.right(), text_rect.top()),
            Align::Center => pos2(text_rect.center().x, text_rect.top()),
            _ => text_rect.min,
        };
        ui.painter().galley(position, galley, label);

        corner_cover(ui, app, category, rect, base);
    }
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    theme::focus_ring(ui, &response);
    response
}

/// The category's own art, tilted into the bottom-right corner and clipped to
/// the card, the way Spotify's browse tiles are drawn.
fn corner_cover(ui: &mut egui::Ui, app: &mut App, category: &Category, rect: Rect, fill: Color32) {
    // Big enough that two corners run off the card and get clipped: that
    // overhang is what makes the tile read as a tilted cover rather than a
    // sticker sitting in the corner.
    let side = rect.height() * 0.78;
    let center = pos2(rect.right() - side * 0.26, rect.bottom() - side * 0.18);
    let cover = Rect::from_center_size(center, Vec2::splat(side));

    let mut clipped = ui.new_child(UiBuilder::new().max_rect(rect));
    clipped.set_clip_rect(rect.intersect(ui.clip_rect()));
    clipped.painter().add(egui::Shape::convex_polygon(
        tilted_corners(cover, COVER_TILT, vec2(3.0, 5.0)),
        Color32::from_black_alpha(70),
        Stroke::NONE,
    ));

    let url = pick_image(&category.icons, 300);
    let drawn = url.is_some_and(|url| {
        let art = app.backend.art();
        art.touch(url);
        let image = egui::Image::new(url).show_loading_spinner(false);
        // A texture that is merely Pending is not an error, and painting it
        // draws nothing: only Ready counts as covered.
        let Ok(egui::load::TexturePoll::Ready { texture }) =
            image.load_for_size(clipped.ctx(), cover.size())
        else {
            return false;
        };
        art.release_bytes(url);
        art.note_decoded(
            url,
            texture.size.x.round() as usize,
            texture.size.y.round() as usize,
        );
        egui::Image::new(texture)
            .rotate(COVER_TILT, Vec2::splat(0.5))
            .paint_at(&clipped, cover);
        true
    });
    if !drawn {
        // Never leave a hole while the art downloads: a darker facet of the
        // card's own colour reads as a cover from across the room.
        clipped.painter().add(egui::Shape::convex_polygon(
            tilted_corners(cover, COVER_TILT, Vec2::ZERO),
            super::blend(fill, Color32::BLACK, 0.32),
            Stroke::NONE,
        ));
    }
}

fn tilted_corners(rect: Rect, angle: f32, offset: Vec2) -> Vec<egui::Pos2> {
    let (sin, cos) = angle.sin_cos();
    let center = rect.center() + offset;
    let half = rect.width() / 2.0;
    [(-half, -half), (half, -half), (half, half), (-half, half)]
        .into_iter()
        .map(|(x, y)| pos2(center.x + x * cos - y * sin, center.y + x * sin + y * cos))
        .collect()
}

/// Spotify gives each browse category a brand colour its Web API does not
/// expose, so the card colour comes from the category id instead: the same
/// category is always the same colour, and a category this build has never
/// heard of still gets one rather than falling off a lookup table.
fn card_color(id: &str) -> Color32 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in id.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    // Saturation and value come from their own bytes so neighbouring cards
    // differ in weight as well as hue and the grid does not read as a rainbow.
    let hue = (hash % 360) as f32 / 360.0;
    let saturation = 0.72 + ((hash >> 13) % 3) as f32 * 0.10;
    let value = 0.50 + ((hash >> 29) % 4) as f32 * 0.12;
    Color32::from(egui::ecolor::HsvaGamma {
        h: hue,
        s: saturation,
        v: value,
        a: 1.0,
    })
}

/// Black or white, whichever the card's fill can carry.
fn label_color(fill: Color32) -> Color32 {
    if relative_luminance(fill) < 0.179 {
        Color32::WHITE
    } else {
        Color32::BLACK
    }
}

fn relative_luminance(color: Color32) -> f32 {
    let channel = |value: u8| {
        let value = f32::from(value) / 255.0;
        if value <= 0.040_45 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(color.r()) + 0.7152 * channel(color.g()) + 0.0722 * channel(color.b())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contrast(a: Color32, b: Color32) -> f32 {
        let (a, b) = (relative_luminance(a), relative_luminance(b));
        let (light, dark) = if a > b { (a, b) } else { (b, a) };
        (light + 0.05) / (dark + 0.05)
    }

    const IDS: &[&str] = &[
        "music",
        "podcasts",
        "live-events",
        "fitness",
        "made-for-you",
        "new-releases",
        "sessions",
        "latin",
        "pop",
        "cumbia",
        "nature",
        "funk",
        "singles",
        "summer",
        "country",
        "fresh-finds",
        "wellness",
        "trendsetters",
        "mixed-by",
        // Ids Spotify actually serves, which no hardcoded table would cover.
        "0JQ5DAqbMKFz6FAsUtgAab",
        "0JQ5DAudkNjCgYMM0TZXDw",
    ];

    #[test]
    fn a_category_keeps_its_colour() {
        for id in IDS {
            assert_eq!(card_color(id), card_color(id));
        }
        assert_ne!(card_color("pop"), card_color("funk"));
    }

    #[test]
    fn every_category_name_stays_legible_on_its_card() {
        for id in IDS {
            let fill = card_color(id);
            let ratio = contrast(fill, label_color(fill));
            assert!(ratio >= 4.5, "{id} reads at {ratio:.2}:1 on {fill:?}");
        }
    }

    #[test]
    fn colours_spread_across_the_hue_circle() {
        let hues: std::collections::HashSet<u32> = IDS
            .iter()
            .map(|id| {
                let hsva = egui::ecolor::HsvaGamma::from(card_color(id));
                (hsva.h * 12.0) as u32
            })
            .collect();
        // A hash that clumped every category into one corner of the wheel
        // would still be stable, and would still look broken.
        assert!(hues.len() >= 8, "only {} distinct hue bands", hues.len());
    }

    #[test]
    fn the_corner_cover_leans() {
        let square = Rect::from_min_size(pos2(0.0, 0.0), Vec2::splat(10.0));
        let upright = tilted_corners(square, 0.0, Vec2::ZERO);
        let tilted = tilted_corners(square, COVER_TILT, Vec2::ZERO);

        assert_eq!(upright.len(), 4);
        assert!(upright[0].y == upright[1].y);
        assert!(tilted[0].y != tilted[1].y);
    }
}
