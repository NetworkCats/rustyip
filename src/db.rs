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
        let loaded = shared.load();
        // Verify the shared DB is functional by doing a simple lookup
        let ip: IpAddr = "1.1.1.1".parse().unwrap();
        let result = loaded.lookup(ip);
        assert!(result.is_ok());
    }

    #[test]
    fn lookup_returns_some_for_known_ip() {
        let reader = test_reader();
        let ip: IpAddr = "1.1.1.1".parse().unwrap();
        let result = lookup(&reader, ip);
        assert!(result.is_some());
        let info = result.unwrap();
        assert_eq!(info.ip, "1.1.1.1");
    }

    #[test]
    fn lookup_returns_none_for_unroutable_ip() {
        let reader = test_reader();
        let ip: IpAddr = "0.0.0.0".parse().unwrap();
        let result = lookup(&reader, ip);
        assert!(result.is_none());
    }

    #[test]
    fn lookup_proxy_returns_data_for_known_ip() {
        let reader = test_reader();
        let ip: IpAddr = "45.77.77.77".parse().unwrap();
        let proxy = lookup_proxy(&reader, ip);
        assert!(proxy.is_some());
        let proxy = proxy.unwrap();
        assert!(proxy.is_proxy);
        assert!(proxy.is_hosting);
    }

    #[test]
    fn lookup_proxy_returns_none_for_unroutable_ip() {
        let reader = test_reader();
        let ip: IpAddr = "0.0.0.0".parse().unwrap();
        let proxy = lookup_proxy(&reader, ip);
        assert!(proxy.is_none());
    }

    #[test]
    fn lookup_asn_number_for_known_ip() {
        let reader = test_reader();
        let ip: IpAddr = "1.1.1.1".parse().unwrap();
        let asn = lookup_asn_number(&reader, ip);
        assert_eq!(asn, Some(13335));
    }

    #[test]
    fn lookup_asn_number_returns_none_for_unroutable_ip() {
        let reader = test_reader();
        let ip: IpAddr = "0.0.0.0".parse().unwrap();
        let asn = lookup_asn_number(&reader, ip);
        assert!(asn.is_none());
    }

    #[test]
    fn lookup_asn_org_for_known_ip() {
        let reader = test_reader();
        let ip: IpAddr = "1.1.1.1".parse().unwrap();
        let org = lookup_asn_org(&reader, ip);
        assert!(org.is_some());
        assert_eq!(org.unwrap(), "Cloudflare, Inc.");
    }

    #[test]
    fn lookup_asn_org_returns_none_for_unroutable_ip() {
        let reader = test_reader();
        let ip: IpAddr = "0.0.0.0".parse().unwrap();
        let org = lookup_asn_org(&reader, ip);
        assert!(org.is_none());
    }

    #[test]
    fn lookup_country_name_for_known_ip() {
        let reader = test_reader();
        let ip: IpAddr = "45.77.77.77".parse().unwrap();
        let country = lookup_country_name(&reader, ip);
        assert_eq!(country.as_deref(), Some("United States"));
    }

    #[test]
    fn lookup_country_name_returns_none_for_unroutable_ip() {
        let reader = test_reader();
        let ip: IpAddr = "0.0.0.0".parse().unwrap();
        let country = lookup_country_name(&reader, ip);
        assert!(country.is_none());
    }

    #[test]
    fn lookup_city_name_for_known_ip() {
        let reader = test_reader();
        let ip: IpAddr = "45.77.77.77".parse().unwrap();
        let city = lookup_city_name(&reader, ip);
        assert_eq!(city.as_deref(), Some("Piscataway"));
    }

    #[test]
    fn lookup_city_name_returns_none_for_unroutable_ip() {
        let reader = test_reader();
        let ip: IpAddr = "0.0.0.0".parse().unwrap();
        let city = lookup_city_name(&reader, ip);
        assert!(city.is_none());
    }

    #[test]
    fn lookup_works_with_ipv6() {
        let reader = test_reader();
        let ip: IpAddr = "2606:4700:4700::1111".parse().unwrap();
        let result = lookup(&reader, ip);
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
        let reader = test_reader();
        let ip: IpAddr = "2606:4700:4700::1111".parse().unwrap();
        assert_eq!(lookup_asn_number(&reader, ip), Some(13335));
        assert!(lookup_asn_org(&reader, ip).is_some());
        assert!(lookup_proxy(&reader, ip).is_some());
    }
}
