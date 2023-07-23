use std::{sync::Arc, time::Duration};

use anyhow::Result;
use async_imap::extensions::idle::IdleResponse::{ManualInterrupt, NewData, Timeout};
use async_native_tls::TlsStream;
use futures::{StreamExt, TryStreamExt};
use itertools::Itertools;

use async_imap::{imap_proto::builders::command::fetch, types::Name, Session};
use tokio::{net::TcpStream, task, time::sleep};
use tracing::{debug, error};

use crate::{
    queue,
    store::{self, Account},
};

use super::parsers;

async fn get_session(account: Account) -> Result<Session<TlsStream<TcpStream>>> {
    let imap_addr = (account.imap_host.clone(), 993);
    let tcp_stream = TcpStream::connect(imap_addr).await?;
    let tls = async_native_tls::TlsConnector::new();
    let tls_stream = tls.connect(account.imap_host.clone(), tcp_stream).await?;

    let client = async_imap::Client::new(tls_stream);
    debug!("-- connected to {}:{}", account.imap_host, 993);

    // the client we have here is unauthenticated.
    // to do anything useful with the e-mails, we need to log in
    let mut imap_session = client
        .login(account.email.clone(), account.password.clone())
        .await
        .map_err(|e| e.0)?;
    debug!("-- logged in a {}", account.email);

    let server_capabilities = imap_session.capabilities().await?;
    debug!(
        "-- Advertised server capabilities: {:?}",
        server_capabilities
            .iter()
            .map(|s| format!("{:?}", s))
            .join(", ")
    );

    // Select the INBOX mailbox
    imap_session.select(account.mailbox.clone()).await?;
    debug!("-- INBOX selected");

    return Ok(imap_session);
}

// pub async fn get_mailboxes(imap_session: Session<TlsStream<TcpStream>>) -> Result<()> {
//     let mailboxes_stream = imap_session.list(Some(""), Some("*")).await;
//     let mailboxes: Vec<Result<Name, async_imap::error::Error>> = mailboxes_stream.collect().await?;

//     for mailbox in mailboxes {
//         match mailbox {
//             Ok(mailbox) => debug!("mailbox found: {:?}", mailbox.name()),
//             Err(e) => debug!("error: {:?}", e),
//         }
//     }
//     Ok(())
// }

pub async fn idle_inbox(
    account: Account,
    store: Arc<dyn store::Store>,
    queue: Arc<dyn queue::Queue>,
) -> Result<()> {
    // Idle for new email messages (unless interrupted or timed out)
    loop {
        let mut imap_session = get_session(account.clone()).await?;
        debug!(
            "-- logged in with account {}",
            account.clone().email.clone()
        );

        imap_session =
            fetch_inbox(imap_session, account.clone(), store.clone(), queue.clone()).await?;

        debug!("-- initializing idle");
        let mut idle = imap_session.idle();
        idle.init().await?;

        debug!("-- idle async wait");
        let (idle_wait, interrupt) = idle.wait();

        let timeout = 15; // 1500s = 25min
        task::spawn(async move {
            debug!("-- thread: waiting '{}' for {} seconds", "email", timeout);
            sleep(Duration::from_secs(timeout)).await;
            debug!(
                "-- thread: waited for '{}' for {} seconds, now interrupting idle",
                "email", timeout
            );
            drop(interrupt);
        });

        let idle_result = idle_wait.await?;
        match idle_result {
            ManualInterrupt => {
                // This could be a timeout from the client (our sleep function)
                debug!("-- IDLE manually interrupted");
                continue; // restart infinite loop, fetching at the beginning of the loop
            }
            Timeout => {
                // This is a timeout from the server
                debug!("-- IDLE timed out");
                continue; // restart infinite loop, fetching at the beginning of the loop
            }
            NewData(data) => {
                // The mailbox has received an update, it is time to trigger fetch
                let s = String::from_utf8(data.borrow_owner().to_vec()).unwrap();
                debug!("-- IDLE data (owner):\n{}", s); // Not relevant, information about the server
                debug!("-- IDLE data (dependent):\n{:?}", data.borrow_dependent());
            }
        }

        // return the session after an idle event is received
        debug!("-- idle DONE");
        imap_session = idle.done().await?;

        // be nice to the server and log out
        debug!("-- logging out");
        imap_session.logout().await?;
    }
}

async fn fetch_inbox(
    mut imap_session: Session<TlsStream<TcpStream>>,
    account: Account,
    store: Arc<dyn store::Store>,
    queue: Arc<dyn queue::Queue>,
) -> Result<Session<TlsStream<TcpStream>>> {
    // Fetch unread email messages
    let mut last_sequence = store.load_last_sequence(account.email.clone()).await?;
    let sequence_set = format!("{}:*", last_sequence);
    let query = "(FLAGS INTERNALDATE RFC822.SIZE BODY.PEEK[TEXT] ENVELOPE UID)";
    debug!(
        "Fetching emails for '{}' with sequence set '{}' and query '{}'",
        account.email.clone(),
        sequence_set,
        query
    );
    let messages_stream = imap_session.fetch(sequence_set, query).await?;
    let raw_messages: Vec<_> = messages_stream.try_collect().await?;
    let mut parsed = 0;
    let mut skipped = 0;
    for raw_message in raw_messages.iter() {
        let message = parsers::parse_message(account.email.clone(), raw_message);
        match message {
            Some(message) => {
                last_sequence = message.seq_id;
                queue
                    .publish_message(queue::QueueMessage {
                        email_message: message,
                    })
                    .await?;
                parsed += 1;
                // debug!("pushed message to queue");
            }
            None => {
                error!("unable to parse message (skipped).");
                skipped += 1;
            }
        }
    }
    store
        .store_last_sequence(account.email.clone(), last_sequence)
        .await?;

    debug!(
        "--  parsed {} | skipped {} | total {}",
        parsed,
        skipped,
        raw_messages.len()
    );
    Ok(imap_session)
}
