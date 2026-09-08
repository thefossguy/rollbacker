mod config;
mod fstab;
mod systemd_generator;

use std::error::Error;

pub const CARGO_BIN_NAME: &str = env!("CARGO_BIN_NAME");

fn main() -> Result<(), Box<dyn Error>> {
    let systemd_rollbacker_generator_config = config::configure()?;
    eprintln!("{systemd_rollbacker_generator_config:#?}");

    let parsed_fstab = fstab::parse_fstab(&systemd_rollbacker_generator_config)?;
    eprintln!("{parsed_fstab:#?}");

    systemd_generator::generate_systemd_services(
        &systemd_rollbacker_generator_config,
        &parsed_fstab,
    )
}
