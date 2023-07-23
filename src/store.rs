use anyhow::Result;
use async_trait::async_trait;
use redis::{AsyncCommands, Client};
use serde_derive::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;
use tracing::debug;

#[derive(Debug, Serialize, Deserialize)]
pub struct Account {
    pub email: String,
    pub password: String,
    pub mailbox: String,   // INBOX by default
    pub imap_host: String, // TODO: could be detected depending on the @provider part of the username
}

impl fmt::Display for Account {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "(email: {}, mailbox: {}, imap_host: {})",
            self.email, self.mailbox, self.imap_host,
        )
    }
}

#[async_trait]
pub trait Store {
    /// Get alist of accounts from the account store that matches a pattern.
    async fn load_accounts_by_host(&self, host: String) -> Result<Vec<Account>>;

    /// Get an account from the account store.
    async fn load_account_by_email(&self, email: String) -> Result<Option<Account>>;

    /// Store a account on the account store.
    async fn store_account(&self, account: Account) -> Result<Option<String>>;

    /// Remove an account from the account store
    async fn destroy_account(&self, email: String) -> Result<()>;

    /// Destroy all accounts belonging to a host
    async fn clear_host_accounts(&self, host: String) -> Result<()>;
}

#[derive(Clone, Debug)]
pub struct RedisStore {
    redis_client: Arc<Client>,
}

impl RedisStore {
    pub async fn new(connection_url: String) -> Self {
        let client = redis::Client::open(connection_url).unwrap();

        // Test the connection
        let mut con = client.get_async_connection().await.unwrap();
        let _: () = con.set("test_key", "test_value").await.unwrap();
        let result: String = con.get("test_key").await.unwrap();
        println!("Redis test_key: {}", result);

        Self {
            redis_client: Arc::new(client),
        }
    }
}

#[async_trait]
impl Store for RedisStore {
    async fn load_accounts_by_host(&self, host: String) -> Result<Vec<Account>> {
        debug!("Load accounts for email host {}", host);
        let mut con = self.redis_client.get_async_connection().await.unwrap();
        let mut cursor: usize = 0;
        let mut accounts: Vec<Account> = vec![];

        loop {
            let res: (usize, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(format!("account:*@{}", host))
                .query_async(&mut con)
                .await
                .unwrap();

            cursor = res.0;
            let keys: Vec<String> = res.1;

            // Fetch and parse the account data for the keys
            for key in keys {
                let account_json: Option<String> = con.get(&key).await.unwrap();
                match account_json {
                    Some(json) => {
                        let account: Account = serde_json::from_str(&json)?;
                        accounts.push(account);
                    }
                    None => (),
                }
            }

            // If the cursor is 0, we have completed the iteration
            if cursor == 0 {
                break;
            }
        }

        Ok(accounts)
    }

    async fn load_account_by_email(&self, email: String) -> Result<Option<Account>> {
        debug!("Load account for email '{}'", email);
        let key = format!("account:{}", email);
        let mut con = self.redis_client.get_async_connection().await.unwrap();
        let account_json: Option<String> = con.get(&key).await.unwrap();
        match account_json {
            Some(json) => Ok(Some(serde_json::from_str(&json)?)),
            None => Ok(None),
        }
    }

    async fn store_account(&self, account: Account) -> Result<Option<String>> {
        debug!("Store account {:?}", account);
        let key = format!("account:{}", account.email);
        let value = serde_json::to_string(&account)?;
        let mut con = self.redis_client.get_async_connection().await.unwrap();
        con.set::<_, _, ()>(&key, &value).await.unwrap();
        Ok(Some(value))
    }

    async fn destroy_account(&self, email: String) -> Result<()> {
        debug!("Destroy account for email '{}'", email);
        let key = format!("account:{}", email);
        let mut con = self.redis_client.get_async_connection().await.unwrap();
        con.del::<_, ()>(&key).await.unwrap();
        Ok(())
    }

    async fn clear_host_accounts(&self, host: String) -> Result<()> {
        debug!("Clear all accounts belonging to a host");
        let mut con = self.redis_client.get_async_connection().await.unwrap();
        let mut cursor: usize = 0;

        loop {
            let res: (usize, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(format!("account:*@{}", host))
                .query_async(&mut con)
                .await
                .unwrap();

            cursor = res.0;
            let keys: Vec<String> = res.1;

            // Delete the keys
            if !keys.is_empty() {
                let _: () = con.del(keys).await.unwrap();
            }

            // If the cursor is 0, we have completed the iteration
            if cursor == 0 {
                break;
            }
        }

        Ok(())
    }
}
