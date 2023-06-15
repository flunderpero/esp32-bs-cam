use anyhow::{bail, Result};
use embedded_svc::io::Write;
use embedded_svc::wifi::{ClientConfiguration, Configuration};
use embedded_svc::{http::client::Client, io::Read};
use esp_idf_hal::peripheral::Peripheral;
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
        sleep(Duration::new(1, 0));
    }
    loop {
        let ip_info = wifi.sta_netif().get_ip_info()?;
        info!("Waiting for IP address: {:?}", ip_info);
        if !ip_info.ip.is_unspecified() {
            break;
        }
        sleep(Duration::new(1, 0));
    }
    info!("WiFi connection established");
    return Ok(Box::new(wifi));
}

pub fn upload(data: &[u8], name: &str) -> Result<()> {
    let connection = EspHttpConnection::new(&HTTPConfiguration {
        use_global_ca_store: true,
        crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_attach),
        ..Default::default()
    })?;
    let mut client = Client::wrap(connection);
    let base_url: String = env!("BS_UPLOAD_URL").to_string();
    let url = base_url + name;
    let content_length_header = format!("{}", data.len());
    let headers = [
        ("accept", "text/plain"),
        ("content-type", "text/plain"),
        ("connection", "close"),
        ("content-length", &*content_length_header),
    ];
    info!("Posting to URL: {}", url);
    let mut request = client.post(url.as_ref(), &headers)?;
    request.write_all(data)?;
    request.flush()?;
    let response = request.submit()?;
    let status = response.status();
    info!("Response code: {}\n", status);
    let mut buf = [0_u8; 256];
    let mut reader = response;
    loop {
        if let Ok(size) = Read::read(&mut reader, &mut buf) {
            if size == 0 {
                break;
            }
            // 5. try converting the bytes into a Rust (UTF-8) string and print it
            let response_text = str::from_utf8(&buf[..size])?;
            info!("{}", response_text);
        }
    }
    client.release();
    match status {
        200..=299 => Ok(()),
        _ => bail!("Unexpected response code: {}", status),
    }
}
