mod config;
mod db;
mod models;

fn main() {
    let _config = config::Config::from_env();
}
