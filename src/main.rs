mod config;
mod filesystem;

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let rollbacker_config = config::configure()?;
    eprintln!("{rollbacker_config:#?}");
    filesystem::rollback_filesystem(&rollbacker_config)
}
