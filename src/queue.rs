use anyhow::Result;
use async_trait::async_trait;
use redis::{AsyncCommands, Client};
use serde_derive::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;

use crate::imap::EmailMessage;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct QueueMessage {
    pub email_message: EmailMessage,
}

impl fmt::Display for QueueMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(email_message: {})", self.email_message)
    }
}

#[async_trait]
pub trait Queue {
    /// Publish a message to the queue.
    async fn publish_message(&self, message: QueueMessage) -> Result<()>;
}

#[derive(Clone, Debug)]
pub struct RedisQueue {
    redis_client: Arc<Client>,
}

impl RedisQueue {
    pub async fn new(connection_url: String) -> Self {
        let client = redis::Client::open(connection_url).unwrap();
        Self {
            redis_client: Arc::new(client),
        }
    }
}

#[async_trait]
impl Queue for RedisQueue {
    async fn publish_message(&self, message: QueueMessage) -> Result<()> {
        let message_str = serde_json::to_string(&message)?;

        let mut con = self.redis_client.get_async_connection().await?;
        let _: () = con.publish("emails", message_str).await?;

        Ok(())
    }
}
