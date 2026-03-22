use serde::Serialize;

use super::mmdb::MmdbProxy;

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

impl From<MmdbProxy> for ProxyInfo {
    fn from(p: MmdbProxy) -> Self {
        Self {
            is_proxy: p.is_proxy,
            is_vpn: p.is_vpn,
            is_tor: p.is_tor,
            is_hosting: p.is_hosting,
            is_cdn: p.is_cdn,
            is_school: p.is_school,
            is_anonymous: p.is_anonymous,
        }
    }
}
