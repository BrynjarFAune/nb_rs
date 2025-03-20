#[allow(unused_imports)]
mod config;
mod fetch;
mod netbox;
mod utils;

use crate::fetch::azure::AzureClient;
#[allow(unused_imports)]
use dotenv::dotenv;
use netbox::{
    dcim, ipam,
    tenancy::{self, Contact},
    vm,
};
use std::sync::Arc;
use tokio::{
    self,
    sync::{Mutex, Semaphore},
    time::Instant,
};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let start_time = Instant::now();

    dotenv().ok();
    let settings = config::load()?;

    let azure_client = Arc::new(AzureClient::new(&settings.azure).await?);

    let netbox_client = Arc::new(netbox::api::ApiClient::new(&settings.netbox));

    let semaphore = Arc::new(Semaphore::new(10));

    // Start prep timer
    let prep_timer = Instant::now();
    // Complete preparation tasks like building the cache, fetching data, ... that needs to
    // complete before parsing and uploading.

    // spawn threads for data collection tasks
    let users_task = {
        let azure_client = Arc::clone(&azure_client);
        tokio::spawn(async move { azure_client.fetch_users().await })
    };
    let intune_devices_task = {
        let azure_client = Arc::clone(&azure_client);
        tokio::spawn(async move { azure_client.fetch_devices().await })
    };

    // can add a print statement here to showcase the tasks above have not neccessarily started or
    // ran yet

    // await the data collection tasks before contiuning
    let (users_res, intune_devices_res) = tokio::join!(users_task, intune_devices_task);

    let users = utils::extract_vec(users_res).await;
    let intune_devices = utils::extract_vec(intune_devices_res).await;

    let prep_elapsed = prep_timer.elapsed();

    // Start parse timer
    let parse_timer = Instant::now();

    let parsed_contacts = Arc::new(Mutex::new(vec![
        Contact::new("test_1".to_string()),
        Contact::new("test_2".to_string()),
        Contact::new("test_3".to_string()),
        Contact::new("test_4".to_string()),
        Contact::new("test_5".to_string()),
        Contact::new("test_6".to_string()),
        Contact::new("test_7".to_string()),
        Contact::new("test_8".to_string()),
        Contact::new("test_9".to_string()),
        Contact::new("test_10".to_string()),
    ]));

    // Stop parse timer
    let parse_elapsed = parse_timer.elapsed();

    // Start sync timer
    let sync_timer = Instant::now();

    // Stop sync timer
    let sync_elapsed = sync_timer.elapsed();

    // Stop main timer
    let elapsed_time = start_time.elapsed();

    println!("Sync completed successfully.");
    println!("{:.2?} elapsed on preparation", prep_elapsed);
    println!("{:.2?} elapsed on parsing", parse_elapsed);
    println!("{:.2?} elapsed on syncing", sync_elapsed);
    println!("{:.2?} elapsed total", elapsed_time);

    Ok(())
}
