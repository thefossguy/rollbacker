use crate::config::{FilesystemType, RollbackerConfig};

use std::collections::HashSet;
use std::error::Error;
use std::process::Command;

use tfg_helpers::command_helpers::CommandOutputStatus;
use tfg_helpers::{get_process_stderr, get_process_stdout, log_then_output};

fn discover_btrfs_subvolumes(
    rollbacker_config: &RollbackerConfig,
) -> Result<HashSet<String>, Box<dyn Error>> {
    let mut discover_all_btrfs_subvolumes_cmd = Command::new("btrfs");
    discover_all_btrfs_subvolumes_cmd.args([
        "subvolume",
        "list",
        "-a",
        &rollbacker_config.superblock_path,
    ]);
    let discover_all_btrfs_subvolumes_process_result =
        log_then_output!(discover_all_btrfs_subvolumes_cmd);
    if (&discover_all_btrfs_subvolumes_process_result).was_process_successful() {
        let discover_all_btrfs_subvolumes_process_output =
            discover_all_btrfs_subvolumes_process_result?;
        let discover_all_btrfs_subvolumes_process_stdout =
            get_process_stdout!(discover_all_btrfs_subvolumes_process_output);
        let mut all_btrfs_subvolumes = HashSet::new();
        for btrfs_subvolume_list_line in discover_all_btrfs_subvolumes_process_stdout.lines() {
            let split_btrfs_subvolume_list_line = btrfs_subvolume_list_line
                .split_whitespace()
                .collect::<Vec<&str>>();
            let btrfs_subvolume_absolute_path = if let Some(btrfs_subvolume_absolute_path) =
                split_btrfs_subvolume_list_line.last()
            {
                if btrfs_subvolume_absolute_path.starts_with("<FS_TREE>/") {
                    btrfs_subvolume_absolute_path
                        .chars()
                        .skip(10)
                        .collect::<String>()
                } else {
                    btrfs_subvolume_absolute_path.to_string()
                }
            } else {
                return Err(format!("Could not determine the btrfs subvolume's absolute path from line '{btrfs_subvolume_list_line}'").into());
            };
            let btrfs_subvolume_absolute_path = format!(
                "{}/{}",
                rollbacker_config.superblock_path, btrfs_subvolume_absolute_path
            );
            all_btrfs_subvolumes.insert(btrfs_subvolume_absolute_path);
        }

        Ok(all_btrfs_subvolumes)
    } else {
        let discover_all_btrfs_subvolumes_process_output =
            discover_all_btrfs_subvolumes_process_result?;
        Err(format!(
            "The command to discover all btrfs subvolumes at '{}' failed{}",
            rollbacker_config.superblock_path,
            get_process_stderr!(discover_all_btrfs_subvolumes_process_output)
        )
        .into())
    }
}

fn rollback_btrfs_subvolume_inner(
    btrfs_subvolume_name: &str,
    fresh_snapshot_name: &str,
) -> Result<(), Box<dyn Error>> {
    let mut btrfs_subvolume_delete_cmd = Command::new("btrfs");
    btrfs_subvolume_delete_cmd.args(["subvolume", "delete", btrfs_subvolume_name]);
    let btrfs_subvolume_delete_process_result = log_then_output!(btrfs_subvolume_delete_cmd);
    if (&btrfs_subvolume_delete_process_result).was_process_successful() {
        let mut btrfs_subvolume_snapshot_cmd = Command::new("btrfs");
        btrfs_subvolume_snapshot_cmd.args([
            "subvolume",
            "snapshot",
            fresh_snapshot_name,
            btrfs_subvolume_name,
        ]);
        let btrfs_subvolume_snapshot_process_result =
            log_then_output!(btrfs_subvolume_snapshot_cmd);
        if (&btrfs_subvolume_snapshot_process_result).was_process_successful() {
            Ok(())
        } else {
            let btrfs_subvolume_snapshot_process_output = btrfs_subvolume_snapshot_process_result?;
            Err(format!("The command to restore subvolume '{btrfs_subvolume_name}' with the fresh snapshot '{fresh_snapshot_name}' failed{}", get_process_stderr!(btrfs_subvolume_snapshot_process_output)).into())
        }
    } else {
        let btrfs_subvolume_delete_process_output = btrfs_subvolume_delete_process_result?;
        Err(format!(
            "The command to delete the subvolume '{btrfs_subvolume_name}' failed{}",
            get_process_stderr!(btrfs_subvolume_delete_process_output)
        )
        .into())
    }
}

