mod cache;
mod config;
mod fetch;
mod netbox;
mod utils;

use cache::LocalCache;
use dotenv::dotenv;
use fetch::azure::IntuneUser;
use futures::{future::join_all, TryFutureExt};
use netbox::models::{
    self, Contact, ContactList, Device, DeviceRole, DeviceType, Manufacturer, Site, Status,
};
use rand;
use std::sync::Arc;
use tokio::{
    self,
    sync::{Mutex, RwLock, Semaphore},
    task,
    time::Instant,
};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    //
    // Prepare environment
    let start_time = Instant::now();
    dotenv().ok();
    let settings = config::load()?;
    let azure_client = Arc::new(fetch::azure::AzureClient::new(&settings.azure).await?);
    let fortigate_client =
        Arc::new(fetch::fortigate::FortiGateClient::new(&settings.fortigate).await?);
    let netbox_client = Arc::new(netbox::api::ApiClient::new(&settings.netbox));
    let semaphore = Arc::new(Semaphore::new(settings.netbox.api_limit.clone()));

    let mut handles = Vec::new();

    // Build cache
    let cache_future = cache::LocalCache::preload(netbox_client.clone());

    println!("Preloaded Contacts:");

    // Get data
    let azure_contacts_future = azure_client.fetch_users().map_err(Into::into);
    let azure_devices_future = azure_client.fetch_devices().map_err(Into::into);
    let fortigate_devices_future = fortigate_client.fetch_devices().map_err(Into::into);

    let (local_cache, azure_contacts, azure_devices, fortigate_devices) = tokio::try_join!(
        cache_future,
        azure_contacts_future,
        azure_devices_future,
        fortigate_devices_future
    )?;

    println!("Azure device: {:?}", azure_devices.first());
    println!("FortiGate device: {:?}", fortigate_devices.first());
    println!("Found {} devices via fortigate", fortigate_devices.len());

    // Push data to netbox

    for contact in azure_contacts {
        if !local_cache.contacts.contains_key(&contact.name) {
            let netbox_client = Arc::clone(&netbox_client);
            let permit = semaphore.clone().acquire_owned().await.unwrap();

            let handle = task::spawn(async move {
                let tmp_contact: Contact = contact.into();
                let result = netbox_client
                    .post::<Contact, Contact>("tenancy/contacts", &tmp_contact)
                    .await;
                println!("++ Contact: {}", &tmp_contact.name);
                drop(permit);
                result
            });
            handles.push(handle);
        }
    }
    join_all(handles).await;

    // End script
    let timer = start_time.elapsed();
    println!("Time elapsed: {:.2?}", timer);

    Ok(())
}

/*
#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let start_time = Instant::now();

    // Start prep timer
    let prep_timer = Instant::now();
    // Complete preparation tasks like building the cache, fetching data, ... that needs to
    // complete before parsing and uploading.

    dotenv().ok();
    let settings = config::load()?;

    // Ready the NetBox cache
    let mut nb_cache = Arc::new(RwLock::new(Cache::new()));

    // Ready the collector clients for Azure, FortiGate & ESET
    let azure_client = Arc::new(fetch::azure::AzureClient::new(&settings.azure).await?);

    // Ready the NetBox client for interacting with the NetBox API
    let netbox_client = Arc::new(netbox::api::ApiClient::new(&settings.netbox));

    // Create the semaphpore to allow limited concurrent api interractions
    let semaphore = Arc::new(Semaphore::new(settings.netbox.api_limit.clone()));

    // spawn threads for data collection tasks
    let users_task = {
        let azure_client = Arc::clone(&azure_client);
        tokio::spawn(async move { azure_client.fetch_users().await })
    };
    let intune_devices_task = {
        let azure_client = Arc::clone(&azure_client);
        tokio::spawn(async move { azure_client.fetch_devices().await })
    };

    // collect contacts from netbox
    let nb_contacts_task = {
        let netbox_client = Arc::clone(&netbox_client);
        tokio::spawn(async move {
            netbox_client
                .get::<ContactList>("tenancy/contacts", None)
                .await
        })
    };

    // can add a print statement here to showcase the tasks above have not neccessarily started or
    // ran yet

    // await the data collection tasks before contiuning
    let (users_res, intune_devices_res, nb_contact_res) =
        tokio::join!(users_task, intune_devices_task, nb_contacts_task);

    let users = utils::extract_vec(users_res).await;
    let intune_devices = utils::extract_vec(intune_devices_res).await;

    // print all contacts from netbox
    println!("nb contact obj pre extraction: {:?}", nb_contact_res);
    let nb_contacts = nb_contact_res.unwrap();
    //let nb_contacts = utils::extract_vec(nb_contact_res).await;
    println!("nb contact obj: {:?}", nb_contacts);
    //println!("NetBox contact count: {}", nb_contacts.len());
    //for contact in nb_contacts {
    //  println!("NetBox contact: {:?}", contact);
    //}

    let prep_elapsed = prep_timer.elapsed();

    // Start parse timer
    let parse_timer = Instant::now();

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
*/
