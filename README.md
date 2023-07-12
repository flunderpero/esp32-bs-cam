# A Simple ESP32 Camera Thing In Rust

Take an image once a second and upload it.

## Development

### Setup

First run:

     cargo install cargo-make
     cargo make install

This will setup the environment according to 
https://esp-rs.github.io/book/installation/index.html 

### Building

Before building you have to source `~/export-esp.sh`:

    source ~/export-esp.sh
    cargo make build
    cargo make flash

The following environment variables drive the configuration:

`BS_WIFI_SSID`
: The SSID to connect to.

`BS_WIFI_PSK`
: The passphrase.

`BS_UPLOAD_URL`
: Where to upload images to via a POST request. 
: The name of the image file will be appended to this URL.
: Example (GCS with public access): https://storage.googleapis.com/upload/storage/v1/b/<BUCKET>/o?uploadType=media&name=

`BS_HTTP_SERVER`
: If set, a basic monitoring HTTP server will be started at port 80
: with the routes `/info` and `/image`.
