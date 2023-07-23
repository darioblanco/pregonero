use std::sync::Arc;

use anyhow::Result;
use async_native_tls::TlsStream;
use futures::{StreamExt, TryStreamExt};
use itertools::Itertools;

use async_imap::{types::Name, Session};
use tokio::net::TcpStream;
use tracing::{debug, error};

use crate::{
    queue,
    store::{self, Account},
};

use super::parsers;

pub async fn get_session(
    imap_server: &str,
    login: &str,
    password: &str,
) -> Result<Session<TlsStream<TcpStream>>> {
    let imap_addr = (imap_server, 993);
    let tcp_stream = TcpStream::connect(imap_addr).await?;
    let tls = async_native_tls::TlsConnector::new();
    let tls_stream = tls.connect(imap_server, tcp_stream).await?;

    let client = async_imap::Client::new(tls_stream);
    debug!("-- connected to {}:{}", imap_addr.0, imap_addr.1);

    // the client we have here is unauthenticated.
    // to do anything useful with the e-mails, we need to log in
    let imap_session = client.login(login, password).await.map_err(|e| e.0)?;
    debug!("-- logged in a {}", login);
    return Ok(imap_session);
}

// pub async fn get_mailboxes(imap_session: Session<TlsStream<TcpStream>>) {
//     let mailboxes_stream = imap_session.list(Some(""), Some("*")).await;
//     let mailboxes: Vec<Result<Name, async_imap::error::Error>> = mailboxes_stream.collect().await;

//     for mailbox in mailboxes {
//         match mailbox {
//             Ok(mailbox) => debug!("mailbox found: {:?}", mailbox.name()),
//             Err(e) => debug!("error: {:?}", e),
//         }
//     }
// }

pub async fn fetch_inbox(
    account: Account,
    store: Arc<dyn store::Store>,
    queue: Arc<dyn queue::Queue>,
) -> Result<()> {
    // the client we have here is unauthenticated.
    // to do anything useful with the e-mails, we need to log in
    let mut imap_session =
        get_session(&account.imap_host, &account.email, &account.password).await?;
    debug!("-- logged in with account {}", &account.email);

    // we want to fetch the first email in the INBOX mailbox
    imap_session.select(&account.mailbox).await?;
    debug!("-- INBOX selected");

    let server_capabilities = imap_session.capabilities().await?;
    debug!(
        "-- Advertised server capabilities: {:?}",
        server_capabilities
            .iter()
            .map(|s| format!("{:?}", s))
            .join(", ")
    );

    // fetch message number 1 in this mailbox, along with its RFC822 field.
    // RFC 822 dictates the format of the body of e-mails
    // let messages_stream = imap_session.fetch("1", "RFC822").await?;

    // fetch all messages from the inbox (1:*)
    let messages_stream = imap_session
        .fetch(
            format!("{}:*", 1), // TODO - load last sequence id from the store
            "(FLAGS INTERNALDATE RFC822.SIZE BODY.PEEK[TEXT] ENVELOPE UID)",
        )
        .await?;
    let raw_messages: Vec<_> = messages_stream.try_collect().await?;
    let mut parsed = 0;
    let mut skipped = 0;
    for raw_message in raw_messages.iter() {
        let message = parsers::parse_message(account.email.clone(), raw_message);
        match message {
            Some(message) => {
                queue
                    .publish_message(queue::QueueMessage {
                        email_message: message,
                    })
                    .await?;
                parsed += 1;
                debug!("pushed message to queue");
            }
            None => {
                error!("unable to parse message (skipped).");
                skipped += 1;
            }
        }
    }

    debug!(
        "--  parsed {} | skipped {} | total {}",
        parsed,
        skipped,
        raw_messages.len()
    );

    // Use idle() to listen for updates continuously
    let mut handle = imap_session.idle();
    let _ = handle.init().await;
    let _ = handle.wait();

    Ok(())
}

// // Function to listen for updates on an IMAP account
// async fn listen_for_updates(
//     imap_session: Session<TlsStream<TcpStream>>,
//     mailbox: &str,
//     last_seq_id: u32,
// ) -> Result<()> {
//     // Fetch the new messages since the last sequential ID
//     let mailbox = format!("{}/{}", mailbox, last_seq_id + 1);
//     let fetch_result = imap_session.fetch(mailbox, "(FLAGS BODY.PEEK[])").await?;
//     for message in fetch_result {
//         // Process the new messages
//     }

//     // Use idle() to listen for updates continuously
//     let handler = imap_session.idle();

//     Ok(())
// }
