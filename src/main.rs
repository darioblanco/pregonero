use queue::Queue;
use std::sync::Arc;
use store::Store;
use tokio::task;
use tracing::{debug, error, info};

use anyhow::Result;

pub mod config;
pub mod fixtures;
pub mod imap;
pub mod queue;
pub mod store;

#[tokio::main]
pub async fn main() -> Result<()> {
    // Load configuration variables
    let config = config::Config::from_env(&config::SystemEnvironment);
    // install global collector configured based on RUST_LOG env var.
    let filter = tracing_subscriber::EnvFilter::from_default_env()
        .add_directive(format!("pregonero={}", config.log_level).parse().unwrap())
        .add_directive("html2text=error".parse().unwrap());
    tracing_subscriber::fmt().with_env_filter(filter).init();

    info!("Initialized config with {:?}", config);

    info!("Setting store in {}...", config.redis_server);
    let store: Arc<dyn Store> =
        Arc::new(store::RedisStore::new(config.redis_server.to_string()).await);
    info!("Store set up at {}", config.redis_server);

    // Load accounts only if test environment is given
    if config.app_env == config::AppEnv::Development {
        info!("Loading test accounts...");
        fixtures::load_accounts(&store).await;
        info!("Test accounts loaded from the fixtures file");
    }

    info!("Setting queue in {}...", config.redis_server);
    let queue: Arc<dyn Queue> =
        Arc::new(queue::RedisQueue::new(config.redis_server.to_string()).await);
    info!("Queue set up at {}", config.redis_server);

    let accounts_res = store.load_accounts_by_host("icloud.com".to_string()).await;
    let mut fetch_tasks = vec![];
    // let mut idle_tasks: Vec<_> = vec![];
    match accounts_res {
        Ok(accounts) => {
            debug!("Accounts loaded: {:?}", accounts);
            for account in accounts {
                let fetch_task = task::spawn(imap::fetch_inbox(
                    account.clone(),
                    store.clone(),
                    queue.clone(),
                ));
                fetch_tasks.push(fetch_task);
                // let idle_task = task::spawn(imap::idle_inbox(
                //     account.clone(),
                //     store.clone(),
                //     queue.clone(),
                // ));
                // idle_tasks.push(idle_task);
            }
        }
        Err(e) => {
            error!("Error while loading accounts: {:?}", e);
        }
    }

    // Await all fetch tasks to finish
    for task in fetch_tasks {
        if let Err(e) = task.await {
            // Handle errors gracefully
            error!("Error in task: {:?}", e);
        }
    }

    // // Await all idle tasks indefinitely
    // loop {
    //     for task in &mut idle_tasks {
    //         if let Err(e) = task.await {
    //             // Handle errors gracefully
    //             error!("Error in idle task: {:?}", e);
    //         }
    //     }
    // }

    Ok(())
}
