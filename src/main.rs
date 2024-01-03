use std::time::Duration;

use unbounded_gpsd::{GpsdConnection, types::{TpvResponse, Response}};
use anyhow::{Result, Context, anyhow};
use streamdeck::{StreamDeck, TextPosition, TextOptions, Colour};
use rusttype::{Font, Scale};
use chrono::{DateTime, Utc};

const BUTTONS: [&str; 7] = ["fire_hydrant.png", "bicycle-parking.png",
    "corrosion.png", "bump.png", "szaglocso.png", "stop.png", "speed_display.png"];
const TIMEOUT: Duration = Duration::from_millis(10);

struct GpsInfo {
    lat: f64,
    lon: f64,
    time: DateTime<Utc>,
    speed: f64,
}

fn main() -> Result<()> {
    let font_data: &[u8] = include_bytes!("/Users/dnet/Library/Fonts/SourceSansPro-Bold.otf");
    let font: Font<'static> = Font::try_from_bytes(font_data).context("font from bytes")?;

    let db = sqlite::open("db.sqlite3")?;
    db.execute("CREATE TABLE IF NOT EXISTS pois (id INTEGER PRIMARY KEY AUTOINCREMENT,
        created DEFAULT CURRENT_TIMESTAMP, poi, lat, lon, gpstime);")?;
    let insert = "INSERT INTO pois (poi, lat, lon, gpstime) VALUES (?, ?, ?, ?);";
    let mut statement = db.prepare(insert)?;

    let mut sd = StreamDeck::connect(0x0fd9, 0x006d, None)?;
    sd.reset()?;
    for (key, image) in BUTTONS.iter().enumerate() {
        sd.set_button_file(key as u8, image, &streamdeck::images::ImageOptions::default())?;
    }

    let mut gpsd = GpsdConnection::new("127.0.0.1:2947").map_err(|e| anyhow!(e.to_string()))?;
    gpsd.set_read_timeout(Some(TIMEOUT)).map_err(|e| anyhow!(e.to_string()))?;
    gpsd.watch(true).map_err(|e| anyhow!(e.to_string()))?;

    let mut last_fix = None;
    loop {
        if let Ok(resp) = gpsd.get_response() {
            if let Response::Tpv(tpv) = resp {
                match tpv {
                    TpvResponse::Fix3D { device, time, mode, time_err, lat, lat_err, lon, lon_err, alt, alt_err, track, track_err, speed, speed_err, climb, climb_err } => last_fix = Some(GpsInfo { lat, lon, time, speed }),
                    TpvResponse::Fix2D { device, time, mode, time_err, lat, lat_err, lon, lon_err, track, track_err, speed, speed_err } => last_fix = Some(GpsInfo { lat, lon, time, speed }),
                    TpvResponse::LatLonOnly { device, time, mode, time_err, lat, lat_err, lon, lon_err, alt, alt_err, track, track_err, speed, speed_err, climb, climb_err } => if let Some(speed) = speed { last_fix = Some(GpsInfo { lat, lon, time, speed }) },
                    _ => (),
                }
            }
            let status = if let Some(ref details) = last_fix {
                format!("{} km/h\n{}", (details.speed * 3.6).round(), details.time.format("%H:%M:%S"))
            } else {
                "NO FIX".to_string()
            };
            sd.set_button_text(10, &font, &TextPosition::Absolute { x: 0, y: 0 }, &status, &TextOptions::new(
                Colour { r: 0xFF, g: 0x00, b: 0x00 },
                Colour { r: 0, g: 0, b: 0 },
                Scale { x: 16.0, y: 16.0 },
                1.0,
            ))?;
        }
        if let Some(ref details) = last_fix {
            if let Ok(btn) = sd.read_buttons(Some(TIMEOUT)) {
                for (key, image) in BUTTONS.iter().enumerate() {
                    if *btn.get(key).context("button by BUTTONS index")? == 1u8 {
                        statement.bind((1, *image))?;
                        statement.bind((2, details.lat))?;
                        statement.bind((3, details.lon))?;
                        statement.bind((4, details.time.timestamp()))?;
                        statement.next()?;
                        statement.reset()?;
                    }
                }
            }
        }
    }
    // TODO number of saved items? all / today?
}
