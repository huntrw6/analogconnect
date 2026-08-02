use std::io::{self, Read};

use analogconnect_core::parse_imsg_contacts;

fn main() {
    if let Err(code) = run() {
        std::process::exit(code);
    }
}

fn run() -> Result<(), i32> {
    let mut payload = String::new();
    if io::stdin().read_to_string(&mut payload).is_err() {
        eprintln!("PBAP_VALIDATION=FAILED reason=stdin_read");
        return Err(1);
    }

    let contacts = match parse_imsg_contacts(&payload) {
        Ok(contacts) => contacts,
        Err(_) => {
            eprintln!("PBAP_VALIDATION=FAILED reason=invalid_payload");
            return Err(1);
        }
    };
    let phone_count = contacts
        .iter()
        .map(|contact| contact.phones.len())
        .sum::<usize>();
    println!(
        "PBAP_VALIDATION=PASS contacts={} phones={}",
        contacts.len(),
        phone_count
    );
    Ok(())
}
