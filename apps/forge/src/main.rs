//! `forge`: the command-line tool (docs/specs/CLI.md).

fn main() -> std::process::ExitCode {
    forge_cli::run(std::env::args_os().skip(1))
}
