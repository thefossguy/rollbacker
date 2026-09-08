mod config;
mod fstab;
mod systemd_generator;

use std::error::Error;

pub const CARGO_BIN_NAME: &str = env!("CARGO_BIN_NAME");

fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args().collect::<Vec<String>>();
    let Some(arg0) = args.get(0) else {
        return Err("Could not determine the 0th argv".into());
    };
    let Ok(canonicalized_arg0) = std::fs::canonicalize(arg0) else {
        return Err(format!("Could not canonicalize the arg0 '{arg0}'").into());
    };
    let Some(arg0_bin_dir) = canonicalized_arg0.parent() else {
        return Err(format!(
            "Could not determine the parent of '{}'",
            canonicalized_arg0.display()
        )
        .into());
    };
    let arg0_bin_dir = arg0_bin_dir.display().to_string();
    eprintln!("{arg0_bin_dir}");

    let systemd_rollbacker_generator_config = config::configure()?;
    eprintln!("{systemd_rollbacker_generator_config:#?}");

    let parsed_fstab = fstab::parse_fstab(&systemd_rollbacker_generator_config)?;
    eprintln!("{parsed_fstab:#?}");

    systemd_generator::generate_systemd_services(
        &systemd_rollbacker_generator_config,
        &parsed_fstab,
        &arg0_bin_dir,
    )
}
