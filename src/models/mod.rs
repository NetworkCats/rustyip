//! Data models for MMDB deserialization and JSON API serialization.

pub mod api;
pub mod mmdb;

use std::fmt::Write;
use std::net::IpAddr;

pub use api::*;
// Mmdb types are re-exported as pub(crate) since they are internal deserialization details.
pub(crate) use mmdb::*;

// --- Conversion from MMDB record to API output ---

fn convert_names(names: &MmdbNames<'_>) -> Option<Names> {
    if names.is_empty() {
        return None;
    }
    Some(Names {
        german: names.german.map(str::to_owned),
        english: names.english.map(str::to_owned),
        spanish: names.spanish.map(str::to_owned),
        french: names.french.map(str::to_owned),
        japanese: names.japanese.map(str::to_owned),
        brazilian_portuguese: names.brazilian_portuguese.map(str::to_owned),
        russian: names.russian.map(str::to_owned),
        simplified_chinese: names.simplified_chinese.map(str::to_owned),
    })
}

fn has_city(c: &MmdbCity<'_>) -> bool {
    c.geoname_id.is_some() || !c.names.is_empty()
}

fn has_continent(c: &MmdbContinent<'_>) -> bool {
    c.code.is_some() || c.geoname_id.is_some() || !c.names.is_empty()
}

fn has_country(c: &MmdbCountry<'_>) -> bool {
    c.geoname_id.is_some() || c.iso_code.is_some() || !c.names.is_empty()
}

fn has_location(l: &MmdbLocation<'_>) -> bool {
    l.accuracy_radius.is_some()
        || l.latitude.is_some()
        || l.longitude.is_some()
        || l.metro_code.is_some()
        || l.time_zone.is_some()
}

fn has_postal(p: &MmdbPostal<'_>) -> bool {
    p.code.is_some()
}

fn has_asn(a: &MmdbAsn<'_>) -> bool {
    a.autonomous_system_number.is_some()
        || a.autonomous_system_organization.is_some()
        || a.as_domain.is_some()
}

pub fn from_mmdb_record(ip: IpAddr, record: &MmdbRecord<'_>) -> IpInfo {
    let city = if has_city(&record.city) {
        Some(CityInfo {
            geoname_id: record.city.geoname_id,
            names: convert_names(&record.city.names),
        })
    } else {
        None
    };

    let continent = if has_continent(&record.continent) {
        Some(ContinentInfo {
            code: record.continent.code.map(|s| s.to_string()),
            geoname_id: record.continent.geoname_id,
            names: convert_names(&record.continent.names),
        })
    } else {
        None
    };

    let country = if has_country(&record.country) {
        Some(CountryInfo {
            geoname_id: record.country.geoname_id,
            iso_code: record.country.iso_code.map(|s| s.to_string()),
            names: convert_names(&record.country.names),
        })
    } else {
        None
    };

    let location = if has_location(&record.location) {
        Some(LocationInfo {
            accuracy_radius: record.location.accuracy_radius,
            latitude: record.location.latitude,
            longitude: record.location.longitude,
            metro_code: record.location.metro_code,
            time_zone: record.location.time_zone.map(|s| s.to_string()),
        })
    } else {
        None
    };

    let postal = if has_postal(&record.postal) {
        Some(PostalInfo {
            code: record.postal.code.map(|s| s.to_string()),
        })
    } else {
        None
    };

    let registered_country = if has_country(&record.registered_country) {
        Some(CountryInfo {
            geoname_id: record.registered_country.geoname_id,
            iso_code: record.registered_country.iso_code.map(|s| s.to_string()),
            names: convert_names(&record.registered_country.names),
        })
    } else {
        None
    };

    let subdivisions = if record.subdivisions.is_empty() {
        None
    } else {
        Some(
            record
                .subdivisions
                .iter()
                .map(|s| SubdivisionInfo {
                    geoname_id: s.geoname_id,
                    iso_code: s.iso_code.map(|s| s.to_string()),
                    names: convert_names(&s.names),
                })
                .collect(),
        )
    };

    let asn = if has_asn(&record.asn) {
        Some(AsnInfo {
            autonomous_system_number: record.asn.autonomous_system_number,
            autonomous_system_organization: record
                .asn
                .autonomous_system_organization
                .map(|s| s.to_string()),
            as_domain: record.asn.as_domain.map(|s| s.to_string()),
        })
    } else {
        None
    };

    let proxy = ProxyInfo::from(record.proxy);

    let mut ip_str = String::with_capacity(45);
    write!(ip_str, "{ip}").expect("IP address formatting cannot fail");

    IpInfo {
        ip: ip_str,
        city,
        continent,
        country,
        location,
        postal,
        registered_country,
        subdivisions,
        asn,
        proxy,
    }
}

