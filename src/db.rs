use std::net::IpAddr;
use std::path::Path;
use std::sync::Arc;

use arc_swap::ArcSwap;
use maxminddb::Reader;
use serde::Deserialize;

use crate::models::{
    IpInfo, MmdbAsn, MmdbCity, MmdbCountry, MmdbProxy, MmdbRecord, ProxyInfo, from_mmdb_record,
};

pub type DbReader = Reader<Vec<u8>>;
pub type SharedDb = Arc<ArcSwap<Option<DbReader>>>;

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

pub fn lookup(db: &SharedDb, ip: IpAddr) -> Option<IpInfo> {
    let guard = db.load();
    let reader = Option::as_ref(&guard)?;
    let result = reader.lookup(ip).ok()?;
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

pub fn lookup_proxy(db: &SharedDb, ip: IpAddr) -> Option<ProxyInfo> {
    let guard = db.load();
    let reader = Option::as_ref(&guard)?;
    let result = reader.lookup(ip).ok()?;
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

pub fn lookup_asn_number(db: &SharedDb, ip: IpAddr) -> Option<u32> {
    let guard = db.load();
    let reader = Option::as_ref(&guard)?;
    let result = reader.lookup(ip).ok()?;
    let record: AsnOnly<'_> = result.decode().ok()??;
    record.asn.autonomous_system_number
}

pub fn lookup_asn_org(db: &SharedDb, ip: IpAddr) -> Option<String> {
    let guard = db.load();
    let reader = Option::as_ref(&guard)?;
    let result = reader.lookup(ip).ok()?;
    let record: AsnOnly<'_> = result.decode().ok()??;
    record.asn.autonomous_system_organization.map(str::to_owned)
}

pub fn lookup_country_name(db: &SharedDb, ip: IpAddr) -> Option<String> {
    let guard = db.load();
    let reader = Option::as_ref(&guard)?;
    let result = reader.lookup(ip).ok()?;
    let record: CountryOnly<'_> = result.decode().ok()??;
    record.country.names.english.map(str::to_owned)
}

pub fn lookup_city_name(db: &SharedDb, ip: IpAddr) -> Option<String> {
    let guard = db.load();
    let reader = Option::as_ref(&guard)?;
    let result = reader.lookup(ip).ok()?;
    let record: CityOnly<'_> = result.decode().ok()??;
    record.city.names.english.map(str::to_owned)
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
        let result = lookup(&shared, ip);
        assert!(result.is_some());
        let info = result.unwrap();
        assert_eq!(info.ip, "1.1.1.1");
    }

    #[test]
    fn lookup_returns_none_for_unroutable_ip() {
        let shared = test_shared_db();
        let ip: IpAddr = "0.0.0.0".parse().unwrap();
        let result = lookup(&shared, ip);
        assert!(result.is_none());
    }

    #[test]
    fn lookup_returns_none_when_db_not_loaded() {
        let shared = new_empty();
        let ip: IpAddr = "1.1.1.1".parse().unwrap();
        assert!(lookup(&shared, ip).is_none());
    }

    #[test]
    fn lookup_proxy_returns_data_for_known_ip() {
        let shared = test_shared_db();
        let ip: IpAddr = "45.77.77.77".parse().unwrap();
        let proxy = lookup_proxy(&shared, ip);
        assert!(proxy.is_some());
        let proxy = proxy.unwrap();
        assert!(proxy.is_proxy);
        assert!(proxy.is_anonymous);
    }

    #[test]
    fn lookup_proxy_returns_none_for_unroutable_ip() {
        let shared = test_shared_db();
        let ip: IpAddr = "0.0.0.0".parse().unwrap();
        let proxy = lookup_proxy(&shared, ip);
        assert!(proxy.is_none());
    }

    #[test]
    fn lookup_asn_number_for_known_ip() {
        let shared = test_shared_db();
        let ip: IpAddr = "1.1.1.1".parse().unwrap();
        let asn = lookup_asn_number(&shared, ip);
        assert_eq!(asn, Some(13335));
    }

    #[test]
    fn lookup_asn_number_returns_none_for_unroutable_ip() {
        let shared = test_shared_db();
        let ip: IpAddr = "0.0.0.0".parse().unwrap();
        let asn = lookup_asn_number(&shared, ip);
        assert!(asn.is_none());
    }

    #[test]
    fn lookup_asn_org_for_known_ip() {
        let shared = test_shared_db();
        let ip: IpAddr = "1.1.1.1".parse().unwrap();
        let org = lookup_asn_org(&shared, ip);
        assert!(org.is_some());
        assert_eq!(org.unwrap(), "Cloudflare, Inc.");
    }

    #[test]
    fn lookup_asn_org_returns_none_for_unroutable_ip() {
        let shared = test_shared_db();
        let ip: IpAddr = "0.0.0.0".parse().unwrap();
        let org = lookup_asn_org(&shared, ip);
        assert!(org.is_none());
    }

    #[test]
    fn lookup_country_name_for_known_ip() {
        let shared = test_shared_db();
        let ip: IpAddr = "45.77.77.77".parse().unwrap();
        let country = lookup_country_name(&shared, ip);
        assert_eq!(country.as_deref(), Some("United States"));
    }

    #[test]
    fn lookup_country_name_returns_none_for_unroutable_ip() {
        let shared = test_shared_db();
        let ip: IpAddr = "0.0.0.0".parse().unwrap();
        let country = lookup_country_name(&shared, ip);
        assert!(country.is_none());
    }

    #[test]
    fn lookup_city_name_for_known_ip() {
        let shared = test_shared_db();
        let ip: IpAddr = "45.77.77.77".parse().unwrap();
        let city = lookup_city_name(&shared, ip);
        assert_eq!(city.as_deref(), Some("Piscataway"));
    }

    #[test]
    fn lookup_city_name_returns_none_for_unroutable_ip() {
        let shared = test_shared_db();
        let ip: IpAddr = "0.0.0.0".parse().unwrap();
        let city = lookup_city_name(&shared, ip);
        assert!(city.is_none());
    }

    #[test]
    fn lookup_works_with_ipv6() {
        let shared = test_shared_db();
        let ip: IpAddr = "2606:4700:4700::1111".parse().unwrap();
        let result = lookup(&shared, ip);
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
        assert_eq!(lookup_asn_number(&shared, ip), Some(13335));
        assert!(lookup_asn_org(&shared, ip).is_some());
        assert!(lookup_proxy(&shared, ip).is_some());
    }
}
