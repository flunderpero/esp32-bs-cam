use anyhow::Result;
use chrono::NaiveDateTime;
use embedded_svc::{http::Method, io::Write};
use esp_idf_hal::gpio::*;
use esp_idf_hal::prelude::Peripherals;
use esp_idf_svc::http::server::EspHttpServer;
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition};
use esp_idf_sys::{self as _, time, time_t};
use log::*;
use std::sync::{Arc, Mutex};
use std::{ptr, thread::sleep, time::Duration, ops::Deref};
use serde::{Serialize};
mod camera;
mod net;

const ERROR_THRESHOLD: u32 = 100;

fn main() {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    info!("Starting up ...");
    let peripherals = Peripherals::take().unwrap();
    // Light up built-in red LED during setup.
    let mut led = PinDriver::output(peripherals.pins.gpio33).unwrap();
    led.set_low().unwrap();
    let sys_loop = EspSystemEventLoop::take().unwrap();
    let nvs = EspDefaultNvsPartition::take().unwrap();
    let _wifi = net::init(sys_loop, peripherals.modem, nvs).unwrap();
    camera::init().unwrap();
    let _sntp = net::sntp().unwrap();
    let statistics = Arc::new(Mutex::new(Statistics {
        startup_at: now(),
        last_capture_at: None,
        last_capture_name: None,
        last_capture: None,
        capture_count: 0,
        capture_count_since_last_error: 0,
        last_error: None,
        error_count: 0,
    }));
    let _http_server = setup_http_server(statistics.clone()).unwrap();
    // Setup is done.
    info!("Ready");
    led.set_high().unwrap();
    loop {
        if let Err(error) = main_loop(statistics.clone()) {
            let mut stats = statistics.lock().unwrap();
            stats.error_count += 1;
            stats.capture_count_since_last_error =  0;
            if stats.error_count >= ERROR_THRESHOLD {
                panic!("There have been too many errors, restarting");
            }
            info!("That's an error - resuming operation: {}", error);
            stats.last_error = Some(format!("[{}]: {}", iso_format(now()), error));
            sleep(Duration::from_secs(2));
        }
    }
}

fn main_loop(statistics: Arc<Mutex<Statistics>>) -> Result<()> {
    let mut uploader = net::Uploader::create().unwrap();
    let mut last_capture_at = now();
    loop {
        // We want to capture at most 1 image per second.
        while now() - last_capture_at < 1 {
            info!("Wow, we're too fast, waiting a bit");
            sleep(Duration::from_millis(100));
        }
        last_capture_at = now();
        let data = camera::capture_image()?;
        let name = format!("test_{}.jpg", iso_format(last_capture_at));
        uploader.upload(&data[..], name.as_str())?;
        let mut stats = statistics.lock().unwrap();
        stats.last_capture = Some(data);
        stats.capture_count += 1;
        stats.capture_count_since_last_error += 1;
        stats.last_capture_at = Some(last_capture_at);
        stats.last_capture_name = Some(name);
    }
}

#[derive(Serialize)]
struct Statistics {
    startup_at: i64,
    last_capture_at: Option<i64>,
    last_capture_name: Option<String>,
    #[serde(skip_serializing)]
    last_capture: Option<Vec<u8>>,
    capture_count: u32,
    capture_count_since_last_error: u32,
    last_error: Option<String>,
    error_count: u32,
}

fn setup_http_server(statistics: Arc<Mutex<Statistics>>) -> Result<EspHttpServer> {
    let mut server = EspHttpServer::new(&Default::default())?;
    let stats1 = statistics.clone();
    server.fn_handler("/info", Method::Get, move |req| {
        let headers = [
            ("content-type", "application/json"),
            ("connection", "close"),
        ];
        let mut res = req.into_response(200, None, &headers)?;
        let stats = stats1.lock().unwrap();
        let json = serde_json::to_string_pretty(&stats.deref())?;
        res.write_all(json.as_bytes())?;
        res.flush()?;
        res.release();
        Ok(())
    })?;
    server.fn_handler("/image", Method::Get, move |req| {
        let stats = statistics.lock().unwrap();
        let data = stats.last_capture.clone();
        match data {
            Some(data) => {
                let content_length_header = format!("{}", data.len());
                let headers = [
                    ("content-type", "image/jpeg"),
                    ("connection", "close"),
                    ("content-length", &*content_length_header),
                ];
                let mut res = req.into_response(200, None, &headers)?;
                res.write_all(&data)?;
                res.flush()?;
            }
            None => {
                let headers = [("content-type", "text/plain"), ("connection", "close")];
                let mut res = req.into_response(200, None, &headers)?;
                res.write(b"No image has been captured yet")?;
                res.flush()?;
            }
        }
        Ok(())
    })?;
    Ok(server)
}

fn iso_format(timestamp: i64) -> String {
    let date = NaiveDateTime::from_timestamp_opt(timestamp, 0);
    match date {
        Some(date) => date.format("%Y%m%dT%H%M%SZ").to_string(),
        None => "<invalid date>".to_owned(),
    }
}

fn now() -> i64 {
    let timer: *mut time_t = ptr::null_mut();
    unsafe { time(timer) as i64 }
}
