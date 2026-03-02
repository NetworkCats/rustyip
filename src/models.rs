use std::fmt::Write;
use std::net::IpAddr;

use serde::{Deserialize, Serialize};

// --- Deserialization types (from MMDB) ---

#[derive(Deserialize, Debug, Default)]
pub struct MmdbRecord<'a> {
    #[serde(borrow, default)]
    pub city: MmdbCity<'a>,
    #[serde(default)]
    pub continent: MmdbContinent<'a>,
    #[serde(default)]
    pub country: MmdbCountry<'a>,
    #[serde(default)]
    pub location: MmdbLocation<'a>,
    #[serde(default)]
    pub postal: MmdbPostal<'a>,
    #[serde(default)]
    pub registered_country: MmdbCountry<'a>,
    #[serde(default)]
    pub subdivisions: Vec<MmdbSubdivision<'a>>,
    #[serde(default)]
    pub asn: MmdbAsn<'a>,
    #[serde(default)]
    pub proxy: MmdbProxy,
}

#[derive(Deserialize, Debug, Default)]
pub struct MmdbCity<'a> {
    pub geoname_id: Option<u32>,
    #[serde(borrow, default)]
    pub names: MmdbNames<'a>,
}

#[derive(Deserialize, Debug, Default)]
pub struct MmdbContinent<'a> {
    pub code: Option<&'a str>,
    pub geoname_id: Option<u32>,
    #[serde(borrow, default)]
    pub names: MmdbNames<'a>,
}

#[derive(Deserialize, Debug, Default)]
pub struct MmdbCountry<'a> {
    pub geoname_id: Option<u32>,
    pub iso_code: Option<&'a str>,
    #[serde(borrow, default)]
    pub names: MmdbNames<'a>,
}

#[derive(Deserialize, Debug, Default)]
pub struct MmdbLocation<'a> {
    pub accuracy_radius: Option<u16>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub metro_code: Option<u16>,
    pub time_zone: Option<&'a str>,
}

#[derive(Deserialize, Debug, Default)]
pub struct MmdbPostal<'a> {
    pub code: Option<&'a str>,
}

#[derive(Deserialize, Debug, Default)]
pub struct MmdbSubdivision<'a> {
    pub geoname_id: Option<u32>,
    pub iso_code: Option<&'a str>,
    #[serde(borrow, default)]
    pub names: MmdbNames<'a>,
}

#[derive(Deserialize, Debug, Default)]
pub struct MmdbAsn<'a> {
    pub autonomous_system_number: Option<u32>,
    pub autonomous_system_organization: Option<&'a str>,
    pub as_domain: Option<&'a str>,
}

#[derive(Deserialize, Debug, Default)]
pub struct MmdbProxy {
    #[serde(default)]
    pub is_proxy: bool,
    #[serde(default)]
    pub is_vpn: bool,
    #[serde(default)]
    pub is_tor: bool,
    #[serde(default)]
    pub is_hosting: bool,
    #[serde(default)]
    pub is_cdn: bool,
    #[serde(default)]
    pub is_school: bool,
    #[serde(default)]
    pub is_anonymous: bool,
}

#[derive(Deserialize, Debug, Default)]
pub struct MmdbNames<'a> {
    #[serde(rename = "de", default)]
    pub german: Option<&'a str>,
    #[serde(rename = "en", default)]
    pub english: Option<&'a str>,
    #[serde(rename = "es", default)]
    pub spanish: Option<&'a str>,
    #[serde(rename = "fr", default)]
    pub french: Option<&'a str>,
    #[serde(rename = "ja", default)]
    pub japanese: Option<&'a str>,
    #[serde(rename = "pt-BR", default)]
    pub brazilian_portuguese: Option<&'a str>,
    #[serde(rename = "ru", default)]
    pub russian: Option<&'a str>,
    #[serde(rename = "zh-CN", default)]
    pub simplified_chinese: Option<&'a str>,
}

impl MmdbNames<'_> {
    fn is_empty(&self) -> bool {
        self.english.is_none()
            && self.german.is_none()
            && self.spanish.is_none()
            && self.french.is_none()
            && self.japanese.is_none()
            && self.brazilian_portuguese.is_none()
            && self.russian.is_none()
            && self.simplified_chinese.is_none()
    }
}

// --- Serialization types (API output) ---

#[derive(Debug, Serialize)]
pub struct IpInfo {
    pub ip: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<CityInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continent: Option<ContinentInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<CountryInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<LocationInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postal: Option<PostalInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registered_country: Option<CountryInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subdivisions: Option<Vec<SubdivisionInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asn: Option<AsnInfo>,
    pub proxy: ProxyInfo,
}

#[derive(Debug, Serialize)]
pub struct Names {
    #[serde(rename = "de", skip_serializing_if = "Option::is_none")]
    pub german: Option<String>,
    #[serde(rename = "en", skip_serializing_if = "Option::is_none")]
    pub english: Option<String>,
    #[serde(rename = "es", skip_serializing_if = "Option::is_none")]
    pub spanish: Option<String>,
    #[serde(rename = "fr", skip_serializing_if = "Option::is_none")]
    pub french: Option<String>,
    #[serde(rename = "ja", skip_serializing_if = "Option::is_none")]
    pub japanese: Option<String>,
    #[serde(rename = "pt-BR", skip_serializing_if = "Option::is_none")]
    pub brazilian_portuguese: Option<String>,
    #[serde(rename = "ru", skip_serializing_if = "Option::is_none")]
    pub russian: Option<String>,
    #[serde(rename = "zh-CN", skip_serializing_if = "Option::is_none")]
    pub simplified_chinese: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CityInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geoname_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub names: Option<Names>,
}

#[derive(Debug, Serialize)]
pub struct ContinentInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geoname_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub names: Option<Names>,
}

#[derive(Debug, Serialize)]
pub struct CountryInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geoname_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iso_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub names: Option<Names>,
}

#[derive(Debug, Serialize)]
pub struct LocationInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accuracy_radius: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metro_code: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_zone: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PostalInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SubdivisionInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geoname_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iso_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub names: Option<Names>,
}

#[derive(Debug, Serialize)]
pub struct AsnInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autonomous_system_number: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autonomous_system_organization: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub as_domain: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProxyInfo {
    pub is_proxy: bool,
    pub is_vpn: bool,
    pub is_tor: bool,
    pub is_hosting: bool,
    pub is_cdn: bool,
    pub is_school: bool,
    pub is_anonymous: bool,
}

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

    let proxy = ProxyInfo {
        is_proxy: record.proxy.is_proxy,
        is_vpn: record.proxy.is_vpn,
        is_tor: record.proxy.is_tor,
        is_hosting: record.proxy.is_hosting,
        is_cdn: record.proxy.is_cdn,
        is_school: record.proxy.is_school,
        is_anonymous: record.proxy.is_anonymous,
    };

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
