use clap::Parser;
use kinic_core::tui::run_with_config;
use tui_kit_lib::{build_launch_config_from_args, TuiArgs};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = TuiArgs::parse();
    let config = build_launch_config_from_args(&args)?;
    run_with_config(config).map_err(Into::into)
}
