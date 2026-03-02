mod cli_detect;
mod config;
mod db;
mod error;
mod models;
mod updater;

fn main() {
    let _config = config::Config::from_env();
}
