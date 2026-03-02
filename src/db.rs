use std::net::IpAddr;
use std::path::Path;
use std::sync::Arc;

use arc_swap::ArcSwap;
use maxminddb::Reader;
use serde::Deserialize;

use crate::models::{
    from_mmdb_record, IpInfo, MmdbAsn, MmdbCity, MmdbCountry, MmdbProxy, MmdbRecord, ProxyInfo,
};

pub type DbReader = Reader<Vec<u8>>;
pub type SharedDb = Arc<ArcSwap<DbReader>>;

pub fn load_db(path: &Path) -> Result<DbReader, maxminddb::MaxMindDbError> {
    Reader::open_readfile(path)
}

pub fn new_shared(reader: DbReader) -> SharedDb {
    Arc::new(ArcSwap::from_pointee(reader))
}

pub fn lookup(db: &DbReader, ip: IpAddr) -> Option<IpInfo> {
    let result = db.lookup(ip).ok()?;
    let record: MmdbRecord<'_> = result.decode().ok()??;
    Some(from_mmdb_record(ip, &record))
}

// Lightweight deserialization structs for single-field endpoints.
// These avoid deserializing the full MMDB record when only one field is needed.

#[derive(Deserialize, Default)]
struct ProxyOnly {
    #[serde(default)]
    proxy: MmdbProxy,
}

#[derive(Deserialize, Default)]
struct AsnOnly<'a> {
    #[serde(borrow, default)]
    asn: MmdbAsn<'a>,
}

#[derive(Deserialize, Default)]
struct CountryOnly<'a> {
    #[serde(borrow, default)]
    country: MmdbCountry<'a>,
}

#[derive(Deserialize, Default)]
struct CityOnly<'a> {
    #[serde(borrow, default)]
    city: MmdbCity<'a>,
}

pub fn lookup_proxy(db: &DbReader, ip: IpAddr) -> Option<ProxyInfo> {
    let result = db.lookup(ip).ok()?;
    let record: ProxyOnly = result.decode().ok()??;
    Some(ProxyInfo {
        is_proxy: record.proxy.is_proxy,
        is_vpn: record.proxy.is_vpn,
        is_tor: record.proxy.is_tor,
        is_hosting: record.proxy.is_hosting,
        is_cdn: record.proxy.is_cdn,
        is_school: record.proxy.is_school,
        is_anonymous: record.proxy.is_anonymous,
    })
}

pub fn lookup_asn_number(db: &DbReader, ip: IpAddr) -> Option<u32> {
    let result = db.lookup(ip).ok()?;
    let record: AsnOnly<'_> = result.decode().ok()??;
    record.asn.autonomous_system_number
}

pub fn lookup_asn_org(db: &DbReader, ip: IpAddr) -> Option<String> {
    let result = db.lookup(ip).ok()?;
    let record: AsnOnly<'_> = result.decode().ok()??;
    record.asn.autonomous_system_organization.map(str::to_owned)
}

pub fn lookup_country_name(db: &DbReader, ip: IpAddr) -> Option<String> {
    let result = db.lookup(ip).ok()?;
    let record: CountryOnly<'_> = result.decode().ok()??;
    record.country.names.english.map(str::to_owned)
}

pub fn lookup_city_name(db: &DbReader, ip: IpAddr) -> Option<String> {
    let result = db.lookup(ip).ok()?;
    let record: CityOnly<'_> = result.decode().ok()??;
    record.city.names.english.map(str::to_owned)
}