fn rollback_btrfs_subvolume(rollbacker_config: &RollbackerConfig) -> Result<(), Box<dyn Error>> {
    let all_btrfs_subvolumes = discover_btrfs_subvolumes(rollbacker_config)?;
    for btrfs_subvol in &all_btrfs_subvolumes {
        if btrfs_subvol.ends_with(&rollbacker_config.fresh_snapshot_suffix) {
            let matching_subvolume_name = btrfs_subvol
                .chars()
                .take(rollbacker_config.fresh_snapshot_suffix.len() + 1)
                .collect::<String>();
            if all_btrfs_subvolumes.contains(&matching_subvolume_name) {
                rollback_btrfs_subvolume_inner(&matching_subvolume_name, btrfs_subvol)?;
            }
        }
    }

    Ok(())
}

fn discover_zfs_datasets_and_snapshots(
    rollbacker_config: &RollbackerConfig,
) -> Result<HashSet<String>, Box<dyn Error>> {
    let mut discover_all_zfs_datasets_and_snapshots_cmd = Command::new("zfs");
    discover_all_zfs_datasets_and_snapshots_cmd.args([
        "list",
        "-H",
        "-t",
        "all",
        "-r",
        &rollbacker_config.superblock_path,
    ]);
    let discover_all_zfs_datasets_and_snapshots_process_result =
        log_then_output!(discover_all_zfs_datasets_and_snapshots_cmd);
    if (&discover_all_zfs_datasets_and_snapshots_process_result).was_process_successful() {
        let discover_all_zfs_datasets_and_snapshots_process_output =
            discover_all_zfs_datasets_and_snapshots_process_result?;
        let discover_all_zfs_datasets_and_snapshots_process_stdout =
            get_process_stdout!(discover_all_zfs_datasets_and_snapshots_process_output);
        let mut all_zfs_datasets_and_snapshots = HashSet::new();
        for zfs_superblock_entry_line in
            discover_all_zfs_datasets_and_snapshots_process_stdout.lines()
        {
            let split_zfs_superblock_entry_line = zfs_superblock_entry_line
                .split_whitespace()
                .collect::<Vec<&str>>();
            let zfs_superblock = if let Some(zfs_superblock) =
                split_zfs_superblock_entry_line.first()
            {
                zfs_superblock.to_string()
            } else {
                return Err(format!("Could not determine the zfs superblock's name from line '{zfs_superblock_entry_line}'").into());
            };
            if zfs_superblock == rollbacker_config.superblock_path {
                continue;
            }
            all_zfs_datasets_and_snapshots.insert(zfs_superblock);
        }
        Ok(all_zfs_datasets_and_snapshots)
    } else {
        let discover_all_zfs_datasets_and_snapshots_process_output =
            discover_all_zfs_datasets_and_snapshots_process_result?;
        Err(format!(
            "The command to list all zfs datasets and snapshots of zpool '{}' failed{}",
            rollbacker_config.superblock_path,
            get_process_stderr!(discover_all_zfs_datasets_and_snapshots_process_output)
        )
        .into())
    }
}

fn rollback_zfs_dataset_inner(
    zfs_dataset_name: &str,
    fresh_snapshot_name: &str,
) -> Result<(), Box<dyn Error>> {
    let mut zfs_rollback_cmd = Command::new("zfs");
    zfs_rollback_cmd.args(["rollback", "-r", fresh_snapshot_name]);
    let zfs_rollback_process_result = log_then_output!(zfs_rollback_cmd);
    if (&zfs_rollback_process_result).was_process_successful() {
        Ok(())
    } else {
        let zfs_rollback_process_output = zfs_rollback_process_result?;
        Err(format!("The command to rollback the zfs dataset '{zfs_dataset_name}' using the snapshot '{fresh_snapshot_name}' failed{}", get_process_stderr!(zfs_rollback_process_output)).into())
    }
}

fn rollback_zfs_dataset(rollbacker_config: &RollbackerConfig) -> Result<(), Box<dyn Error>> {
    let all_zfs_datasets_and_snapshots = discover_zfs_datasets_and_snapshots(rollbacker_config)?;
    for zfs_dataset_or_snapshot in &all_zfs_datasets_and_snapshots {
        if zfs_dataset_or_snapshot.ends_with(&rollbacker_config.fresh_snapshot_suffix) {
            let matching_zfs_dataset_name = zfs_dataset_or_snapshot
                .chars()
                .take(rollbacker_config.fresh_snapshot_suffix.len() + 1)
                .collect::<String>();
            if all_zfs_datasets_and_snapshots.contains(&matching_zfs_dataset_name) {
                rollback_zfs_dataset_inner(&matching_zfs_dataset_name, zfs_dataset_or_snapshot)?;
            }
        }
    }
    Ok(())
}

pub fn rollback_filesystem(rollbacker_config: &RollbackerConfig) -> Result<(), Box<dyn Error>> {
    match rollbacker_config.filesystem_type {
        FilesystemType::Btrfs => rollback_btrfs_subvolume(rollbacker_config),
        FilesystemType::Zfs => rollback_zfs_dataset(rollbacker_config),
    }
}
