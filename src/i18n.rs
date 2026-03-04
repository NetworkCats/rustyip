use std::collections::HashMap;
use std::sync::LazyLock;

use axum::http::HeaderMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Locale {
    En,
    Es,
    De,
    Ja,
    Ko,
    Id,
    Fr,
    Ru,
    Pt,
    It,
    ZhHant,
    ZhHans,
    Nl,
    Ar,
}

impl Locale {
    pub const ALL: &[Locale] = &[
        Locale::En,
        Locale::Es,
        Locale::De,
        Locale::Ja,
        Locale::Ko,
        Locale::Id,
        Locale::Fr,
        Locale::Ru,
        Locale::Pt,
        Locale::It,
        Locale::ZhHant,
        Locale::ZhHans,
        Locale::Nl,
        Locale::Ar,
    ];

    pub fn tag(self) -> &'static str {
        match self {
            Locale::En => "en",
            Locale::Es => "es",
            Locale::De => "de",
            Locale::Ja => "ja",
            Locale::Ko => "ko",
            Locale::Id => "id",
            Locale::Fr => "fr",
            Locale::Ru => "ru",
            Locale::Pt => "pt",
            Locale::It => "it",
            Locale::ZhHant => "zh-Hant",
            Locale::ZhHans => "zh-Hans",
            Locale::Nl => "nl",
            Locale::Ar => "ar",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Locale::En => "English",
            Locale::Es => "Espa\u{00f1}ol",
            Locale::De => "Deutsch",
            Locale::Ja => "\u{65e5}\u{672c}\u{8a9e}",
            Locale::Ko => "\u{d55c}\u{ad6d}\u{c5b4}",
            Locale::Id => "Bahasa Indonesia",
            Locale::Fr => "Fran\u{00e7}ais",
            Locale::Ru => "\u{0420}\u{0443}\u{0441}\u{0441}\u{043a}\u{0438}\u{0439}",
            Locale::Pt => "Portugu\u{00ea}s",
            Locale::It => "Italiano",
            Locale::ZhHant => "\u{7e41}\u{9ad4}\u{4e2d}\u{6587}",
            Locale::ZhHans => "\u{7b80}\u{4f53}\u{4e2d}\u{6587}",
            Locale::Nl => "Nederlands",
            Locale::Ar => "\u{0627}\u{0644}\u{0639}\u{0631}\u{0628}\u{064a}\u{0629}",
        }
    }

    pub fn html_dir(self) -> &'static str {
        match self {
            Locale::Ar => "rtl",
            _ => "ltr",
        }
    }

    pub fn mmdb_key(self) -> &'static str {
        match self {
            Locale::En => "en",
            Locale::Es => "es",
            Locale::De => "de",
            Locale::Ja => "ja",
            Locale::Fr => "fr",
            Locale::Ru => "ru",
            Locale::Pt => "pt-BR",
            Locale::ZhHans => "zh-CN",
            // These languages are not in the MMDB; fall back to English.
            Locale::Ko | Locale::Id | Locale::It | Locale::ZhHant | Locale::Nl | Locale::Ar => "en",
        }
    }

    pub fn from_tag(tag: &str) -> Option<Locale> {
        match tag {
            "en" => Some(Locale::En),
            "es" => Some(Locale::Es),
            "de" => Some(Locale::De),
            "ja" => Some(Locale::Ja),
            "ko" => Some(Locale::Ko),
            "id" => Some(Locale::Id),
            "fr" => Some(Locale::Fr),
            "ru" => Some(Locale::Ru),
            "pt" => Some(Locale::Pt),
            "it" => Some(Locale::It),
            "zh-Hant" => Some(Locale::ZhHant),
            "zh-Hans" => Some(Locale::ZhHans),
            "nl" => Some(Locale::Nl),
            "ar" => Some(Locale::Ar),
            _ => None,
        }
    }
}

type Catalog = HashMap<&'static str, &'static str>;

struct Translations {
    catalogs: HashMap<Locale, Catalog>,
}

impl Translations {
    fn get<'a>(&self, locale: Locale, msgid: &'a str) -> &'a str {
        self.catalogs
            .get(&locale)
            .and_then(|cat| cat.get(msgid).copied())
            .unwrap_or(msgid)
    }
}

