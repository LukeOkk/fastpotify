//! Bundled gettext catalogs. English is the source language and the fallback
//! for every message a catalog has not translated yet.
//!
//! Marking a string for translation is [`crate::tr!`]:
//!
//! ```ignore
//! let locale = app.locale; // Copy, so it never borrows `app`
//! theme::text(ui, tr!(locale, "Settings"), theme::bold(28.0), palette.text);
//! ```

use std::borrow::Cow;
use std::sync::OnceLock;
use tr::Translator;

include!(concat!(env!("OUT_DIR"), "/catalogs.rs"));

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Locale {
    #[default]
    #[value(name = "en", alias = "en-US")]
    English,
    #[value(name = "de-DE", alias = "de")]
    German,
    #[value(name = "es")]
    Spanish,
    #[value(name = "nl")]
    Dutch,
    #[value(name = "pt-BR")]
    PortugueseBrazil,
    #[value(name = "pt-PT")]
    PortuguesePortugal,
    #[value(name = "fr")]
    French,
    #[value(name = "sv")]
    Swedish,
    #[value(name = "pl")]
    Polish,
    #[value(name = "ru")]
    Russian,
    #[value(name = "it")]
    Italian,
    #[value(name = "ja")]
    Japanese,
    #[value(name = "zh-Hans")]
    ChineseSimplified,
    #[value(name = "zh-Hant")]
    ChineseTraditional,
}

