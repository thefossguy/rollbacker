use crate::config::SystemdRollbackerGeneratorConfig;

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use tfg_helpers::make_formatted_error;

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub enum FilesystemType {
    Btrfs,
    Ext4,
    Fat,
    Swap,
    Xfs,
    Zfs,
}
impl FilesystemType {
    fn from(filesystem_type: &str) -> Result<Self, Box<dyn Error>> {
        match filesystem_type {
            "btrfs" => Ok(Self::Btrfs),
            "ext4" => Ok(Self::Ext4),
            "fat" | "vfat" => Ok(Self::Fat),
            "swap" => Ok(Self::Swap),
            "xfs" => Ok(Self::Xfs),
            "zfs" => Ok(Self::Zfs),
            _ => Err(format!("The filesystem type '{filesystem_type}' is not supported").into()),
        }
    }
}
impl fmt::Debug for FilesystemType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Btrfs => write!(f, "btrfs"),
            Self::Ext4 => write!(f, "ext4"),
            Self::Fat => write!(f, "fat"),
            Self::Swap => write!(f, "swap"),
            Self::Xfs => write!(f, "xfs"),
            Self::Zfs => write!(f, "zfs"),
        }
    }
}
impl fmt::Display for FilesystemType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct SuperblockEntry {
    pub device: String,
    pub filesystem_type: FilesystemType,
    pub mount_path: Option<String>,
}
impl SuperblockEntry {
    fn from(
        fstab_entry: &str,
        systemd_rollbacker_generator_config: &SystemdRollbackerGeneratorConfig,
    ) -> Result<(Self, String), Box<dyn Error>> {
        let fstab_entry_fields = fstab_entry.split_whitespace().collect::<Vec<&str>>();

        let device = if let Some(device) = fstab_entry_fields.first() {
            match *device {
                device if device.starts_with("LABEL=") => format!(
                    "/dev/disk/by-label/{}",
                    device.chars().skip(6).collect::<String>()
                ),
                device if device.starts_with("PARTLABEL=") => format!(
                    "/dev/disk/by-partlabel/{}",
                    device.chars().skip(10).collect::<String>()
                ),
                device if device.starts_with("PARTUUID=") => format!(
                    "/dev/disk/by-partuuid/{}",
                    device.chars().skip(9).collect::<String>()
                ),
                device if device.starts_with("UUID=") => format!(
                    "/dev/disk/by-uuid/{}",
                    device.chars().skip(5).collect::<String>()
                ),
                // Now it is either an actual block dev or a zpool or something else. If it is
                // something we don't support (NFS share, pseudo filesystem, etc), we anyways don't
                // make use of it.
                _ => device.to_string(),
            }
        } else {
            return Err(format!("Could not determine the \"device\" field's value from the fstab entry '{fstab_entry}'").into());
        };

        let device_parts = device.split('/').collect::<Vec<&str>>();

        let filesystem_type = if let Some(filesystem_type_str) = fstab_entry_fields.get(2) {
            FilesystemType::from(filesystem_type_str)?
        } else {
            return Err(format!("Could not determine the \"filesystem type\" field's value from the fstab entry '{fstab_entry}'").into());
        };

        let mount_path = if filesystem_type == FilesystemType::Zfs {
            None
        } else if let Some(device_part_last) = device_parts.last() {
            Some(format!(
                "{}/{}",
                systemd_rollbacker_generator_config.mount_path, device_part_last
            ))
        } else {
            return Err(format!("Could not determine the device's '{device}' final part").into());
        };

        let device = if filesystem_type == FilesystemType::Zfs
            && let Some(zpool_name) = device_parts.first()
        {
            zpool_name.to_string()
        } else {
            device
        };

        let device_mount_path = if let Some(device_mount_path) = fstab_entry_fields.get(1) {
            device_mount_path.to_string()
        } else {
            return Err(format!("Could not determine the \"mount path\" field's value from the fstab entry '{fstab_entry}'").into());
        };

        Ok((
            Self {
                device,
                filesystem_type,
                mount_path,
            },
            device_mount_path,
        ))
    }
}

pub type DeviceMountPaths = Vec<String>;
pub type ParsedFstab = HashMap<SuperblockEntry, DeviceMountPaths>;

pub fn parse_fstab(
    systemd_rollbacker_generator_config: &SystemdRollbackerGeneratorConfig,
) -> Result<ParsedFstab, Box<dyn Error>> {
    let mut parsed_fstab = HashMap::new();
    match std::fs::read_to_string(&systemd_rollbacker_generator_config.fstab) {
        Err(e) => Err(format!(
            "Could not read the contents of '{}'{}",
            systemd_rollbacker_generator_config.fstab,
            make_formatted_error!(e)
        )
        .into()),
        Ok(fstab_contents) => {
            for fstab_line in fstab_contents.lines() {
                if fstab_line.is_empty() || fstab_line.starts_with('#') {
                    continue;
                }

                if let Ok((superblock_entry, device_mount_path)) =
                    SuperblockEntry::from(fstab_line, systemd_rollbacker_generator_config)
                {
                    parsed_fstab
                        .entry(superblock_entry)
                        .or_insert_with(Vec::new)
                        .push(device_mount_path);
                }
            }
            Ok(parsed_fstab)
        }
    }
}
