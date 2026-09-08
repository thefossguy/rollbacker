use crate::systemd_generator::SYSTEMD_GENERATOR_DIR;

use std::error::Error;
use std::fs;

use tfg_helpers::make_formatted_error;

const DEFAULT_FSTAB: &str = "/etc/fstab";
const DEFAULT_MOUNT_PATH: &str = "/mnt/rollbacker";
const DEFAULT_FRESH_SNAPSHOT_SUFFIX: &str = "@:fresh";

#[derive(Debug)]
pub struct SystemdRollbackerGeneratorConfig {
    pub fstab: String,
    pub mount_path: String,
    pub fresh_snapshot_suffix: String,
}

pub fn configure() -> Result<SystemdRollbackerGeneratorConfig, Box<dyn Error>> {
    use lexopt::prelude::*;

    let mut fstab = DEFAULT_FSTAB.to_string();
    let mut mount_path = DEFAULT_MOUNT_PATH.to_string();
    let mut fresh_snapshot_suffix = DEFAULT_FRESH_SNAPSHOT_SUFFIX.to_string();

    let mut lexopt_parser = lexopt::Parser::from_env();
    while let Some(arg) = lexopt_parser.next()? {
        match arg {
            Long("fstab") => {
                let specified_fstab = lexopt_parser.value()?.string()?;
                match fs::canonicalize(&specified_fstab) {
                    Err(e) => return Err(format!(
                        "Could not canonicalize the path of specified fstab '{specified_fstab}'{}",
                        make_formatted_error!(e)
                    )
                    .into()),
                    Ok(canonicalized_fstab) => fstab = canonicalized_fstab.display().to_string(),
                }
            }

            Long("mount-path") => mount_path = lexopt_parser.value()?.string()?,

            Long("fresh-snapshot-suffix") => {
                fresh_snapshot_suffix = lexopt_parser.value()?.string()?;
            }

            _ => return Err(arg.unexpected().into()),
        }
    }

    if let Err(e) = fs::create_dir_all(SYSTEMD_GENERATOR_DIR) {
        Err(format!("Could not create the directory '{SYSTEMD_GENERATOR_DIR}' to store systemd unit files{}", make_formatted_error!(e)).into())
    } else {
        Ok(SystemdRollbackerGeneratorConfig {
            fstab,
            mount_path,
            fresh_snapshot_suffix,
        })
    }
}
