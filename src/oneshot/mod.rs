#[tokio::test]
async fn oneshot_send_drops_sender_handle() {
    let (tx, rx) = tokio::sync::oneshot::channel::<u32>();

    tokio::spawn(async move {
        let _ = tx.send(1);
        //drop(tx); // use of moved value.
    });

    match rx.await {
        Ok(_) => println!("something came out"),
        Err(_) => println!("the sender dropped"),
    }
}
