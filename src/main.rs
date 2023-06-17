use anyhow::{Context, Result};
use chrono::NaiveDateTime;
use embedded_svc::{http::Method, io::Write};
use esp_idf_hal::gpio::*;
use esp_idf_hal::prelude::Peripherals;
use esp_idf_svc::http::server::EspHttpServer;
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition};
use esp_idf_sys::{self as _, time, time_t};
use log::*;
use std::sync::{Arc, Mutex};
use std::{ptr, thread::sleep, time::Duration};
mod camera;
mod net;

fn main(){
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    info!("Starting up ...");
    let peripherals = Peripherals::take().context("Failed to get `peripherals`").unwrap();
    // Light up built-in red LED during setup.
    let mut led = PinDriver::output(peripherals.pins.gpio33).unwrap();
    led.set_low().unwrap();
    let sys_loop = EspSystemEventLoop::take().context("Failed to get `sys_loop`").unwrap();
    let nvs = EspDefaultNvsPartition::take().unwrap();
    let _wifi = net::init(sys_loop, peripherals.modem, nvs).unwrap();
    camera::init().unwrap();
    let _sntp = net::sntp().unwrap();
    let statistics = Arc::new(Mutex::new(Statistics {
        last_capture_date_time: None,
        last_capture_name: None,
        capture_count: 0,
    }));
    let _http_server = setup_http_server(statistics.clone()).unwrap();
    let mut uploader = net::Uploader::create().unwrap();
    // Setup is done.
    info!("Ready");
    led.set_high().unwrap();
    loop {
        let data = camera::capture_image().unwrap();
        let now = now().unwrap();
        let name = format!("test_{}.jpg", iso_format(now));
        uploader.upload(&data[..], name.as_str()).unwrap();
        let mut stats = statistics.lock().unwrap();
        stats.capture_count += 1;
        stats.last_capture_date_time = Some(now);
        stats.last_capture_name = Some(name);
        sleep(Duration::from_millis(100));
    }
}

struct Statistics {
    last_capture_date_time: Option<NaiveDateTime>,
    last_capture_name: Option<String>,
    capture_count: u32,
}

fn setup_http_server(statistics: Arc<Mutex<Statistics>>) -> Result<EspHttpServer> {
    let mut server = EspHttpServer::new(&Default::default())?;
    server.fn_handler("/info", Method::Get, move |req| {
        let headers = [
            ("content-type", "application/json"),
            ("connection", "close"),
        ];
        let mut res = req.into_response(200, None, &headers)?;
        let stats = statistics.lock().unwrap();
        let last_capture_date_time = match stats.last_capture_date_time {
            Some(date) => iso_format(date),
            None => "".to_string(),
        };
        let now = iso_format(now()?);
        let json = format!(
            "{{\n\"date_time\": \"{}\"\n\"last_capture_date_time\": \"{}\"\n\"capture_count\": {}\n}}",
            now, last_capture_date_time, stats.capture_count
        );
        res.write_all(json.as_bytes())?;
        res.flush()?;
        res.release();
        Ok(())
    })?;
    server.fn_handler("/image", Method::Get, move |req| {
        let data = &camera::capture_image()?[..];
        let content_length_header = format!("{}", data.len());
        let headers = [
            ("content-type", "image/jpeg"),
            ("connection", "close"),
            ("content-length", &*content_length_header),
        ];
        let mut res = req.into_response(200, None, &headers)?;
        res.write_all(data)?;
        res.flush()?;
        res.release();
        Ok(())
    })?;
    Ok(server)
}

fn iso_format(date: NaiveDateTime) -> String {
    date.format("%Y%m%dT%H%M%SZ").to_string()
}

fn now() -> Result<NaiveDateTime> {
    unsafe {
        let timer: *mut time_t = ptr::null_mut();
        let timestamp = time(timer);
        let date_time = NaiveDateTime::from_timestamp_opt(timestamp as i64, 0)
            .context("Unable to parse timestamp")?;
        Ok(date_time)
    }
}