impl Locale {
    fn translator(self) -> Option<&'static dyn Translator> {
        match self {
            Self::English => None,
            Self::German => Some(&de_de::Translator),
            Self::Spanish => Some(&es::Translator),
            Self::Dutch => Some(&nl::Translator),
            Self::PortugueseBrazil => Some(&pt_br::Translator),
            Self::PortuguesePortugal => Some(&pt_pt::Translator),
            Self::French => Some(&fr::Translator),
            Self::Swedish => Some(&sv::Translator),
            Self::Polish => Some(&pl::Translator),
            Self::Russian => Some(&ru::Translator),
            Self::Italian => Some(&it::Translator),
            Self::Japanese => Some(&ja::Translator),
            Self::ChineseSimplified => Some(&zh_hans::Translator),
            Self::ChineseTraditional => Some(&zh_hant::Translator),
        }
    }

    pub fn liked_song_count(self, count: u32) -> String {
        ngettext(
            self,
            // Translators: Keep {count} exactly as written. It becomes the number of liked songs.
            "Playlist • {count} song",
            "Playlist • {count} songs",
            count,
        )
        .replace("{count}", &count.to_string())
    }

    /// The language tag this locale is named by on the command line and in
    /// `settings.json`. [`Self::from_language_tag`] reads every one of them
    /// back, so a stored choice survives a round trip.
    pub fn tag(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::German => "de-DE",
            Self::Spanish => "es",
            Self::Dutch => "nl",
            Self::PortugueseBrazil => "pt-BR",
            Self::PortuguesePortugal => "pt-PT",
            Self::French => "fr",
            Self::Swedish => "sv",
            Self::Polish => "pl",
            Self::Russian => "ru",
            Self::Italian => "it",
            Self::Japanese => "ja",
            Self::ChineseSimplified => "zh-Hans",
            Self::ChineseTraditional => "zh-Hant",
        }
    }

    /// The language's name in that language, for the picker. A listener who
    /// has the app in a language they cannot read must still be able to find
    /// their own, so these are never translated and never marked with
    /// [`crate::tr!`].
    /// The locale Spotify's catalogue wants, so browse categories come back
    /// named in the listener's language. Its `locale` parameter takes a
    /// language and a region joined by an underscore and ignores a bare
    /// language, so each one carries the region its catalogue is keyed by.
    pub fn spotify_locale(self) -> &'static str {
        match self {
            Self::English => "en_US",
            Self::German => "de_DE",
            Self::Spanish => "es_ES",
            Self::Dutch => "nl_NL",
            Self::PortugueseBrazil => "pt_BR",
            Self::PortuguesePortugal => "pt_PT",
            Self::French => "fr_FR",
            Self::Swedish => "sv_SE",
            Self::Polish => "pl_PL",
            Self::Russian => "ru_RU",
            Self::Italian => "it_IT",
            Self::Japanese => "ja_JP",
            Self::ChineseSimplified => "zh_CN",
            Self::ChineseTraditional => "zh_TW",
        }
    }

    pub fn native_name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::German => "Deutsch",
            Self::Spanish => "Español",
            Self::Dutch => "Nederlands",
            Self::PortugueseBrazil => "Português (Brasil)",
            Self::PortuguesePortugal => "Português (Portugal)",
            Self::French => "Français",
            Self::Swedish => "Svenska",
            Self::Polish => "Polski",
            Self::Russian => "Русский",
            Self::Italian => "Italiano",
            Self::Japanese => "日本語",
            Self::ChineseSimplified => "简体中文",
            Self::ChineseTraditional => "繁體中文",
        }
    }

    /// The language the desktop is read in, or English when it reads one no
    /// catalog covers.
    ///
    /// `sys_locale` asks each platform the way that platform answers --
    /// `CFLocaleCopyPreferredLanguages` on macOS, `GetUserPreferredUILanguages`
    /// on Windows, and the `LANGUAGE`/`LC_ALL`/`LC_MESSAGES`/`LANG` chain
    /// elsewhere -- which is why this has no platform code of its own.
    /// [`crate::lyrics::system_language`] asks the same crate but keeps only
    /// the two-letter language, which cannot tell Brazilian Portuguese from
    /// European, or Simplified Chinese from Traditional.
    pub fn from_system() -> Self {
        // The suite asserts the English interface, so it must mean the same
        // on a translator's Spanish desktop as on CI. A test that is about
        // another language sets `Settings::language` and says so.
        if cfg!(test) {
            return Self::English;
        }
        // A desktop does not change language while the app is running, and on
        // macOS the answer costs a Core Foundation round trip.
        static DETECTED: OnceLock<Locale> = OnceLock::new();
        *DETECTED.get_or_init(|| {
            // In preference order, so a desktop that reads Spanish first and
            // English second gets Spanish, while one that reads Norwegian
            // first and English second gets English rather than the default.
            sys_locale::get_locales()
                .find_map(|tag| Self::from_language_tag(&tag))
                .unwrap_or_default()
        })
    }

    /// The catalog closest to a language tag, BCP 47 (`"pt-BR"`,
    /// `"zh-Hant-TW"`) or POSIX (`"es_UY.UTF-8"`), or `None` when no catalog
    /// speaks it.
    pub fn from_language_tag(tag: &str) -> Option<Self> {
        let tag = tag.to_ascii_lowercase();
        // A POSIX name carries the encoding and a modifier the language is
        // not in: es_UY.UTF-8, sr_RS@latin.
        let tag = tag.split(['.', '@']).next()?;
        let mut subtags = tag.split(['-', '_']).filter(|part| !part.is_empty());
        let language = subtags.next()?;
        let mut script = None;
        let mut region = None;
        for subtag in subtags {
            match subtag.len() {
                4 => script = script.or(Some(subtag)),
                // A region is two letters (BR) or three digits (419);
                // Windows also writes the legacy Chinese scripts as CHS/CHT.
                2 | 3 => region = region.or(Some(subtag)),
                _ => {}
            }
        }
        Some(match language {
            "en" => Self::English,
            "de" => Self::German,
            "es" => Self::Spanish,
            "nl" => Self::Dutch,
            "fr" => Self::French,
            "sv" => Self::Swedish,
            "pl" => Self::Polish,
            "ru" => Self::Russian,
            "it" => Self::Italian,
            "ja" => Self::Japanese,
            // Every Portuguese-speaking country outside Brazil follows the
            // European standard, and so does a bare "pt" on a European
            // desktop -- but Brazil has more Portuguese speakers than all of
            // them together, so it takes the ambiguous tag.
            "pt" => match region {
                Some("pt" | "ao" | "cv" | "gw" | "mo" | "mz" | "st" | "tl") => {
                    Self::PortuguesePortugal
                }
                _ => Self::PortugueseBrazil,
            },
            // The script decides when it is written down; otherwise the
            // region does, and mainland China's Simplified is the default.
            "zh" => match (script, region) {
                (Some("hant"), _) | (_, Some("tw" | "hk" | "mo" | "cht")) => {
                    Self::ChineseTraditional
                }
                _ => Self::ChineseSimplified,
            },
            _ => return None,
        })
    }
}

/// Every bundled language, in the order the picker lists them: by their own
/// names, as a reader of each would sort them.
pub const LOCALES: &[Locale] = &[
    Locale::German,
    Locale::English,
    Locale::Spanish,
    Locale::French,
    Locale::Italian,
    Locale::Dutch,
    Locale::Polish,
    Locale::PortugueseBrazil,
    Locale::PortuguesePortugal,
    Locale::Swedish,
    Locale::Russian,
    Locale::Japanese,
    Locale::ChineseSimplified,
    Locale::ChineseTraditional,
];

