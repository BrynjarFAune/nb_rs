use std::sync::Arc;

use crate::netbox::api::{ApiClient, CreateTable};
use async_trait::async_trait;
use futures::{stream, StreamExt};
use reqwest::Response;
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;

#[derive(Debug, Deserialize, Serialize)]
pub struct Site {
    id: Option<u32>,
    name: String,
    slug: String,
    status: Status,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Status {
    value: String,
    label: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Contact {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
}

impl Contact {
    pub fn new(name: String) -> Self {
        Self {
            id: None,
            name,
            email: None,
            title: None,
            group: None,
        }
    }

    pub fn with_id(mut self, id: u32) -> Self {
        self.id = Some(id);
        self
    }

    pub fn with_group(mut self, group: String) -> Self {
        self.group = Some(group);
        self
    }

    pub fn with_email(mut self, email: String) -> Self {
        self.email = Some(email);
        self
    }

    pub fn with_title(mut self, title: String) -> Self {
        self.title = Some(title);
        self
    }
}

#[derive(Debug, Deserialize)]
pub struct SiteList {
    pub count: u32,
    pub results: Vec<Site>,
}

#[derive(Debug, Deserialize)]
pub struct ContactList {
    pub count: u32,
    pub results: Vec<Contact>,
}

impl ApiClient {
    pub async fn create_contact(&self, contact: &Contact) -> Result<(), reqwest::Error> {
        self.post::<Contact, Contact>("tenancy/contacts/", contact)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl CreateTable for Contact {
    async fn create(&self, api: &ApiClient) -> Result<(), reqwest::Error> {
        api.create_contact(self).await
    }
}
