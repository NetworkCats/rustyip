use serde::Deserialize;

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

#[derive(Deserialize, Debug, Default, Clone, Copy)]
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
    pub fn is_empty(&self) -> bool {
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
