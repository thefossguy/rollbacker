mod config;
mod fstab;
mod systemd_generator;

use std::error::Error;

pub const CARGO_BIN_NAME: &str = env!("CARGO_BIN_NAME");

fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args().collect::<Vec<String>>();
    let Some(first_arg) = args.first() else {
        return Err("Could not determine the 0th argv".into());
    };
    let Ok(canonicalized_first_arg) = std::fs::canonicalize(first_arg) else {
        return Err(format!("Could not canonicalize the arg0 '{first_arg}'").into());
    };
    let Some(first_arg_bin_dir) = canonicalized_first_arg.parent() else {
        return Err(format!(
            "Could not determine the parent of '{}'",
            canonicalized_first_arg.display()
        )
        .into());
    };
    let first_arg_bin_dir = first_arg_bin_dir.display().to_string();

    let systemd_rollbacker_generator_config = config::configure()?;
    eprintln!("{systemd_rollbacker_generator_config:#?}");

    let parsed_fstab = fstab::parse_fstab(&systemd_rollbacker_generator_config)?;
    eprintln!("{parsed_fstab:#?}");

    systemd_generator::generate_systemd_services(
        &systemd_rollbacker_generator_config,
        &parsed_fstab,
        &first_arg_bin_dir,
    )
}
