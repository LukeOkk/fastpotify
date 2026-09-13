---
title: Translating Fastpotify
description: Edit standard gettext catalogs and preview the translation pilot.
nav_order: 6
---

Fastpotify uses gettext `.po` files, so contributors can use existing translation
editors such as Poedit or import the catalogs into Weblate. Translations are
bundled in the application. No translation service is contacted at runtime.

## Choosing a language

Fastpotify starts in the language the desktop is set to -- read from macOS's
preferred languages, Windows's preferred UI languages, or the `LANGUAGE`,
`LC_ALL`, `LC_MESSAGES` and `LANG` variables -- and falls back to English when
that language has no catalog here. Settings › Appearance › Language overrides
it, listing every bundled language under its own name; the choice applies
immediately and is saved. Regions map to the nearest catalog: `es_UY` and
`es_ES` are both Spanish, `de_AT` is German, `pt_BR` and `pt_PT` are separate
catalogs, and `zh_TW`, `zh_HK` and `zh-Hant` are Traditional Chinese while
`zh_CN`, `zh_SG` and `zh-Hans` are Simplified.

## Languages

| Language | Tag (`--demo-language`, `settings.json`) |
| --- | --- |
| English | `en` |
| Spanish | `es` |
| German | `de-DE` |
| Dutch | `nl` |
| Portuguese (Brazil) | `pt-BR` |
| Portuguese (Portugal) | `pt-PT` |
| French | `fr` |
| Swedish | `sv` |
| Polish | `pl` |
| Russian | `ru` |
| Italian | `it` |
| Japanese | `ja` |
| Chinese (Simplified) | `zh-Hans` |
| Chinese (Traditional) | `zh-Hant` |

Coverage is still growing: a message no catalog has translated is shown in
English rather than left blank, so an incomplete language is usable while it
fills in. Corrections from fluent speakers are welcome.

Song, album, artist and playlist names come from Spotify or their creators and
are kept as provided, as are Spotify's own brand terms.

## Edit and preview

The repository's `assets/i18n/fastpotify.pot` is the English source template.
Open the PO for your language, such as `assets/i18n/es.po`, in your translation editor. Edit `msgstr` values;
keep `msgid`, `msgid_plural`, and placeholders such as `{count}` unchanged.
Translator comments explain the placeholders. Clear a fuzzy flag only after
reviewing the translation against its current English source.

Build and preview your changes with:

```sh
cargo run --features demo -- --demo --demo-language es
cargo run --features demo -- --demo --demo-language es --demo-show light --demo-size 760x620
```

Check a narrow and a normal window, light and dark themes, keyboard navigation,
and screen-reader names. `--demo-shot PATH` saves the preview and exits. Demo
data needs no Spotify account. When automating screenshots, give the process
its own XDG config, data and state directories on Linux so framework window and
scroll state do not carry between captures.

## Update the template and check catalogs

Maintainers mark source phrases with `tr!(locale, "English text")` and whole
counted phrases with `trn!(locale, "Singular", "Plural", count)`. The source
must be a literal: build an interpolated string by translating a
`{placeholder}` form and calling `.replace()` on the result. Add any new source
file to `assets/i18n/POTFILES`. With GNU gettext tools that support Rust
installed, run:

```sh
.github/scripts/update-translations.sh
.github/scripts/update-translations.sh --check
cargo test --locked --test localization
```

The update command extracts the template with `xgettext` and merges it into
existing PO files with `msgmerge`. The check command verifies the template and
uses `msgfmt` to check catalog syntax and format placeholders. Submit changed
PO files and the template, together with any required source changes. Generated
Rust catalogs stay in Cargo's build directory and are not committed.

Normal application builds need no external gettext tools. The build validates
the PO files and compiles their translations and plural expressions to Rust.
Missing, empty, fuzzy, or incomplete plural entries fall back to the full English
phrase. Each locale's `Plural-Forms` header determines its plural choices;
languages are not restricted to two forms.

## Another language or a correction

Create another PO from the template using your editor's new-translation command,
or `msginit`. Set its language and plural rules and translate the pilot entries.
A maintainer must also register the locale in the app and preview it before it
becomes available. Adding a PO alone does not add a production language option.

Use the [translation problem form](https://github.com/crmne/fastpotify/issues/new?template=translation.yml)
for incorrect wording, missing translations or text that does not fit. Each
report gets its own issue. Include the language, version, affected control, and
the text you see; a suggested correction is welcome. The catalog headers link
to this form too.
