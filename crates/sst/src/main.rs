//! `sst` binary (port of `cmd/sst/main.go`).

use std::io::Write;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut stderr = std::io::stderr();
    let code = if super_star_trek::is_classic(&args) {
        super_star_trek::classic::run_classic_cli(&mut stdin, &mut stdout, &args)
    } else {
        super_star_trek::run(&args, &mut stdin, &mut stdout, &mut stderr)
    };
    let _ = stdout.flush();
    let _ = stderr.flush();
    std::process::exit(code);
}
