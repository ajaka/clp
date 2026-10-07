use anyhow::{Result, bail};

mod cli;
pub mod common;
mod detect_platform;
mod modes;
#[cfg(test)]
mod testutil;

use cli::{Operation, parse};
use detect_platform::can_proceed;

pub fn begin(args: Vec<String>) -> Result<()> {
    if !can_proceed()? {
        bail!("{}", modes::no_manager_message());
    }

    if args.iter().any(|a| a == "--help" || a == "-help") {
        return modes::help::handle_help();
    }

    let cli = parse(&args)?;

    match cli.operation {
        Operation::Paste => return modes::paste::handle_paste(),
        Operation::Clear => return modes::clear::handle_clear(),
        _ => {}
    }

    let selection = modes::select(&cli)?;
    modes::deliver(&cli, &selection)
}