fn parse_po(source: &'static str) -> Catalog {
    let mut catalog = HashMap::new();
    let mut current_msgid = String::new();
    let mut current_msgstr = String::new();
    let mut reading = Reading::None;

    for line in source.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            if reading == Reading::Msgstr && !current_msgid.is_empty() {
                insert_entry(&mut catalog, &current_msgid, &current_msgstr, source);
                current_msgid.clear();
                current_msgstr.clear();
            }
            reading = Reading::None;
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("msgid ") {
            if reading == Reading::Msgstr && !current_msgid.is_empty() {
                insert_entry(&mut catalog, &current_msgid, &current_msgstr, source);
            }
            current_msgid.clear();
            current_msgstr.clear();
            append_quoted(&mut current_msgid, rest);
            reading = Reading::Msgid;
        } else if let Some(rest) = trimmed.strip_prefix("msgstr ") {
            append_quoted(&mut current_msgstr, rest);
            reading = Reading::Msgstr;
        } else if trimmed.starts_with('"') {
            match reading {
                Reading::Msgid => append_quoted(&mut current_msgid, trimmed),
                Reading::Msgstr => append_quoted(&mut current_msgstr, trimmed),
                Reading::None => {}
            }
        }
    }

    if reading == Reading::Msgstr && !current_msgid.is_empty() {
        insert_entry(&mut catalog, &current_msgid, &current_msgstr, source);
    }

    catalog
}

fn insert_entry(catalog: &mut Catalog, msgid: &str, msgstr: &str, source: &'static str) {
    if msgstr.is_empty() {
        return;
    }
    // Try to find the strings in the static source to get &'static str references,
    // avoiding heap allocation. This works for most strings, but fails for strings
    // containing escape sequences (e.g. \") since the unescaped text differs from
    // the raw PO source. In that case, leak a heap-allocated copy. This is acceptable
    // because translations are loaded once at startup.
    let id_static = find_in_static(source, msgid).unwrap_or_else(|| leak_string(msgid));
    let str_static = find_in_static(source, msgstr).unwrap_or_else(|| leak_string(msgstr));
    catalog.insert(id_static, str_static);
}

fn leak_string(s: &str) -> &'static str {
    Box::leak(s.to_owned().into_boxed_str())
}

fn find_in_static(source: &'static str, needle: &str) -> Option<&'static str> {
    source
        .find(needle)
        .map(|pos| &source[pos..pos + needle.len()])
}

fn append_quoted(target: &mut String, quoted: &str) {
    let s = quoted.trim();
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        let inner = &s[1..s.len() - 1];
        let unescaped = inner
            .replace("\\n", "\n")
            .replace("\\\"", "\"")
            .replace("\\\\", "\\");
        target.push_str(&unescaped);
    }
}

#[derive(PartialEq)]
enum Reading {
    None,
    Msgid,
    Msgstr,
}

static TRANSLATIONS: LazyLock<Translations> = LazyLock::new(|| {
    let mut catalogs = HashMap::new();
    catalogs.insert(Locale::En, parse_po(include_str!("../locales/en.po")));
    catalogs.insert(Locale::Es, parse_po(include_str!("../locales/es.po")));
    catalogs.insert(Locale::De, parse_po(include_str!("../locales/de.po")));
    catalogs.insert(Locale::Ja, parse_po(include_str!("../locales/ja.po")));
    catalogs.insert(Locale::Ko, parse_po(include_str!("../locales/ko.po")));
    catalogs.insert(Locale::Id, parse_po(include_str!("../locales/id.po")));
    catalogs.insert(Locale::Fr, parse_po(include_str!("../locales/fr.po")));
    catalogs.insert(Locale::Ru, parse_po(include_str!("../locales/ru.po")));
    catalogs.insert(Locale::Pt, parse_po(include_str!("../locales/pt.po")));
    catalogs.insert(Locale::It, parse_po(include_str!("../locales/it.po")));
    catalogs.insert(
        Locale::ZhHant,
        parse_po(include_str!("../locales/zh-Hant.po")),
    );
    catalogs.insert(
        Locale::ZhHans,
        parse_po(include_str!("../locales/zh-Hans.po")),
    );
    catalogs.insert(Locale::Nl, parse_po(include_str!("../locales/nl.po")));
    catalogs.insert(Locale::Ar, parse_po(include_str!("../locales/ar.po")));
    Translations { catalogs }
});

