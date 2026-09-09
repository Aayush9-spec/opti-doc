use anyhow::Result;
use optidock_core::Storage;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_home() -> std::path::PathBuf {
    let base = std::env::temp_dir().join(format!(
        "optidock-storage-test-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&base);
    base
}

#[test]
fn persists_conversation_and_messages() -> Result<()> {
    let home = temp_home();
    let storage = Storage::open(home.clone())?;

    let conversation = storage.conversations.create("Test conversation", None)?;
    storage.messages.create(
        &conversation.id,
        "user",
        "Hello from the CLI",
        None,
        None,
    )?;
    storage.messages.create(
        &conversation.id,
        "assistant",
        "Hello! How can I help?",
        None,
        None,
    )?;

    let listed = storage.conversations.list(None, 10)?;
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].title, "Test conversation");

    let messages = storage.messages.list(&conversation.id, 20)?;
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].role, "user");
    assert_eq!(messages[1].role, "assistant");

    Ok(())
}

#[test]
fn searches_messages_by_keyword() -> Result<()> {
    let home = temp_home();
    let storage = Storage::open(home)?;

    let conversation = storage.conversations.create("Docker debug", None)?;
    storage.messages.create(
        &conversation.id,
        "user",
        "docker connection refused while starting backend",
        None,
        None,
    )?;
    storage.messages.create(
        &conversation.id,
        "assistant",
        "The API is healthy and the issue was a config mistake.",
        None,
        None,
    )?;

    let results = storage.search.query("docker connection refused")?;
    assert!(!results.is_empty());
    assert!(results.iter().any(|r| r.content.contains("docker connection refused")));

    Ok(())
}
