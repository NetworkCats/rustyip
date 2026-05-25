//! MaxMind MMDB database loading, shared state, and lookup functions.

use std::net::IpAddr;
use std::path::Path;
use std::sync::Arc;

use arc_swap::ArcSwap;
use maxminddb::{MaxMindDbError, PathElement, Reader};
use serde::Deserialize;

use crate::models::{
    IpInfo, MmdbProxy, MmdbRecord, PageInfo, ProxyInfo, from_mmdb_record,
    serialize_mmdb_record_json,
};

pub type DbReader = Reader<Vec<u8>>;
pub type SharedDb = Arc<ArcSwap<Option<DbReader>>>;
pub type LookupOutcome<T> = Result<Option<T>, MaxMindDbError>;

pub fn load_db(path: &Path) -> Result<DbReader, maxminddb::MaxMindDbError> {
    Reader::open_readfile(path)
}

pub fn new_shared(reader: DbReader) -> SharedDb {
    Arc::new(ArcSwap::from_pointee(Some(reader)))
}

pub fn new_empty() -> SharedDb {
    Arc::new(ArcSwap::from_pointee(None))
}

pub fn is_ready(db: &SharedDb) -> bool {
    db.load().is_some()
}

pub fn lookup(db: &SharedDb, ip: IpAddr) -> LookupOutcome<IpInfo> {
    with_reader(db, ip, |result| {
        let Some(record) = result.decode::<MmdbRecord<'_>>()? else {
            return Ok(None);
        };
        Ok(Some(from_mmdb_record(ip, &record)))
    })
}

pub fn lookup_json(db: &SharedDb, ip: IpAddr) -> LookupOutcome<Vec<u8>> {
    with_reader(db, ip, |result| {
        let Some(record) = result.decode::<MmdbRecord<'_>>()? else {
            return Ok(None);
        };
        Ok(Some(serialize_mmdb_record_json(ip, &record)))
    })
}

