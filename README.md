# A Simple ESP32 Camera Thing In Rust

Take an image once a second and upload it.

## Development

### Setup

Follow https://esp-rs.github.io/book/installation/index.html but make
sure you install version 2 of `espflash`:

     cargo install espflash --version 2.0.0-rc.4

### Building

Before building you have to source `../export-esp.sh`.

The following environment variables drive the configuration:

`BS_WIFI_SSID`
: The SSID to connect to.

`BS_WIFI_PSK`
: The passphrase.

`BS_UPLOAD_URL`
: Where to upload images to via a POST request. 
: The name of the image file will be appended to this URL.

`BS_HTTP_SERVER`
: If set, a basic monitoring HTTP server will be started at port 80
: with the routes `/info` and `/image`.
