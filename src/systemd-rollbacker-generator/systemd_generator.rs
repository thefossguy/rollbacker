use crate::config::SystemdRollbackerGeneratorConfig;
use crate::fstab::{DeviceMountPaths, FilesystemType, ParsedFstab, SuperblockEntry};

use std::error::Error;
use std::fs;
use std::process::Command;

use tfg_helpers::command_helpers::CommandOutputStatus;
use tfg_helpers::{get_process_stderr, log_then_output, make_formatted_error};

//pub const SYSTEMD_GENERATOR_DIR: &str = "/run/systemd/system";
pub const SYSTEMD_GENERATOR_DIR: &str = "/home/pratham/my-git-repos/pratham/staging/rollbacker";

fn strip_multi_line_prefix(target_string: &str) -> String {
    target_string
        .trim()
        .lines()
        .map(str::trim_start)
        .collect::<Vec<_>>()
        .join("\n")
}

fn convert_device_mount_paths_to_mount_units(device_mount_paths: &DeviceMountPaths) -> String {
    device_mount_paths
        .iter()
        .map(|device_mount_path| format!("{}.mount", systemd_escape(device_mount_path)))
        .collect::<Vec<String>>()
        .join(" ")
}

fn systemd_escape(device_path: &str) -> String {
    if device_path == "/" {
        "-".to_string()
    } else {
        device_path
            .replacen('/', "", 1)
            .replace('-', "\\x2d")
            .replace('/', "-")
    }
}

