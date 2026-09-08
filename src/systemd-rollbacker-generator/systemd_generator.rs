use crate::config::SystemdRollbackerGeneratorConfig;
use crate::fstab::{DeviceMountPaths, FilesystemType, ParsedFstab, SuperblockEntry};

use std::error::Error;
use std::fs;

use tfg_helpers::make_formatted_error;

pub const SYSTEMD_GENERATOR_DIR: &str = "/run/systemd/generator";

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
) -> Result<(), Box<dyn Error>> {
    let mount_units = convert_device_mount_paths_to_mount_units(device_mount_paths);
    let mount_path = superblock_entry.mount_path.clone().unwrap();
    let superblock_mount_base_unit_name = systemd_escape(&mount_path);
    let superblock_mount_base_unit =
        format!("{SYSTEMD_GENERATOR_DIR}/{superblock_mount_base_unit_name}");

    let superblock_mount_unit_path = format!("{superblock_mount_base_unit}.mount");
    let superblock_mount_unit_contents = strip_multi_line_prefix(
        format!(
            "
        [Unit]
        Requires={}.device

        [Mount]
        What={}
        Where={}
        Type={}
        Options=subvolid=5
            ",
            systemd_escape(&superblock_entry.device),
            superblock_entry.device,
            mount_path,
            superblock_entry.filesystem_type,
        )
        .as_str(),
    );
    if let Err(e) = fs::write(&superblock_mount_unit_path, superblock_mount_unit_contents) {
        return Err(format!(
            "Could not write to {superblock_mount_unit_path}{}",
            make_formatted_error!(e)
        )
        .into());
    }

    let rollback_service_unit_path = format!("{superblock_mount_base_unit}.service");
    let rollback_service_unit_contents = strip_multi_line_prefix(format!("
        [Unit]
        Requires={superblock_mount_base_unit_name}.mount
        After={superblock_mount_base_unit_name}.mount
        Before=sysroot.mount {mount_units}
        RequiredBy=sysroot.mount {mount_units}

        [Service]
        Type=oneshot
        RemainAfterExit=yes
        ExecStart=/usr/bin/env bash -c 'rollbacker --filesystem-type {} --fresh-snapshot-suffix {} --superblock-path {mount_path} && systemctl stop {superblock_mount_unit_path}'
        ", superblock_entry.filesystem_type, systemd_rollbacker_generator_config.fresh_snapshot_suffix).as_str());
    if let Err(e) = fs::write(&rollback_service_unit_path, rollback_service_unit_contents) {
        return Err(format!(
            "Could not write to {rollback_service_unit_path}{}",
            make_formatted_error!(e)
        )
        .into());
    }

    Ok(())
}
fn generate_systemd_zfs_services(
    systemd_rollbacker_generator_config: &SystemdRollbackerGeneratorConfig,
    superblock_entry: &SuperblockEntry,
    device_mount_paths: &DeviceMountPaths,
) -> Result<(), Box<dyn Error>> {
    let mount_units = convert_device_mount_paths_to_mount_units(device_mount_paths);
    let mount_path = superblock_entry.device.clone();
    let superblock_mount_base_unit =
        format!("{SYSTEMD_GENERATOR_DIR}/{}", systemd_escape(&mount_path));

    let rollback_service_unit_path = format!("{superblock_mount_base_unit}.service");
    let rollback_service_unit_contents = strip_multi_line_prefix(format!("
        [Unit]
        Requires=zfs-import-{}.service
        After=zfs-import-{}.service
        RequiredBy=zfs-import.target {mount_units}
        Before=zfs-import.target {mount_units}

        [Service]
        Type=oneshot
        RemainAfterExit=yes
        ExecStart=rollbacker --filesystem-type {} --fresh-snapshot-suffix {} --superblock-path {mount_path}
        ", superblock_entry.device, superblock_entry.device, superblock_entry.filesystem_type, systemd_rollbacker_generator_config.fresh_snapshot_suffix).as_str());
    if let Err(e) = fs::write(&rollback_service_unit_path, rollback_service_unit_contents) {
        return Err(format!(
            "Could not write to {rollback_service_unit_path}{}",
            make_formatted_error!(e)
        )
        .into());
    }
    Ok(())
}

pub fn generate_systemd_services(
    systemd_rollbacker_generator_config: &SystemdRollbackerGeneratorConfig,
    parsed_fstab: &ParsedFstab,
) -> Result<(), Box<dyn Error>> {
    for (superblock_entry, device_mount_paths) in parsed_fstab {
        match superblock_entry.filesystem_type {
            FilesystemType::Btrfs => generate_systemd_btrfs_services(
                systemd_rollbacker_generator_config,
                superblock_entry,
                device_mount_paths,
            )?,
            FilesystemType::Zfs => generate_systemd_zfs_services(
                systemd_rollbacker_generator_config,
                superblock_entry,
                device_mount_paths,
            )?,
            _ => eprintln!(
                "The filesystem '{}' does not support rollbacks, generating nothing",
                superblock_entry.filesystem_type
            ),
        }
    }

    Ok(())
}
