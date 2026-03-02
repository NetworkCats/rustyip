use std::net::IpAddr;
use std::path::Path;
use std::sync::Arc;

use arc_swap::ArcSwap;
use maxminddb::Reader;

use crate::models::{from_mmdb_record, IpInfo, MmdbRecord};

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
