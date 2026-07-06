use clap::Parser;
use remote_restart_plugin::cli::Cli;
use remote_restart_plugin::config::AppConfig;
use remote_restart_plugin::output;

fn main() {
    let cli = Cli::parse();
    let config = AppConfig::from_cli(&cli);

    if let Err(error) = remote_restart_plugin::run(cli.module_name(), config) {
        output::eprint_error(format_args!("{error}"));
        std::process::exit(1);
    }
}
