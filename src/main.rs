mod cli_detect;
mod config;
mod db;
mod error;
mod handlers;
mod models;
mod routes;
mod updater;

fn main() {
    let _config = config::Config::from_env();
}
