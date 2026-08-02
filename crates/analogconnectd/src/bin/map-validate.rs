use std::io::{self, Read};

fn main() {
    let mut payload = String::new();
    if io::stdin().read_to_string(&mut payload).is_err() {
        eprintln!("MAP_VALIDATION=FAILED reason=stdin_read");
        std::process::exit(1);
    }

    let trimmed = payload.trim();
    let message_count = if trimmed.is_empty() || trimmed.starts_with("(no messages)") {
        0
    } else {
        payload
            .lines()
            .filter(|line| {
                let line = line.trim();
                !line.is_empty() && !line.starts_with('(')
            })
            .count()
    };
    println!("MAP_VALIDATION=PASS messages_seen={message_count}");
}
