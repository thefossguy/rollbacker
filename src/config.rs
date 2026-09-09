// SPDX-License-Identifier: GPL-2.0-only
// Copyright (C) 2026 Pratham Patel <prathampatel@thefossguy.com>
//
// This program is free software; you can redistribute it and/or
// modify it under the terms of the GNU General Public License
// as published by the Free Software Foundation; only as version 2
// of the License, NOT as a later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program; if not, see <https://www.gnu.org/licenses/>.

use std::error::Error;
use std::fmt;

#[derive(PartialEq)]
pub enum FilesystemType {
    Btrfs,
    Zfs,
}
impl FilesystemType {
    fn from(filesystem_type: &str) -> Result<Self, Box<dyn Error>> {
        match filesystem_type {
            "btrfs" => Ok(Self::Btrfs),
            "zfs" => Ok(Self::Zfs),
            _ => Err(format!("The filesystem type '{filesystem_type}' is unsupported").into()),
        }
    }
}
impl fmt::Debug for FilesystemType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Btrfs => write!(f, "btrfs"),
            Self::Zfs => write!(f, "zfs"),
        }
    }
}
impl fmt::Display for FilesystemType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Debug)]
pub struct RollbackerConfig {
    pub filesystem_type: FilesystemType,
    pub fresh_snapshot_suffix: String,
    pub superblock_path: String,
}

pub fn configure() -> Result<RollbackerConfig, Box<dyn Error>> {
    use lexopt::prelude::*;

    let mut filesystem_type = None;
    let mut fresh_snapshot_suffix = None;
    let mut superblock_path = None;

    let mut lexopt_parser = lexopt::Parser::from_env();
    while let Some(arg) = lexopt_parser.next()? {
        match arg {
            Long("filesystem-type") => {
                filesystem_type = Some(FilesystemType::from(&lexopt_parser.value()?.string()?)?);
            }
            Long("fresh-snapshot-suffix") => {
                fresh_snapshot_suffix = Some(lexopt_parser.value()?.string()?);
            }
            Long("superblock-path") => superblock_path = Some(lexopt_parser.value()?.string()?),
            _ => return Err(arg.unexpected().into()),
        }
    }

    let Some(filesystem_type) = filesystem_type else {
        return Err("`--filesystem-type` must be specified".into());
    };
    let Some(fresh_snapshot_suffix) = fresh_snapshot_suffix else {
        return Err("`--fresh-snapshot-suffix` must be specified".into());
    };
    let superblock_path = if let Some(superblock_path) = superblock_path {
        if filesystem_type != FilesystemType::Zfs && std::path::Path::new(&superblock_path).exists()
        {
            superblock_path
        } else {
            return Err(format!(
                "The specified superblock path '{superblock_path}' does not exist"
            )
            .into());
        }
    } else {
        return Err("`--superblock-path` must be specified".into());
    };

    Ok(RollbackerConfig {
        filesystem_type,
        fresh_snapshot_suffix,
        superblock_path,
    })
}