fn generate_systemd_btrfs_services(
    systemd_rollbacker_generator_config: &SystemdRollbackerGeneratorConfig,
    superblock_entry: &SuperblockEntry,
    device_mount_paths: &DeviceMountPaths,
    first_arg_bin_dir: &str,
) -> Vec<String> {
    let mut systemd_units_to_enable = Vec::new();
    let mount_path = superblock_entry.mount_path.clone().unwrap();
    let mount_path_systemd_escaped = systemd_escape(&mount_path);

    let superblock_mount_unit_name = format!("{mount_path_systemd_escaped}.mount");
    let superblock_mount_unit_path =
        format!("{SYSTEMD_GENERATOR_DIR}/{superblock_mount_unit_name}");
    let superblock_mount_unit_contents = strip_multi_line_prefix(
        format!(
            "
        [Unit]
        Requires={}.device

        [Mount]
        What={}
        Where={mount_path}
        Type={}
        Options=subvolid=5
        ",
            systemd_escape(&superblock_entry.device),
            superblock_entry.device,
            superblock_entry.filesystem_type,
        )
        .as_str(),
    );
    match fs::write(&superblock_mount_unit_path, &superblock_mount_unit_contents) {
        Ok(()) => systemd_units_to_enable.push(superblock_mount_unit_name.clone()),
        Err(e) => eprintln!(
            "ERROR: Could not write to '{superblock_mount_unit_path}'{}",
            make_formatted_error!(e)
        ),
    }

    let systemd_mount_units = convert_device_mount_paths_to_mount_units(device_mount_paths);
    let rollback_service_unit_name = format!("rollbacker-{mount_path_systemd_escaped}.service");
    let rollback_service_unit_path =
        format!("{SYSTEMD_GENERATOR_DIR}/{rollback_service_unit_name}");
    let rollback_service_unit_contents = strip_multi_line_prefix(format!("
        [Unit]
        After={superblock_mount_unit_name}
        Requires={superblock_mount_unit_name}
        Before=sysroot.mount {systemd_mount_units}

        [Service]
        Type=oneshot
        RemainAfterExit=yes
        ExecStart={first_arg_bin_dir}/rollbacker --filesystem-type {} --fresh-snapshot-suffix {} --superblock-path {mount_path}

        [Install]
        RequiredBy={systemd_mount_units}
        ", superblock_entry.filesystem_type, systemd_rollbacker_generator_config.fresh_snapshot_suffix)
        .as_str());
    match fs::write(&rollback_service_unit_path, &rollback_service_unit_contents) {
        Ok(()) => systemd_units_to_enable.push(rollback_service_unit_name),
        Err(e) => eprintln!(
            "ERROR: Could not write to '{rollback_service_unit_path}'{}",
            make_formatted_error!(e)
        ),
    }

    systemd_units_to_enable
}
fn generate_systemd_zfs_services(
    systemd_rollbacker_generator_config: &SystemdRollbackerGeneratorConfig,
    superblock_entry: &SuperblockEntry,
    device_mount_paths: &DeviceMountPaths,
    first_arg_bin_dir: &str,
) -> String {
    let zpool_name = superblock_entry.device.clone();
    let systemd_mount_units = convert_device_mount_paths_to_mount_units(device_mount_paths);

    let zfs_dataset_rollback_unit_name = format!("rollbacker-{zpool_name}.service");
    let zfs_dataset_rollback_unit_path =
        format!("{SYSTEMD_GENERATOR_DIR}/{zfs_dataset_rollback_unit_name}");
    let zfs_dataset_rollback_unit_contents = strip_multi_line_prefix(format!("
        [Unit]
        Requires=zfs-import-{zpool_name}.service
        After=zfs-import-{zpool_name}.service
        Before={systemd_mount_units}

        [Service]
        Type=oneshot
        RemainAfterExit=yes
        ExecStart={first_arg_bin_dir}/rollbacker --filesystem-type {} --fresh-snapshot-suffix {} --superblock-path {zpool_name}

        [Install]
        RequiredBy={systemd_mount_units}
        ", superblock_entry.filesystem_type, systemd_rollbacker_generator_config.fresh_snapshot_suffix)
        .as_str());
    match fs::write(
        &zfs_dataset_rollback_unit_path,
        &zfs_dataset_rollback_unit_contents,
    ) {
        Ok(()) => zfs_dataset_rollback_unit_name,
        Err(e) => {
            eprintln!(
                "ERROR: Could not write to '{zfs_dataset_rollback_unit_path}'{}",
                make_formatted_error!(e)
            );
            String::new()
        }
    }
}

pub fn generate_systemd_services(
    systemd_rollbacker_generator_config: &SystemdRollbackerGeneratorConfig,
    parsed_fstab: &ParsedFstab,
    first_arg_bin_dir: &str,
) -> Result<(), Box<dyn Error>> {
    let mut systemd_units_to_enable = vec!["enable".to_string()];
    for (superblock_entry, device_mount_paths) in parsed_fstab {
        match superblock_entry.filesystem_type {
            FilesystemType::Btrfs => {
                systemd_units_to_enable.extend(generate_systemd_btrfs_services(
                    systemd_rollbacker_generator_config,
                    superblock_entry,
                    device_mount_paths,
                    first_arg_bin_dir,
                ));
            }
            FilesystemType::Zfs => systemd_units_to_enable.push(generate_systemd_zfs_services(
                systemd_rollbacker_generator_config,
                superblock_entry,
                device_mount_paths,
                first_arg_bin_dir,
            )),
            _ => eprintln!(
                "The filesystem '{}' does not support rollbacks, generating nothing",
                superblock_entry.filesystem_type
            ),
        }
    }

    let mut systemctl_daemon_reload_cmd = Command::new("systemctl");
    systemctl_daemon_reload_cmd.arg("daemon-reload");
    let systemctl_daemon_reload_process_result = log_then_output!(systemctl_daemon_reload_cmd);
    if (&systemctl_daemon_reload_process_result).was_process_successful() {
        let mut systemctl_enable_cmd = Command::new("systemctl");
        systemctl_enable_cmd.args(systemd_units_to_enable.iter().collect::<Vec<&String>>());
        let systemctl_enable_process_result = log_then_output!(systemctl_enable_cmd);
        if (&systemctl_enable_process_result).was_process_successful() {
            Ok(())
        } else {
            let systemctl_enable_process_output = systemctl_enable_process_result?;
            Err(format!(
                "The command to enable the newly generated systemd units failed{}",
                get_process_stderr!(systemctl_enable_process_output)
            )
            .into())
        }
    } else {
        let systemctl_daemon_reload_process_output = systemctl_daemon_reload_process_result?;
        Err(format!(
            "The command to reload the systemd manager configuration failed{}",
            get_process_stderr!(systemctl_daemon_reload_process_output)
        )
        .into())
    }
}
