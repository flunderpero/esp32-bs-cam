// This is needed because we use the `binstart` feature of `esp-idf-sys`.
use esp_idf_sys as _;
use log::*;
mod camera;
mod net;
use anyhow::{Error, Result};
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_hal::prelude::Peripherals;
use std::{thread::sleep, time::Duration};

fn main() {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    info!("Starting up ...");
    let peripherals = Peripherals::take().unwrap();
    let sys_loop = EspSystemEventLoop::take().unwrap();
    net::init(sys_loop, peripherals).or_else(error).unwrap();
    camera::init().or_else(error).unwrap();
    info!("Ready");
    loop {
        info!("Just sleeping ...");
        sleep(Duration::new(10, 0));
    }
}

fn error(err: Error) -> Result<()> {
    error!("Startup failed: {}", err);
    return Err(err);
}
