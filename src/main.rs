mod config;
mod db;
mod models;
mod updater;

fn main() {
    let _config = config::Config::from_env();
}
