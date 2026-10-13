use clap::Parser;

#[derive(Parser)]
#[command(name = "skill-hive-cli", version, about = "Skill Hive command line")]
struct Cli {
    /// Reserved for the reference command tree. Phase 0 prints the version.
    #[arg(long, hide = true)]
    _placeholder: bool,
}

fn main() {
    let _cli = Cli::parse();
    println!("skill-hive-cli {}", env!("CARGO_PKG_VERSION"));
}
