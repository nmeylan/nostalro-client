mod config;

use config::Config;

fn main() {
    let config = Config::load_or_default("config.json");
    println!("ragnarok-client (packetver: {})", config.packetver);
}
