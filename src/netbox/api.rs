use anyhow::{anyhow, Result};
use futures::{stream, StreamExt};
use reqwest::{Client, Error as ReqwestError, StatusCode};
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, io::ErrorKind, sync::Arc};
use tokio::sync::Semaphore;

use crate::{config::NetBoxConfig, netbox::models};
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

#[derive(Debug, Deserialize, Clone)]
pub struct NetBoxResponse<T> {
    count: i32,
    next: String,
    results: Vec<T>,
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
        T: CreateTable + 'static,
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
    pub async fn get<T>(&self, endpoint: &str, id: Option<i32>) -> Result<Vec<T>, ReqwestError>
    where
        T: for<'de> Deserialize<'de> + Debug,
    {
        let mut next_link = match id {
            Some(id) => format!("{}/{}/{}", self.api_url, endpoint, id),
            None => format!("{}/{}", self.api_url, endpoint),
        };
        let mut results: Vec<T> = Vec::new();

        while !next_link.is_empty() {
            let response = self
                .client
                .get(&next_link)
                .header("Authorization", format!("Token {}", self.api_key))
                .header("Content-Type", "application/json")
                .send()
                .await?;

            match response.status() {
                StatusCode::OK => {
                    println!("Test before extraction");
                    let response_data: NetBoxResponse<T> = response.json().await?;
                    println!("Test after extraction");
                    results.extend(response_data.results);
                    if response_data.next.is_empty() {
                        break;
                    } else {
                        next_link = response_data.next;
                    }
                }
                _ => {
                    eprintln!(
                        "Failed to get data from {}: {}",
                        next_link,
                        response.status()
                    );
                }
            }
        }

        println!("len results: {}", results.len());
        if results.len() > 0 {
            println!("first result: {:?}", results.first());
        }

        Ok(results)
    }

    // Generic POST request
    pub async fn post<T, B>(&self, endpoint: &str, body: &B) -> Result<T, ReqwestError>
    where
        T: for<'de> Deserialize<'de>,
        B: Serialize + Debug,
    {
        let url = format!("{}/{}/", self.api_url, endpoint);

        println!("POST request: {} - {:?}", url, body);

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