/// Marks a user-facing string for translation: `tr!(locale, "Search")`.
///
/// The source must be a literal, because the catalogs are keyed by the
/// English text at extraction time -- build an interpolated string by
/// translating a `{placeholder}` form and calling `.replace()` on the
/// result, the way [`Locale::liked_song_count`] does. A counted phrase is
/// [`crate::trn!`], not this.
///
/// `xgettext` finds these by name (see
/// `.github/scripts/update-translations.sh`), so the macro must be called
/// `tr!`, not aliased or wrapped.
#[macro_export]
macro_rules! tr {
    ($locale:expr, $source:literal $(,)?) => {
        $crate::i18n::gettext($locale, $source)
    };
}

/// Marks a counted phrase: `trn!(locale, "{count} song", "{count} songs", n)`.
///
/// Both forms are whole sentences, so a language that counts differently
/// than English can rewrite them; the catalog's own plural rules choose.
#[macro_export]
macro_rules! trn {
    ($locale:expr, $singular:literal, $plural:literal, $count:expr $(,)?) => {
        $crate::i18n::ngettext($locale, $singular, $plural, $count)
    };
}

/// The English source is also the fallback for untranslated messages.
pub fn gettext(locale: Locale, source: &'static str) -> Cow<'static, str> {
    locale
        .translator()
        .map_or(Cow::Borrowed(source), |catalog| {
            catalog.translate(source, None)
        })
}

/// Select a whole translated phrase using the catalog's gettext plural rules.
pub fn ngettext(
    locale: Locale,
    singular: &'static str,
    plural: &'static str,
    count: u32,
) -> Cow<'static, str> {
    locale.translator().map_or(
        Cow::Borrowed(if count == 1 { singular } else { plural }),
        |catalog| catalog.ntranslate(count.into(), singular, plural, None),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_locale_asks_spotify_in_a_language_and_a_region() {
        // A bare language is ignored by the catalogue, so every mapping has to
        // carry a region or the grid silently comes back in English.
        for &locale in LOCALES {
            let tag = locale.spotify_locale();
            let (language, region) = tag
                .split_once('_')
                .unwrap_or_else(|| panic!("{tag:?} is missing its region"));
            assert_eq!(language.len(), 2, "{tag:?} language is not ISO 639-1");
            assert_eq!(region.len(), 2, "{tag:?} region is not ISO 3166-1");
            assert!(
                region.chars().all(|c| c.is_ascii_uppercase()),
                "{tag:?} region must be upper case"
            );
        }
        assert_eq!(Locale::Spanish.spotify_locale(), "es_ES");
        assert_eq!(Locale::PortugueseBrazil.spotify_locale(), "pt_BR");
        assert_eq!(Locale::ChineseTraditional.spotify_locale(), "zh_TW");
    }

    #[test]
    fn missing_messages_use_the_english_source() {
        assert_eq!(gettext(Locale::German, "Home"), "Start");
        let missing = "Not translated yet";
        assert_eq!(gettext(Locale::German, missing), missing);
        assert_eq!(gettext(Locale::English, "Home"), "Home");
        assert_eq!(Locale::default(), Locale::English);
    }

    #[test]
    fn zero_one_and_many_songs_have_complete_localized_labels() {
        for (count, english, german) in [
            (0, "Playlist • 0 songs", "Playlist • 0 Titel"),
            (1, "Playlist • 1 song", "Playlist • 1 Titel"),
            (2, "Playlist • 2 songs", "Playlist • 2 Titel"),
            (
                100_000,
                "Playlist • 100000 songs",
                "Playlist • 100000 Titel",
            ),
        ] {
            assert_eq!(Locale::English.liked_song_count(count), english);
            assert_eq!(Locale::German.liked_song_count(count), german);
        }
    }
    #[test]
    fn locale_plural_rules_cover_european_and_asian_forms() {
        for (locale, count, expected) in [
            (Locale::Spanish, 1, "Playlist • 1 canción"),
            (Locale::Spanish, 2, "Playlist • 2 canciones"),
            (Locale::French, 0, "Playlist • 0 titre"),
            (Locale::French, 2, "Playlist • 2 titres"),
            (Locale::Italian, 1, "Playlist • 1 brano"),
            (Locale::Italian, 2, "Playlist • 2 brani"),
            (Locale::PortugueseBrazil, 0, "Playlist • 0 música"),
            (Locale::PortuguesePortugal, 0, "Playlist • 0 músicas"),
            (Locale::Dutch, 2, "Playlist • 2 nummers"),
            (Locale::Swedish, 2, "Spellista • 2 låtar"),
            (Locale::Polish, 1, "Playlista • 1 utwór"),
            (Locale::Polish, 2, "Playlista • 2 utwory"),
            (Locale::Polish, 5, "Playlista • 5 utworów"),
            (Locale::Polish, 21, "Playlista • 21 utworów"),
            (Locale::Polish, 22, "Playlista • 22 utwory"),
            (Locale::Russian, 1, "Плейлист • 1 трек"),
            (Locale::Russian, 11, "Плейлист • 11 треков"),
            (Locale::Russian, 21, "Плейлист • 21 трек"),
            (Locale::Russian, 22, "Плейлист • 22 трека"),
            (Locale::Russian, 112, "Плейлист • 112 треков"),
            (Locale::Japanese, 0, "プレイリスト • 0曲"),
            (Locale::Japanese, 2, "プレイリスト • 2曲"),
            (Locale::ChineseSimplified, 2, "歌单 • 2 首歌曲"),
            (Locale::ChineseTraditional, 2, "播放清單 • 2 首歌曲"),
        ] {
            assert_eq!(locale.liked_song_count(count), expected);
        }
    }

    #[test]
    fn language_tags_map_to_the_closest_catalog() {
        for (tag, expected) in [
            // The region never changes the language on its own.
            ("es_UY", Locale::Spanish),
            ("es-ES", Locale::Spanish),
            ("es_UY.UTF-8", Locale::Spanish),
            ("es-419", Locale::Spanish),
            ("de-AT", Locale::German),
            ("de_CH.UTF-8", Locale::German),
            ("en-GB", Locale::English),
            // Portuguese and Chinese are two catalogs each.
            ("pt-BR", Locale::PortugueseBrazil),
            ("pt", Locale::PortugueseBrazil),
            ("pt-PT", Locale::PortuguesePortugal),
            ("pt_MZ", Locale::PortuguesePortugal),
            ("zh-Hans", Locale::ChineseSimplified),
            ("zh-CN", Locale::ChineseSimplified),
            ("zh_CN.GB2312", Locale::ChineseSimplified),
            ("zh-Hans-SG", Locale::ChineseSimplified),
            ("zh", Locale::ChineseSimplified),
            ("zh-Hant", Locale::ChineseTraditional),
            ("zh-TW", Locale::ChineseTraditional),
            ("zh-HK", Locale::ChineseTraditional),
            ("zh-Hant-MO", Locale::ChineseTraditional),
            // Windows writes the legacy Chinese scripts this way.
            ("zh-CHT", Locale::ChineseTraditional),
            ("zh-CHS", Locale::ChineseSimplified),
        ] {
            assert_eq!(Locale::from_language_tag(tag), Some(expected), "{tag}");
        }
    }

    #[test]
    fn untranslated_and_unset_languages_have_no_catalog() {
        // No catalog: the caller falls back to English rather than guessing.
        for tag in ["nb-NO", "ko-KR", "ar", "C", "POSIX", "", "-", "und"] {
            assert_eq!(Locale::from_language_tag(tag), None, "{tag}");
        }
    }

    #[test]
    fn every_locale_round_trips_through_its_tag_and_is_listed_once() {
        for locale in LOCALES {
            assert_eq!(Locale::from_language_tag(locale.tag()), Some(*locale));
            assert!(!locale.native_name().is_empty());
            assert_eq!(
                LOCALES.iter().filter(|other| *other == locale).count(),
                1,
                "{}",
                locale.tag()
            );
        }
        assert_eq!(
            LOCALES.len(),
            <Locale as clap::ValueEnum>::value_variants().len()
        );
    }

    #[test]
    fn the_system_language_is_a_catalog_the_app_carries() {
        // Under test this is pinned to English; off the test harness it is
        // whatever the desktop reads, which must still be a real catalog.
        let detected = Locale::from_system();
        assert!(LOCALES.contains(&detected));
        assert_eq!(Locale::from_system(), detected, "the answer is stable");
    }

    #[test]
    fn the_macros_translate_and_count_like_the_functions_behind_them() {
        assert_eq!(tr!(Locale::Spanish, "Search"), "Buscar");
        assert_eq!(tr!(Locale::English, "Search"), "Search");
        assert_eq!(
            trn!(
                Locale::Spanish,
                "Playlist • {count} song",
                "Playlist • {count} songs",
                2
            )
            .replace("{count}", "2"),
            "Playlist • 2 canciones"
        );
    }
}
