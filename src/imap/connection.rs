use anyhow::Result;
use futures::{StreamExt, TryStreamExt};

use async_imap::types::Name;
use tokio::net::TcpStream;
use tracing::{debug, error};

use super::parsers;

pub async fn fetch_inbox(
    imap_server: &str,
    login: &str,
    password: &str,
    mailbox: &str,
) -> Result<Vec<parsers::Message>> {
    let imap_addr = (imap_server, 993);
    let tcp_stream = TcpStream::connect(imap_addr).await?;
    let tls = async_native_tls::TlsConnector::new();
    let tls_stream = tls.connect(imap_server, tcp_stream).await?;

    let client = async_imap::Client::new(tls_stream);
    debug!("-- connected to {}:{}", imap_addr.0, imap_addr.1);

    // the client we have here is unauthenticated.
    // to do anything useful with the e-mails, we need to log in
    let mut imap_session = client.login(login, password).await.map_err(|e| e.0)?;
    debug!("-- logged in a {}", login);

    let mailboxes_stream = imap_session.list(Some(""), Some("*")).await?;
    let mailboxes: Vec<Result<Name, async_imap::error::Error>> = mailboxes_stream.collect().await;

    for mailbox in mailboxes {
        match mailbox {
            Ok(mailbox) => debug!("mailbox found: {:?}", mailbox.name()),
            Err(e) => debug!("error: {:?}", e),
        }
    }

    // we want to fetch the first email in the INBOX mailbox
    imap_session.select(mailbox).await?;
    debug!("-- INBOX selected");

    // fetch message number 1 in this mailbox, along with its RFC822 field.
    // RFC 822 dictates the format of the body of e-mails
    // let messages_stream = imap_session.fetch("1", "RFC822").await?;

    // fetch all messages from the inbox (1:*)
    let messages_stream = imap_session
        .fetch(
            "1:*",
            "(FLAGS INTERNALDATE RFC822.SIZE BODY.PEEK[TEXT] ENVELOPE)",
        )
        .await?;
    let raw_messages: Vec<_> = messages_stream.try_collect().await?;
    let mut messages = Vec::<parsers::Message>::new();
    for raw_message in raw_messages.iter() {
        let message = parsers::parse_message(login.to_string(), raw_message);
        match message {
            Some(message) => messages.push(message),
            None => error!("Unable to parse message {:?}. SKIPPED.", raw_message),
        }
    }

    println!("-- {} messages processed, logging out", messages.len());

    // be nice to the server and log out
    imap_session.logout().await?;

    Ok(messages)
}
