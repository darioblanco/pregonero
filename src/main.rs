use std::sync::Arc;
use store::Store;

use tracing::{debug, error, info};

use anyhow::Result;

pub mod config;
pub mod imap;
pub mod store;

#[tokio::main]
pub async fn main() -> Result<()> {
    // Load configuration variables
    let config = config::Config::from_env(&config::SystemEnvironment);
    // install global collector configured based on RUST_LOG env var.
    let filter = tracing_subscriber::EnvFilter::from_default_env()
        .add_directive(format!("pregonero={}", config.log_level).parse().unwrap())
        .add_directive("html2text=info".parse().unwrap());
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        // .with_max_level(config.log_level)
        .init();

    info!("Initializing config with {:?}", config);

    info!("Loading Store...");
    let store: Arc<dyn Store> =
        Arc::new(store::RedisStore::new(config.redis_server.to_string()).await);
    debug!("Load test account into the store");
    let stored_account = store
        .store_account(store::Account {
            email: config.imap_username,
            password: config.imap_password,
            mailbox: config.imap_mailbox,
            imap_host: config.imap_host,
        })
        .await;
    match stored_account {
        Ok(stored_account) => debug!("Account stored: {:?}", stored_account),
        Err(e) => {
            error!("Error while storing test account: {:?}", e);
            return Err(e);
        }
    }

    let accounts_res = store.load_accounts_by_host("icloud.com".to_string()).await;
    match accounts_res {
        Ok(accounts) => {
            debug!("Accounts loaded: {:?}", accounts);
            for account in accounts {
                let res = imap::fetch_inbox(
                    &account.imap_host,
                    &account.email,
                    &account.password,
                    &account.mailbox,
                )
                .await;

                match res {
                    Ok(_) => debug!("Messages loaded: {:?}", res),
                    Err(e) => error!("Error while fetching inbox: {:?}", e),
                }
            }
        }
        Err(e) => {
            info!("Error while loading accounts for 'icloud.com': {:?}", e);
        }
    }

    Ok(())
}