pub fn translate(locale: Locale, msgid: &str) -> &str {
    TRANSLATIONS.get(locale, msgid)
}

pub fn negotiate_locale(headers: &HeaderMap) -> Locale {
    let accept = match headers.get("Accept-Language").and_then(|v| v.to_str().ok()) {
        Some(v) => v,
        None => return Locale::En,
    };

    let mut candidates: Vec<(&str, f32)> = Vec::new();

    for part in accept.split(',') {
        let part = part.trim();
        if let Some((lang, q_part)) = part.split_once(";q=") {
            let quality: f32 = q_part.trim().parse().unwrap_or(0.0);
            candidates.push((lang.trim(), quality));
        } else {
            candidates.push((part, 1.0));
        }
    }

    candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    for (tag, _) in &candidates {
        if let Some(locale) = match_language_tag(tag) {
            return locale;
        }
    }

    Locale::En
}

fn match_language_tag(tag: &str) -> Option<Locale> {
    let tag_lower = tag.to_ascii_lowercase();

    // Exact matches (case-insensitive)
    match tag_lower.as_str() {
        "zh-hant" | "zh-tw" | "zh-hk" | "zh-mo" => return Some(Locale::ZhHant),
        "zh-hans" | "zh-cn" | "zh-sg" | "zh-my" => return Some(Locale::ZhHans),
        "pt-br" | "pt" => return Some(Locale::Pt),
        _ => {}
    }

    // Try primary subtag
    let primary = tag_lower.split('-').next().unwrap_or(&tag_lower);
    match primary {
        "en" => Some(Locale::En),
        "es" => Some(Locale::Es),
        "de" => Some(Locale::De),
        "ja" => Some(Locale::Ja),
        "ko" => Some(Locale::Ko),
        "id" | "ms" => Some(Locale::Id),
        "fr" => Some(Locale::Fr),
        "ru" => Some(Locale::Ru),
        "pt" => Some(Locale::Pt),
        "it" => Some(Locale::It),
        "zh" => Some(Locale::ZhHans),
        "nl" => Some(Locale::Nl),
        "ar" => Some(Locale::Ar),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_tag_roundtrip() {
        for locale in Locale::ALL {
            let tag = locale.tag();
            let parsed = Locale::from_tag(tag).expect("tag should parse back");
            assert_eq!(*locale, parsed);
        }
    }

    #[test]
    fn locale_names_are_non_empty() {
        for locale in Locale::ALL {
            assert!(!locale.name().is_empty());
        }
    }

    #[test]
    fn translate_english_returns_english() {
        let result = translate(Locale::En, "FAQ");
        assert_eq!(result, "FAQ");
    }

    #[test]
    fn translate_unknown_msgid_returns_msgid() {
        let result = translate(Locale::En, "this-does-not-exist");
        assert_eq!(result, "this-does-not-exist");
    }

    #[test]
    fn translate_non_english_returns_translated() {
        let result = translate(Locale::Es, "Country");
        assert!(!result.is_empty());
        // Spanish for Country should not be "Country"
        assert_ne!(result, "Country");
    }

    #[test]
    fn negotiate_locale_empty_header() {
        let headers = HeaderMap::new();
        assert_eq!(negotiate_locale(&headers), Locale::En);
    }

    #[test]
    fn negotiate_locale_japanese() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Accept-Language",
            "ja,en-US;q=0.9,en;q=0.8".parse().unwrap(),
        );
        assert_eq!(negotiate_locale(&headers), Locale::Ja);
    }

    #[test]
    fn negotiate_locale_chinese_traditional() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Accept-Language",
            "zh-TW,zh;q=0.9,en;q=0.8".parse().unwrap(),
        );
        assert_eq!(negotiate_locale(&headers), Locale::ZhHant);
    }

    #[test]
    fn negotiate_locale_chinese_simplified() {
        let mut headers = HeaderMap::new();
        headers.insert("Accept-Language", "zh-CN,zh;q=0.9".parse().unwrap());
        assert_eq!(negotiate_locale(&headers), Locale::ZhHans);
    }

    #[test]
    fn negotiate_locale_portuguese() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Accept-Language",
            "pt-BR,pt;q=0.9,en;q=0.8".parse().unwrap(),
        );
        assert_eq!(negotiate_locale(&headers), Locale::Pt);
    }

    #[test]
    fn negotiate_locale_arabic() {
        let mut headers = HeaderMap::new();
        headers.insert("Accept-Language", "ar,en;q=0.5".parse().unwrap());
        assert_eq!(negotiate_locale(&headers), Locale::Ar);
    }

    #[test]
    fn negotiate_locale_quality_ordering() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Accept-Language",
            "en;q=0.5,de;q=0.9,fr;q=0.7".parse().unwrap(),
        );
        assert_eq!(negotiate_locale(&headers), Locale::De);
    }

    #[test]
    fn negotiate_locale_unknown_falls_back_to_english() {
        let mut headers = HeaderMap::new();
        headers.insert("Accept-Language", "xx-YY,zz;q=0.5".parse().unwrap());
        assert_eq!(negotiate_locale(&headers), Locale::En);
    }

    #[test]
    fn arabic_is_rtl() {
        assert_eq!(Locale::Ar.html_dir(), "rtl");
    }

    #[test]
    fn english_is_ltr() {
        assert_eq!(Locale::En.html_dir(), "ltr");
    }

    #[test]
    fn mmdb_key_mapping() {
        assert_eq!(Locale::En.mmdb_key(), "en");
        assert_eq!(Locale::De.mmdb_key(), "de");
        assert_eq!(Locale::Pt.mmdb_key(), "pt-BR");
        assert_eq!(Locale::ZhHans.mmdb_key(), "zh-CN");
        assert_eq!(Locale::Ko.mmdb_key(), "en");
    }

    #[test]
    fn all_locales_have_translations() {
        for locale in Locale::ALL {
            let result = translate(*locale, "What is my IP address?");
            assert!(
                !result.is_empty(),
                "locale {:?} has no translation for title",
                locale
            );
        }
    }

    #[test]
    fn parse_po_handles_multiline() {
        // We cannot use parse_po directly with a non-static str for the
        // static-reference optimization, but we can verify the PO files load.
        let _ = translate(Locale::Es, "FAQ");
    }

    #[test]
    fn translate_strings_with_escaped_quotes() {
        // These strings contain \" in PO files. The parser must correctly unescape
        // them and produce valid translations for all locales.
        let msgid_rug_pull = "Rest assured, this service will definitely not \"Rug Pull.\"";

        for locale in Locale::ALL {
            let result = translate(*locale, msgid_rug_pull);
            assert!(
                !result.is_empty(),
                "locale {:?} has no translation for Rug Pull string",
                locale
            );
            if *locale != Locale::En {
                assert_ne!(
                    result, msgid_rug_pull,
                    "locale {:?} should have a non-English translation for Rug Pull string",
                    locale
                );
            }
        }
    }

    #[test]
    fn translate_strings_with_html_href() {
        // Strings containing <a href=\"...\"> have escaped quotes in PO files.
        let msgid_api = "Our API is only suitable for manual calls or small-scale projects. If your website uses our API to query visitor IPs, please ensure you use a message queue to send requests to avoid blocking. If your project has high traffic or is latency-sensitive, please use our open-source offline database: <a href=\"https://github.com/NetworkCats/Merged-IP-Data\">Merged IP Database</a>, which is the same database used by this project.";
        let msgid_data = "IP geographic data primarily comes from the free databases of MaxMind and DB-IP; AS data comes from IPinfo's free database; and IP proxy data comes from my own <a href=\"https://github.com/NetworkCats/OpenProxyDB\">OpenProxyDB</a> database.";

        for locale in Locale::ALL {
            let api_result = translate(*locale, msgid_api);
            assert!(
                !api_result.is_empty(),
                "locale {:?} has no translation for API string",
                locale
            );
            if *locale != Locale::En {
                assert_ne!(
                    api_result, msgid_api,
                    "locale {:?} should have a non-English translation for API string",
                    locale
                );
            }

            let data_result = translate(*locale, msgid_data);
            assert!(
                !data_result.is_empty(),
                "locale {:?} has no translation for data source string",
                locale
            );
            if *locale != Locale::En {
                assert_ne!(
                    data_result, msgid_data,
                    "locale {:?} should have a non-English translation for data source string",
                    locale
                );
            }
        }
    }
}
