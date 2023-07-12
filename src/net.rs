use anyhow::{bail, Result};
use embedded_svc::wifi::{ClientConfiguration, Configuration};
use embedded_svc::{http::client::Client, io::Read};
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_svc::sntp::{self, SyncStatus};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    http::client::{Configuration as HTTPConfiguration, EspHttpConnection},
    nvs::EspDefaultNvsPartition,
    wifi::EspWifi,
};
use log::info;
use std::{str, thread::sleep, time::Duration};

pub fn init(
    sys_loop: EspSystemEventLoop,
    modem: impl Peripheral<P = esp_idf_hal::modem::Modem> + 'static,
    nvs: EspDefaultNvsPartition,
) -> Result<Box<EspWifi<'static>>> {
    let wifi_ssid: &'static str = env!("BS_WIFI_SSID");
    let wifi_psk: &'static str = env!("BS_WIFI_PSK");
    info!("Connecting to WiFi...");
    let mut wifi = EspWifi::new(modem, sys_loop, Some(nvs))?;
    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: wifi_ssid.into(),
        password: wifi_psk.into(),
        ..Default::default()
    }))?;
    wifi.start()?;
    wifi.connect()?;
    while !wifi.is_connected()? {
        let config = wifi.get_configuration()?;
        info!("Waiting for WiFi connection: {:?}", config);
        sleep(Duration::from_secs(1));
    }
    loop {
        let ip_info = wifi.sta_netif().get_ip_info()?;
        info!("Waiting for IP address: {:?}", ip_info);
        if !ip_info.ip.is_unspecified() {
            break;
        }
        sleep(Duration::from_secs(1));
    }
    info!("WiFi connection established");
    Ok(Box::new(wifi))
}

pub fn sntp() -> Result<Box<sntp::EspSntp>> {
    info!("Initializing SNTP ...");
    let sntp = sntp::EspSntp::new_default()?;
    info!("SNTP waiting for status ...");
    while sntp.get_sync_status() != SyncStatus::Completed {
        info!(
            "Waiting for SNTP to be in sync: {:?}",
            sntp.get_sync_status()
        );
        sleep(Duration::from_secs(1));
    }
    info!("SNTP initialzied");
    Ok(Box::new(sntp))
}

pub struct Uploader {
    client: Client<EspHttpConnection>,
}

impl Uploader {
    pub fn create() -> Result<Uploader> {
        let connection = EspHttpConnection::new(&HTTPConfiguration {
            use_global_ca_store: true,
            crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_attach),
            timeout: Some(Duration::from_secs(20)),
            buffer_size: Some(100000),
            buffer_size_tx: Some(100000),
            ..Default::default()
        })?;
        let client = Client::wrap(connection);
        Ok(Uploader { client })
    }

    pub fn upload(&mut self, data: &[u8], name: &str) -> Result<()> {
        let base_url: String = env!("BS_UPLOAD_URL").to_string();
        let url = base_url + name;
        let content_length_header = format!("{}", data.len());
        let headers = [
            ("content-type", "image/jpeg"),
            // Don't use this keep-alive header here, it makes things
            // slower and slower over time.
            // ("connection", "keep-alive"),
            ("content-length", &*content_length_header),
        ];
        info!("Posting to URL: {}, content-length: {}", url, data.len());
        let mut req = self.client.post(url.as_ref(), &headers)?;
        let written = req.write(data)?;
        info!("{} bytes written", written);
        req.flush()?;
        let mut res = req.submit()?;
        let status = res.status();
        info!("Response code: {}\n", status);
        let mut buf = [0_u8; 256];
        loop {
            if let Ok(size) = Read::read(&mut res, &mut buf) {
                if size == 0 {
                    break;
                }
                let response_text =
                    str::from_utf8(&buf[..size]).unwrap_or("Failed to parse response");
                info!("Response: {}", response_text);
            }
        }
        res.release();
        match status {
            200..=299 => Ok(()),
            _ => bail!("Unexpected response code: {}", status),
        }
    }
}
