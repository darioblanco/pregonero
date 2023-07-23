use anyhow::Result;
use async_trait::async_trait;
use redis::{AsyncCommands, Client};
use serde_derive::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;
use tracing::debug;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_store_account_and_load_by_email_and_destroy() {
        let store = RedisStore::new("redis://localhost:6380/0".to_string()).await;

        let account = Account {
            email: "test@test.com".to_string(),
            password: "password".to_string(),
            mailbox: "INBOX".to_string(),
            imap_host: "imap.test.com".to_string(),
        };

        // Store the account
        let stored_account_result = store.store_account(account.clone()).await.unwrap();
        assert!(stored_account_result.is_some());

        // Load the account by email
        let loaded_account = store
            .load_account_by_email(account.email.clone())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded_account.email, account.email);
        assert_eq!(loaded_account.password, account.password);
        assert_eq!(loaded_account.mailbox, account.mailbox);
        assert_eq!(loaded_account.imap_host, account.imap_host);

        // Destroy the account
        store.destroy_account(account.email.clone()).await.unwrap();
        assert_eq!(
            store
                .load_account_by_email(account.email.clone())
                .await
                .unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn test_load_accounts_by_host_and_clear() {
        let store = RedisStore::new("redis://localhost:6380/1".to_string()).await;

        let account1 = Account {
            email: "test1@test.com".to_string(),
            password: "password1".to_string(),
            mailbox: "INBOX".to_string(),
            imap_host: "imap.test.com".to_string(),
        };

        let account2 = Account {
            email: "test2@test.com".to_string(),
            password: "password2".to_string(),
            mailbox: "INBOX".to_string(),
            imap_host: "imap.test.com".to_string(),
        };

        // Store accounts
        store.store_account(account1.clone()).await.unwrap();
        store.store_account(account2.clone()).await.unwrap();

        // Load accounts by host
        let accounts = store
            .load_accounts_by_host("test.com".to_string())
            .await
            .unwrap();
        assert_eq!(accounts.len(), 2);
        assert_eq!(accounts.contains(&account1), true);
        assert_eq!(accounts.contains(&account2), true);

        // Clear host accounts
        store
            .clear_host_accounts("test.com".to_string())
            .await
            .unwrap();
        assert_eq!(
            store
                .load_accounts_by_host("test.com".to_string())
                .await
                .unwrap()
                .len(),
            0
        );
    }
}
