use analogconnectd::audio::{PwDumpRunner, ScoNodeLocator};

fn main() {
    let locator = ScoNodeLocator::new(PwDumpRunner::default());
    match locator.locate() {
        Ok(_) => println!("SCO_NODE_VALIDATION=PASS pair_found=1"),
        Err(error) => {
            eprintln!("SCO_NODE_VALIDATION=FAILED reason={error}");
            std::process::exit(1);
        }
    }
}