pub fn get_en_name(names: &Option<Names>) -> &str {
    names
        .as_ref()
        .and_then(|n| n.english.as_deref())
        .unwrap_or_default()
}

pub fn get_localized_name<'a>(names: &'a Option<Names>, mmdb_key: &str) -> &'a str {
    names
        .as_ref()
        .and_then(|n| match mmdb_key {
            "de" => n.german.as_deref().or(n.english.as_deref()),
            "es" => n.spanish.as_deref().or(n.english.as_deref()),
            "fr" => n.french.as_deref().or(n.english.as_deref()),
            "ja" => n.japanese.as_deref().or(n.english.as_deref()),
            "pt-BR" => n.brazilian_portuguese.as_deref().or(n.english.as_deref()),
            "ru" => n.russian.as_deref().or(n.english.as_deref()),
            "zh-CN" => n.simplified_chinese.as_deref().or(n.english.as_deref()),
            _ => n.english.as_deref(),
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    fn empty_mmdb_names<'a>() -> MmdbNames<'a> {
        MmdbNames::default()
    }

    fn english_only_names<'a>() -> MmdbNames<'a> {
        MmdbNames {
            english: Some("TestName"),
            ..Default::default()
        }
    }

    fn full_names<'a>() -> MmdbNames<'a> {
        MmdbNames {
            german: Some("Deutsch"),
            english: Some("English"),
            spanish: Some("Espanol"),
            french: Some("Francais"),
            japanese: Some("Nihongo"),
            brazilian_portuguese: Some("Portugues"),
            russian: Some("Russkiy"),
            simplified_chinese: Some("Zhongwen"),
        }
    }

    // --- MmdbNames::is_empty tests ---

    #[test]
    fn mmdb_names_is_empty_when_all_none() {
        let names = empty_mmdb_names();
        assert!(names.is_empty());
    }

    #[test]
    fn mmdb_names_is_not_empty_with_english() {
        let names = english_only_names();
        assert!(!names.is_empty());
    }

    #[test]
    fn mmdb_names_is_not_empty_with_non_english_only() {
        let names = MmdbNames {
            german: Some("Berlin"),
            ..Default::default()
        };
        assert!(!names.is_empty());
    }

    #[test]
    fn mmdb_names_is_not_empty_when_fully_populated() {
        let names = full_names();
        assert!(!names.is_empty());
    }

    // --- convert_names tests ---

    #[test]
    fn convert_names_returns_none_for_empty() {
        let names = empty_mmdb_names();
        assert!(convert_names(&names).is_none());
    }

    #[test]
    fn convert_names_returns_some_with_english() {
        let names = english_only_names();
        let result = convert_names(&names).expect("should produce Some");
        assert_eq!(result.english.as_deref(), Some("TestName"));
        assert!(result.german.is_none());
        assert!(result.spanish.is_none());
    }

    #[test]
    fn convert_names_copies_all_languages() {
        let names = full_names();
        let result = convert_names(&names).expect("should produce Some");
        assert_eq!(result.german.as_deref(), Some("Deutsch"));
        assert_eq!(result.english.as_deref(), Some("English"));
        assert_eq!(result.spanish.as_deref(), Some("Espanol"));
        assert_eq!(result.french.as_deref(), Some("Francais"));
        assert_eq!(result.japanese.as_deref(), Some("Nihongo"));
        assert_eq!(result.brazilian_portuguese.as_deref(), Some("Portugues"));
        assert_eq!(result.russian.as_deref(), Some("Russkiy"));
        assert_eq!(result.simplified_chinese.as_deref(), Some("Zhongwen"));
    }

    // --- has_* predicate tests ---

    #[test]
    fn has_city_false_for_default() {
        let city = MmdbCity::default();
        assert!(!has_city(&city));
    }

    #[test]
    fn has_city_true_with_geoname_id() {
        let city = MmdbCity {
            geoname_id: Some(12345),
            names: empty_mmdb_names(),
        };
        assert!(has_city(&city));
    }

    #[test]
    fn has_city_true_with_names() {
        let city = MmdbCity {
            geoname_id: None,
            names: english_only_names(),
        };
        assert!(has_city(&city));
    }

    #[test]
    fn has_continent_false_for_default() {
        let continent = MmdbContinent::default();
        assert!(!has_continent(&continent));
    }

    #[test]
    fn has_continent_true_with_code() {
        let continent = MmdbContinent {
            code: Some("NA"),
            geoname_id: None,
            names: empty_mmdb_names(),
        };
        assert!(has_continent(&continent));
    }

    #[test]
    fn has_country_false_for_default() {
        let country = MmdbCountry::default();
        assert!(!has_country(&country));
    }

    #[test]
    fn has_country_true_with_iso_code() {
        let country = MmdbCountry {
            geoname_id: None,
            iso_code: Some("US"),
            names: empty_mmdb_names(),
        };
        assert!(has_country(&country));
    }

    #[test]
    fn has_location_false_for_default() {
        let loc = MmdbLocation::default();
        assert!(!has_location(&loc));
    }

    #[test]
    fn has_location_true_with_latitude() {
        let loc = MmdbLocation {
            latitude: Some(40.0),
            ..Default::default()
        };
        assert!(has_location(&loc));
    }

    #[test]
    fn has_location_true_with_timezone() {
        let loc = MmdbLocation {
            time_zone: Some("America/New_York"),
            ..Default::default()
        };
        assert!(has_location(&loc));
    }

    #[test]
    fn has_postal_false_for_default() {
        let postal = MmdbPostal::default();
        assert!(!has_postal(&postal));
    }

    #[test]
    fn has_postal_true_with_code() {
        let postal = MmdbPostal {
            code: Some("10001"),
        };
        assert!(has_postal(&postal));
    }

    #[test]
    fn has_asn_false_for_default() {
        let asn = MmdbAsn::default();
        assert!(!has_asn(&asn));
    }

    #[test]
    fn has_asn_true_with_number() {
        let asn = MmdbAsn {
            autonomous_system_number: Some(13335),
            autonomous_system_organization: None,
            as_domain: None,
        };
        assert!(has_asn(&asn));
    }

    #[test]
    fn has_asn_true_with_org() {
        let asn = MmdbAsn {
            autonomous_system_number: None,
            autonomous_system_organization: Some("Cloudflare"),
            as_domain: None,
        };
        assert!(has_asn(&asn));
    }

    #[test]
    fn has_asn_true_with_domain() {
        let asn = MmdbAsn {
            autonomous_system_number: None,
            autonomous_system_organization: None,
            as_domain: Some("cloudflare.com"),
        };
        assert!(has_asn(&asn));
    }

    // --- from_mmdb_record tests ---

    #[test]
    fn from_mmdb_record_with_empty_record() {
        let record = MmdbRecord::default();
        let ip = IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4));
        let info = from_mmdb_record(ip, &record);

        assert_eq!(info.ip, "1.2.3.4");
        assert!(info.city.is_none());
        assert!(info.continent.is_none());
        assert!(info.country.is_none());
        assert!(info.location.is_none());
        assert!(info.postal.is_none());
        assert!(info.registered_country.is_none());
        assert!(info.subdivisions.is_none());
        assert!(info.asn.is_none());
        assert!(!info.proxy.is_proxy);
        assert!(!info.proxy.is_vpn);
        assert!(!info.proxy.is_tor);
        assert!(!info.proxy.is_hosting);
    }

    #[test]
    fn from_mmdb_record_with_populated_record() {
        let record = MmdbRecord {
            city: MmdbCity {
                geoname_id: Some(5101798),
                names: english_only_names(),
            },
            continent: MmdbContinent {
                code: Some("NA"),
                geoname_id: Some(6255149),
                names: english_only_names(),
            },
            country: MmdbCountry {
                geoname_id: Some(6252001),
                iso_code: Some("US"),
                names: english_only_names(),
            },
            location: MmdbLocation {
                accuracy_radius: Some(1000),
                latitude: Some(40.5),
                longitude: Some(-74.3),
                metro_code: Some(501),
                time_zone: Some("America/New_York"),
            },
            postal: MmdbPostal {
                code: Some("08854"),
            },
            registered_country: MmdbCountry {
                geoname_id: Some(6252001),
                iso_code: Some("US"),
                names: english_only_names(),
            },
            subdivisions: vec![MmdbSubdivision {
                geoname_id: Some(5101760),
                iso_code: Some("NJ"),
                names: english_only_names(),
            }],
            asn: MmdbAsn {
                autonomous_system_number: Some(20473),
                autonomous_system_organization: Some("The Constant Company"),
                as_domain: Some("vultr.com"),
            },
            proxy: MmdbProxy {
                is_proxy: true,
                is_hosting: true,
                ..Default::default()
            },
        };

        let ip = IpAddr::V4(Ipv4Addr::new(45, 77, 77, 77));
        let info = from_mmdb_record(ip, &record);

        assert_eq!(info.ip, "45.77.77.77");

        let city = info.city.as_ref().expect("city should be present");
        assert_eq!(city.geoname_id, Some(5101798));

        let continent = info
            .continent
            .as_ref()
            .expect("continent should be present");
        assert_eq!(continent.code.as_deref(), Some("NA"));

        let country = info.country.as_ref().expect("country should be present");
        assert_eq!(country.iso_code.as_deref(), Some("US"));

        let location = info.location.as_ref().expect("location should be present");
        assert_eq!(location.accuracy_radius, Some(1000));
        assert_eq!(location.latitude, Some(40.5));
        assert_eq!(location.time_zone.as_deref(), Some("America/New_York"));

        let postal = info.postal.as_ref().expect("postal should be present");
        assert_eq!(postal.code.as_deref(), Some("08854"));

        assert!(info.registered_country.is_some());

        let subdivisions = info
            .subdivisions
            .as_ref()
            .expect("subdivisions should be present");
        assert_eq!(subdivisions.len(), 1);
        assert_eq!(subdivisions[0].iso_code.as_deref(), Some("NJ"));

        let asn = info.asn.as_ref().expect("asn should be present");
        assert_eq!(asn.autonomous_system_number, Some(20473));
        assert_eq!(
            asn.autonomous_system_organization.as_deref(),
            Some("The Constant Company")
        );
        assert_eq!(asn.as_domain.as_deref(), Some("vultr.com"));

        assert!(info.proxy.is_proxy);
        assert!(info.proxy.is_hosting);
        assert!(!info.proxy.is_vpn);
        assert!(!info.proxy.is_tor);
    }

    #[test]
    fn from_mmdb_record_with_ipv6() {
        let record = MmdbRecord::default();
        let ip = IpAddr::V6(Ipv6Addr::new(0x2606, 0x4700, 0x4700, 0, 0, 0, 0, 0x1111));
        let info = from_mmdb_record(ip, &record);
        assert_eq!(info.ip, "2606:4700:4700::1111");
    }

    #[test]
    fn from_mmdb_record_empty_subdivisions_become_none() {
        let record = MmdbRecord {
            subdivisions: vec![],
            ..Default::default()
        };
        let ip = IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1));
        let info = from_mmdb_record(ip, &record);
        assert!(info.subdivisions.is_none());
    }

    #[test]
    fn from_mmdb_record_proxy_flags_are_forwarded() {
        let record = MmdbRecord {
            proxy: MmdbProxy {
                is_proxy: true,
                is_vpn: true,
                is_tor: true,
                is_hosting: true,
                is_cdn: true,
                is_school: true,
                is_anonymous: true,
            },
            ..Default::default()
        };
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        let info = from_mmdb_record(ip, &record);
        assert!(info.proxy.is_proxy);
        assert!(info.proxy.is_vpn);
        assert!(info.proxy.is_tor);
        assert!(info.proxy.is_hosting);
        assert!(info.proxy.is_cdn);
        assert!(info.proxy.is_school);
        assert!(info.proxy.is_anonymous);
    }

    // --- get_en_name tests ---

    #[test]
    fn get_en_name_returns_empty_for_none() {
        let names: Option<Names> = None;
        assert_eq!(get_en_name(&names), "");
    }

    #[test]
    fn get_en_name_returns_empty_when_english_missing() {
        let names = Some(Names {
            german: Some("Berlin".to_string()),
            english: None,
            spanish: None,
            french: None,
            japanese: None,
            brazilian_portuguese: None,
            russian: None,
            simplified_chinese: None,
        });
        assert_eq!(get_en_name(&names), "");
    }

    #[test]
    fn get_en_name_returns_english_value() {
        let names = Some(Names {
            english: Some("New York".to_string()),
            german: None,
            spanish: None,
            french: None,
            japanese: None,
            brazilian_portuguese: None,
            russian: None,
            simplified_chinese: None,
        });
        assert_eq!(get_en_name(&names), "New York");
    }
}
