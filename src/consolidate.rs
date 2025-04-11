use crate::{
    fetch::{azure::IntuneDevice, fortigate::FortiGateDevice},
    netbox::models::Device,
};

fn normalize_mac(mac: &str) -> String {
    mac.to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .collect()
}
