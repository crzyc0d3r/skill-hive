fn main() {
    let output = hive_cli::execute(std::env::args());
    print!("{}", output.stdout);
    if !output.stdout.is_empty() && !output.stdout.ends_with('\n') {
        println!();
    }
    std::process::exit(output.code);
}
