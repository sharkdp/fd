fn main() {
    let min_version = "1.57";

    match version_check::is_min_version(min_version) {
        Some(true) => {}
        // rustc version too small or can't figure it out
        _ => {
            eprintln!("'fd' requires rustc >= {}", min_version);
            std::process::exit(1);
        }
    }
}
