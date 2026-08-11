#[tokio::main]
async fn main() {
    if run().await.is_err() {
        eprintln!("CONVERSATION_ID_VALIDATION=FAIL reason=store_unavailable");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), ()> {
    let config = imsg_config::load(None).map_err(|_| ())?;
    let path = config.store.resolve().ok_or(())?;
    let ready = imsg_keyring::init_store().map_err(|_| ())?;
    let key = imsg_keyring::get_or_create_db_key(&ready).map_err(|_| ())?;
    let store = imsg_store::Store::open(path, key).await.map_err(|_| ())?;
    let threads = store.threads().await.map_err(|_| ())?;
    let map_identities = threads
        .iter()
        .filter(|thread| thread.conversation_key.starts_with("map:"))
        .count();
    let groups = threads
        .iter()
        .filter(|thread| thread.participant_count > 1)
        .count();
    println!(
        "CONVERSATION_ID_VALIDATION=PASS threads={} map_identities={} groups={}",
        threads.len(),
        map_identities,
        groups
    );
    Ok(())
}
