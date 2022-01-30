use tokio::sync::mpsc;

#[derive(Debug)]
pub enum Message {
    SomeU8(u8),
    SomeBool(bool),
    SomeChar(char),
}

/// Panics if all the channel sender handles are gone (disables all branches).
pub async fn until_message_panic_happy(
    bytes: &mut mpsc::Receiver<u8>,
    bools: &mut mpsc::Receiver<bool>,
    chars: &mut mpsc::Receiver<char>,
) -> Message {
    tokio::select! {
        Some(byte) = bytes.recv() => {
            Message::SomeU8(byte)
        },
        Some(bool) = bools.recv() => {
            Message::SomeBool(bool)
        },
        Some(char) = chars.recv() => {
            Message::SomeChar(char)
        },
        // no else branch.
    }
}

pub async fn until_message(
    bytes: &mut mpsc::Receiver<u8>,
    bools: &mut mpsc::Receiver<bool>,
    chars: &mut mpsc::Receiver<char>,
) -> Option<Message> {
    tokio::select! {
        Some(byte) = bytes.recv() => {
            Some(Message::SomeU8(byte))
        },
        Some(bool) = bools.recv() => {
            Some(Message::SomeBool(bool))
        },
        Some(char) = chars.recv() => {
            Some(Message::SomeChar(char))
        },
        else => None
    }
}

pub async fn until_message_loopy(
    bytes: &mut mpsc::Receiver<u8>,
    bools: &mut mpsc::Receiver<bool>,
    chars: &mut mpsc::Receiver<char>,
) -> Option<Message> {
    loop {
        tokio::select! {
            Some(byte) = bytes.recv() => {
                break Some(Message::SomeU8(byte))
            },
            Some(bool) = bools.recv() => {
                break Some(Message::SomeBool(bool))
            },
            Some(char) = chars.recv() => {
                break Some(Message::SomeChar(char))
            },
            else => break None
        }
    }
}

#[tokio::test]
async fn no_panic() {
    let (byte_tx, mut bytes) = mpsc::channel(1);
    let (bool_tx, mut bools) = mpsc::channel(1);
    let (char_tx, mut chars) = mpsc::channel(1);

    byte_tx.send(8).await.unwrap();
    bool_tx.send(true).await.unwrap();
    char_tx.send('a').await.unwrap();

    let msg = until_message(&mut bytes, &mut bools, &mut chars).await;
    assert!(msg.is_some());
}

#[tokio::test]
#[should_panic(expected = "all branches are disabled and there is no else branch")]
async fn panics() {
    let (_, mut bytes) = mpsc::channel(1);
    let (_, mut bools) = mpsc::channel(1);
    let (_, mut chars) = mpsc::channel(1);

    until_message_panic_happy(&mut bytes, &mut bools, &mut chars).await;
}

#[tokio::test]
async fn non_loopy_is_same_as_loopy() {
    let (byte_tx, mut bytes) = mpsc::channel(1);
    let (bool_tx, mut bools) = mpsc::channel(1);
    let (char_tx, mut chars) = mpsc::channel(1);

    byte_tx.send(8).await.unwrap();
    bool_tx.send(true).await.unwrap();
    char_tx.send('a').await.unwrap();

    let msg = until_message_loopy(&mut bytes, &mut bools, &mut chars).await;
    assert!(msg.is_some());
}
