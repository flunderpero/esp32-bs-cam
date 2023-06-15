// This is needed because we use the `binstart` feature of `esp-idf-sys`.
use anyhow::{Context, Result};
use esp_idf_hal::gpio::*;
use esp_idf_hal::prelude::Peripherals;
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition};
use esp_idf_sys as _;
use log::*;
use std::{thread::sleep, time::Duration};
mod camera;
mod net;

fn main() -> Result<()> {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    info!("Starting up ...");
    let sys_loop = EspSystemEventLoop::take().context("Failed to get `sys_loop`")?;
    let peripherals = Peripherals::take().context("Failed to get `peripherals`")?;

    let nvs = EspDefaultNvsPartition::take()?;
    let mut led = PinDriver::output(peripherals.pins.gpio4)?;
    led.set_high()?;
    let _wifi = net::init(sys_loop.clone(), peripherals.modem, nvs.clone())?;
    camera::init()?;
    led.set_low()?;
    info!("Ready");
    let data = b"Test";
    net::upload(data, "test")?;
    loop {
        info!("Just sleeping ...");
        sleep(Duration::new(10, 0));
    }
}
