use anyhow::{Result};
use embedded_svc::wifi::{ClientConfiguration, Configuration};
use esp_idf_hal::{peripheral::Peripheral};
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition, wifi::EspWifi};
use log::info;
use std::{thread::sleep, time::Duration};

pub fn init(
    sys_loop: EspSystemEventLoop,
    modem: impl Peripheral<P = esp_idf_hal::modem::Modem> + 'static,
    nvs: EspDefaultNvsPartition
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
