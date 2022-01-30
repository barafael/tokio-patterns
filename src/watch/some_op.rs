use rand::Rng;
use std::time::Duration;
use tokio::sync::mpsc;

pub async fn do_thing<T>(id: T, on_stop: mpsc::Sender<T>) {
    tokio::time::sleep(random_delay()).await;
    let _ = on_stop.send(id).await;
}

fn random_delay() -> Duration {
    let mut rng = rand::thread_rng();
    Duration::from_millis(rng.gen_range(250..1000))
}
