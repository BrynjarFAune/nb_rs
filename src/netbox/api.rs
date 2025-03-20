use anyhow::{anyhow, Result};
use futures::{stream, StreamExt};
use reqwest::{Client, Error as ReqwestError};
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, sync::Arc};
use tokio::sync::Semaphore;

use crate::config::NetBoxConfig;
use async_trait::async_trait;

#[async_trait]
pub trait CreateTable: Send + Sync + std::fmt::Debug {
    async fn create(&self, api: &ApiClient) -> Result<(), ReqwestError>;
}

#[derive(Debug, Clone)]
pub struct ApiClient {
    client: Client,
    api_url: String,
    api_key: String,
    api_limit: usize,
}

impl ApiClient {
    pub fn new(config: &NetBoxConfig) -> Self {
        let api_url = config.api_url.clone();
        let api_key = config.api_key.clone();
        let api_limit = config.api_limit.clone();

        Self {
            client: Client::new(),
            api_url,
            api_key,
            api_limit,
        }
    }

    pub async fn sync_objects<T>(&self, objects: Vec<T>, semaphore: Arc<Semaphore>, name: &str)
    where
        T: CreateTable + std::fmt::Debug + Send + Sync + 'static,
    {
        let client = Arc::new(self.clone());
        let api_limit = self.api_limit;

        println!("Syncing {} {} objects...", objects.len(), name);

        stream::iter(objects)
            .for_each_concurrent(api_limit, |obj| {
                let client = client.clone();
                let sem = semaphore.clone();
                async move {
                    let permit = sem.acquire_owned().await.unwrap();
                    println!("-> Creating: {:?}", obj);
                    if let Err(e) = obj.create(&client).await {
                        eprintln!("Failed to create {:?}: {:?}", obj, e);
                    } else {
                        println!("Created: {:?}", obj);
                    }
                    drop(permit);
                }
            })
            .await;

        println!("Finished syncing {}.", name);
    }

    // Generic GET request
    pub async fn get<T>(&self, endpoint: &str) -> Result<T, ReqwestError>
    where
        T: for<'de> Deserialize<'de>,
    {
        let url = format!("{}{}", self.api_url, endpoint);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Token {}", self.api_key))
            .header("Content-Type", "application/json")
            .send()
            .await?;

        response.json::<T>().await
    }

    // Generic POST request
    pub async fn post<T, B>(&self, endpoint: &str, body: &B) -> Result<T, ReqwestError>
    where
        T: for<'de> Deserialize<'de>,
        B: Serialize + Debug,
    {
        let url = format!("{}{}", self.api_url, endpoint);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Token {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await?;

        response.json::<T>().await
    }
}
