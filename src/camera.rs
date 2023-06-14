use esp_idf_sys::camera::*;
use log::*;

pub fn init() {
    let camera_config = esp_idf_sys::camera::camera_config_t {
        pin_pwdn: 32,
        pin_reset: -1,
        pin_xclk: 0,
        pin_d7: 35,
        pin_d6: 34,
        pin_d5: 39,
        pin_d4: 36,
        pin_d3: 21,
        pin_d2: 19,
        pin_d1: 18,
        pin_d0: 5,
        pin_vsync: 25,
        pin_href: 23,
        pin_pclk: 22,
        // The following two field are actually `pin_sccb_sda` and `pin_sccb_scl` but
        // the idf bindgen was not able to handle the definition in `esp_camera.h`.
        __bindgen_anon_1: camera_config_t__bindgen_ty_1 { pin_sccb_sda: 26 },
        __bindgen_anon_2: camera_config_t__bindgen_ty_2 { pin_sccb_scl: 27 },
        sccb_i2c_port: 0, // Unused

        //XCLK 20MHz or 10MHz for OV2640 double FPS (Experimental)
        xclk_freq_hz: 20000000,
        ledc_timer: esp_idf_sys::camera::ledc_timer_t_LEDC_TIMER_0,
        ledc_channel: esp_idf_sys::camera::ledc_channel_t_LEDC_CHANNEL_0,

        pixel_format: esp_idf_sys::camera::pixformat_t_PIXFORMAT_JPEG, //YUV422,GRAYSCALE,RGB565,JPEG
        frame_size: esp_idf_sys::camera::framesize_t_FRAMESIZE_QVGA, //QQVGA-UXGA Do not use sizes above QVGA when not JPEG

        jpeg_quality: 12, //0-63 lower number means higher quality
        fb_count: 1,      //if more than one, i2s runs in continuous mode. Use only with JPEG
        fb_location: esp_idf_sys::camera::camera_fb_location_t_CAMERA_FB_IN_PSRAM,
        grab_mode: esp_idf_sys::camera::camera_grab_mode_t_CAMERA_GRAB_WHEN_EMPTY,
    };
    if unsafe { esp_idf_sys::camera::esp_camera_init(&camera_config) } != 0 {
        info!("camera init failed!");
        return;
    } else {
        info!("camera ready! >>>>>>>>>>>>>>>>>>>>>>>>>>>>");
    }
}