/// Loads the shared reader, performs a lookup, and passes the raw lookup result
/// to `f` for decoding. This eliminates the repeated load-guard-lookup boilerplate
/// while preserving lookup/decode errors for callers that need HTTP error mapping.
fn with_reader<T>(
    db: &SharedDb,
    ip: IpAddr,
    f: impl FnOnce(maxminddb::LookupResult<'_, Vec<u8>>) -> LookupOutcome<T>,
) -> LookupOutcome<T> {
    let guard = db.load();
    let Some(reader) = Option::as_ref(&guard) else {
        return Ok(None);
    };
    let result = reader.lookup(ip)?;
    f(result)
}

fn lookup_path<T>(db: &SharedDb, ip: IpAddr, path: &[PathElement<'_>]) -> LookupOutcome<T>
where
    T: for<'de> Deserialize<'de>,
{
    with_reader(db, ip, |result| result.decode_path(path))
}

// Lightweight deserialization struct for the full proxy object.
// Single-flag endpoints use `decode_path` below instead.

#[derive(Deserialize, Default)]
struct ProxyOnly {
    #[serde(default)]
    proxy: MmdbProxy,
}

#[derive(Deserialize, Default)]
struct PageRecord<'a> {
    #[serde(borrow, default)]
    city: crate::models::MmdbCity<'a>,
    #[serde(default)]
    country: crate::models::MmdbCountry<'a>,
    #[serde(default)]
    asn: crate::models::MmdbAsn<'a>,
    #[serde(default)]
    proxy: MmdbProxy,
}

pub fn lookup_proxy(db: &SharedDb, ip: IpAddr) -> LookupOutcome<ProxyInfo> {
    with_reader(db, ip, |result| {
        let Some(record) = result.decode::<ProxyOnly>()? else {
            return Ok(None);
        };
        Ok(Some(record.proxy.into()))
    })
}

pub fn lookup_page_info(db: &SharedDb, ip: IpAddr, mmdb_key: &str) -> LookupOutcome<PageInfo> {
    with_reader(db, ip, |result| {
        let Some(record) = result.decode::<PageRecord<'_>>()? else {
            return Ok(None);
        };

        let asn_number = record.asn.autonomous_system_number;
        let asn = asn_number.map(|n| format!("AS{n}")).unwrap_or_default();

        Ok(Some(PageInfo {
            ip: ip.to_string(),
            asn,
            asn_number,
            org: record
                .asn
                .autonomous_system_organization
                .unwrap_or_default()
                .to_owned(),
            country: localized_mmdb_name(&record.country.names, mmdb_key).to_owned(),
            country_code: record.country.iso_code.unwrap_or_default().to_owned(),
            city: localized_mmdb_name(&record.city.names, mmdb_key).to_owned(),
            proxy: record.proxy.into(),
        }))
    })
}

fn localized_mmdb_name<'a>(names: &'a crate::models::MmdbNames<'a>, mmdb_key: &str) -> &'a str {
    match mmdb_key {
        "de" => names.german.or(names.english),
        "es" => names.spanish.or(names.english),
        "fr" => names.french.or(names.english),
        "ja" => names.japanese.or(names.english),
        "pt-BR" => names.brazilian_portuguese.or(names.english),
        "ru" => names.russian.or(names.english),
        "zh-CN" => names.simplified_chinese.or(names.english),
        _ => names.english,
    }
    .unwrap_or_default()
}

fn lookup_proxy_flag(db: &SharedDb, ip: IpAddr, field: &'static str) -> LookupOutcome<bool> {
    lookup_path(
        db,
        ip,
        &[PathElement::Key("proxy"), PathElement::Key(field)],
    )
}

pub(crate) fn lookup_is_proxy(db: &SharedDb, ip: IpAddr) -> LookupOutcome<bool> {
    lookup_proxy_flag(db, ip, "is_proxy")
}

pub(crate) fn lookup_is_vpn(db: &SharedDb, ip: IpAddr) -> LookupOutcome<bool> {
    lookup_proxy_flag(db, ip, "is_vpn")
}

pub(crate) fn lookup_is_hosting(db: &SharedDb, ip: IpAddr) -> LookupOutcome<bool> {
    lookup_proxy_flag(db, ip, "is_hosting")
}

pub(crate) fn lookup_is_tor(db: &SharedDb, ip: IpAddr) -> LookupOutcome<bool> {
    lookup_proxy_flag(db, ip, "is_tor")
}

pub fn lookup_asn_number(db: &SharedDb, ip: IpAddr) -> LookupOutcome<u32> {
    lookup_path(
        db,
        ip,
        &[
            PathElement::Key("asn"),
            PathElement::Key("autonomous_system_number"),
        ],
    )
}

pub fn lookup_asn_org(db: &SharedDb, ip: IpAddr) -> LookupOutcome<String> {
    lookup_path(
        db,
        ip,
        &[
            PathElement::Key("asn"),
            PathElement::Key("autonomous_system_organization"),
        ],
    )
}

pub fn lookup_country_name(db: &SharedDb, ip: IpAddr) -> LookupOutcome<String> {
    lookup_path(
        db,
        ip,
        &[
            PathElement::Key("country"),
            PathElement::Key("names"),
            PathElement::Key("en"),
        ],
    )
}

pub fn lookup_city_name(db: &SharedDb, ip: IpAddr) -> LookupOutcome<String> {
    lookup_path(
        db,
        ip,
        &[
            PathElement::Key("city"),
            PathElement::Key("names"),
            PathElement::Key("en"),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn test_db_path() -> String {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        format!("{manifest_dir}/data/Merged-IP.mmdb")
    }

    fn test_reader() -> DbReader {
        load_db(Path::new(&test_db_path())).expect("failed to load test DB")
    }

    fn test_shared_db() -> SharedDb {
        new_shared(test_reader())
    }

    #[test]
    fn load_db_with_valid_path() {
        let reader = load_db(Path::new(&test_db_path()));
        assert!(reader.is_ok());
    }

    #[test]
    fn load_db_with_invalid_path() {
        let reader = load_db(Path::new("/nonexistent/path.mmdb"));
        assert!(reader.is_err());
    }

    #[test]
    fn new_shared_creates_valid_shared_db() {
        let reader = test_reader();
        let shared = new_shared(reader);
        assert!(is_ready(&shared));
        let guard = shared.load();
        let reader = Option::as_ref(&guard).unwrap();
        let ip: IpAddr = "1.1.1.1".parse().unwrap();
        let result = reader.lookup(ip);
        assert!(result.is_ok());
    }

    #[test]
    fn new_empty_creates_unready_shared_db() {
        let shared = new_empty();
        assert!(!is_ready(&shared));
    }

    #[test]
    fn lookup_returns_some_for_known_ip() {
        let shared = test_shared_db();
        let ip: IpAddr = "1.1.1.1".parse().unwrap();
        let result = lookup(&shared, ip).unwrap();
        assert!(result.is_some());
        let info = result.unwrap();
        assert_eq!(info.ip, "1.1.1.1");
    }

    #[test]
    fn lookup_returns_none_for_unroutable_ip() {
        let shared = test_shared_db();
        let ip: IpAddr = "0.0.0.0".parse().unwrap();
        let result = lookup(&shared, ip).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn lookup_returns_none_when_db_not_loaded() {
        let shared = new_empty();
        let ip: IpAddr = "1.1.1.1".parse().unwrap();
        assert!(lookup(&shared, ip).unwrap().is_none());
    }

    #[test]
    fn lookup_json_matches_owned_ip_info_for_ipv4() {
        assert_json_lookup_matches_ip_info("45.77.77.77");
    }

    #[test]
    fn lookup_json_matches_owned_ip_info_for_ipv6() {
        assert_json_lookup_matches_ip_info("2606:4700:4700::1111");
    }

    fn assert_json_lookup_matches_ip_info(ip: &str) {
        let shared = test_shared_db();
        let ip: IpAddr = ip.parse().unwrap();
        let info = lookup(&shared, ip).unwrap().unwrap();
        let json = lookup_json(&shared, ip).unwrap().unwrap();
        let expected = serde_json::to_value(info).unwrap();
        let actual: serde_json::Value = serde_json::from_slice(&json).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn lookup_json_returns_none_when_db_not_loaded() {
        let shared = new_empty();
        let ip: IpAddr = "1.1.1.1".parse().unwrap();
        assert!(lookup_json(&shared, ip).unwrap().is_none());
    }

    #[test]
    fn lookup_page_info_returns_rendered_fields() {
        let shared = test_shared_db();
        let ip: IpAddr = "45.77.77.77".parse().unwrap();
        let info = lookup_page_info(&shared, ip, "en").unwrap().unwrap();
        assert_eq!(info.ip, "45.77.77.77");
        assert_eq!(info.asn, "AS20473");
        assert_eq!(info.asn_number, Some(20473));
        assert_eq!(info.org, "The Constant Company, LLC");
        assert_eq!(info.country, "United States");
        assert_eq!(info.country_code, "US");
        assert_eq!(info.city, "Piscataway");
    }

    #[test]
    fn lookup_proxy_returns_none_for_unroutable_ip() {
        let shared = test_shared_db();
        let ip: IpAddr = "0.0.0.0".parse().unwrap();
        let proxy = lookup_proxy(&shared, ip).unwrap();
        assert!(proxy.is_none());
    }

    #[test]
    fn lookup_asn_number_for_known_ip() {
        let shared = test_shared_db();
        let ip: IpAddr = "1.1.1.1".parse().unwrap();
        let asn = lookup_asn_number(&shared, ip).unwrap();
        assert_eq!(asn, Some(13335));
    }

    #[test]
    fn lookup_asn_number_returns_none_for_unroutable_ip() {
        let shared = test_shared_db();
        let ip: IpAddr = "0.0.0.0".parse().unwrap();
        let asn = lookup_asn_number(&shared, ip).unwrap();
        assert!(asn.is_none());
    }

    #[test]
    fn lookup_asn_org_for_known_ip() {
        let shared = test_shared_db();
        let ip: IpAddr = "1.1.1.1".parse().unwrap();
        let org = lookup_asn_org(&shared, ip).unwrap();
        assert!(org.is_some());
        assert_eq!(org.unwrap(), "Cloudflare, Inc.");
    }

    #[test]
    fn lookup_asn_org_returns_none_for_unroutable_ip() {
        let shared = test_shared_db();
        let ip: IpAddr = "0.0.0.0".parse().unwrap();
        let org = lookup_asn_org(&shared, ip).unwrap();
        assert!(org.is_none());
    }

    #[test]
    fn lookup_country_name_for_known_ip() {
        let shared = test_shared_db();
        let ip: IpAddr = "45.77.77.77".parse().unwrap();
        let country = lookup_country_name(&shared, ip).unwrap();
        assert_eq!(country.as_deref(), Some("United States"));
    }

    #[test]
    fn lookup_country_name_returns_none_for_unroutable_ip() {
        let shared = test_shared_db();
        let ip: IpAddr = "0.0.0.0".parse().unwrap();
        let country = lookup_country_name(&shared, ip).unwrap();
        assert!(country.is_none());
    }

    #[test]
    fn lookup_city_name_for_known_ip() {
        let shared = test_shared_db();
        let ip: IpAddr = "45.77.77.77".parse().unwrap();
        let city = lookup_city_name(&shared, ip).unwrap();
        assert_eq!(city.as_deref(), Some("Piscataway"));
    }

    #[test]
    fn lookup_city_name_returns_none_for_unroutable_ip() {
        let shared = test_shared_db();
        let ip: IpAddr = "0.0.0.0".parse().unwrap();
        let city = lookup_city_name(&shared, ip).unwrap();
        assert!(city.is_none());
    }

    #[test]
    fn lookup_works_with_ipv6() {
        let shared = test_shared_db();
        let ip: IpAddr = "2606:4700:4700::1111".parse().unwrap();
        let result = lookup(&shared, ip).unwrap();
        assert!(result.is_some());
        let info = result.unwrap();
        assert_eq!(info.ip, "2606:4700:4700::1111");
        assert!(info.asn.is_some());
        assert_eq!(
            info.asn.as_ref().unwrap().autonomous_system_number,
            Some(13335)
        );
    }

    #[test]
    fn partial_lookups_work_with_ipv6() {
        let shared = test_shared_db();
        let ip: IpAddr = "2606:4700:4700::1111".parse().unwrap();
        assert_eq!(lookup_asn_number(&shared, ip).unwrap(), Some(13335));
        assert!(lookup_asn_org(&shared, ip).unwrap().is_some());
        assert!(lookup_proxy(&shared, ip).unwrap().is_some());
    }
}
