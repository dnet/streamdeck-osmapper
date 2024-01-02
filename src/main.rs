use unbounded_gpsd::{GpsdConnection, types::{TpvResponse, Response}};
use anyhow::{Result, Context, anyhow};
use streamdeck::{StreamDeck, TextPosition, TextOptions, Colour};
use rusttype::{Font, Scale};

const BUTTONS: [&str; 6] = ["fire_hydrant.png", "bicycle-parking.png",
    "corrosion.png", "bump.png", "szaglocso.png", "stop.png"]; // TODO add speed meter w/ 7seg display

fn main() -> Result<()> {
    let font_data: &[u8] = include_bytes!("/Users/dnet/Library/Fonts/SourceSansPro-Bold.otf");
    let font: Font<'static> = Font::try_from_bytes(font_data).context("font from bytes")?;

    let mut sd = StreamDeck::connect(0x0fd9, 0x006d, None)?;
    sd.reset()?;
    for (key, image) in BUTTONS.iter().enumerate() {
        sd.set_button_file(key as u8, image, &streamdeck::images::ImageOptions::default())?;
    }

    let mut gpsd = GpsdConnection::new("127.0.0.1:2947").map_err(|e| anyhow!(e.to_string()))?;
    gpsd.set_read_timeout(None).map_err(|e| anyhow!(e.to_string()))?;
    let resp = gpsd.get_response().map_err(|e| anyhow!(e.to_string()))?;
    dbg!(&resp);
    gpsd.watch(true).map_err(|e| anyhow!(e.to_string()))?;

    loop {
        let resp = gpsd.get_response().map_err(|e| anyhow!(e.to_string()))?;
        let status: String = if let Response::Tpv(tpv) = resp {
            match tpv {
                TpvResponse::Fix3D { device, time, mode, time_err, lat, lat_err, lon, lon_err, alt, alt_err, track, track_err, speed, speed_err, climb, climb_err } => format!("{} km/h\n{}", (speed * 3.6).round(), time.format("%H:%M:%S")),
                TpvResponse::Fix2D { device, time, mode, time_err, lat, lat_err, lon, lon_err, track, track_err, speed, speed_err } => format!("{} km/h\n{}", (speed * 3.6).round(), time.format("%H:%M:%S")),
                TpvResponse::LatLonOnly { device, time, mode, time_err, lat, lat_err, lon, lon_err, alt, alt_err, track, track_err, speed, speed_err, climb, climb_err } => format!("{} km/h\n{}", (speed.unwrap_or(0.0) * 3.6).round(), time.format("%H:%M:%S")),
                TpvResponse::NoFix { device, time, mode } => format!("NO FIX\n{}", time.format("%H:%M:%S")),
                TpvResponse::Nothing { device, time, mode } => time.and_then(|t| Some(t.format("%H:%M:%S").to_string())).unwrap_or_else(|| "Nothing".to_string()),
                TpvResponse::Dustbin { device, time, mode, time_err, lat, lat_err, lon, lon_err, alt, alt_err, track, track_err, speed, speed_err, climb, climb_err } => time.and_then(|t| Some(t.format("%H:%M:%S").to_string())).unwrap_or_else(|| "Dustbin".to_string()),
            }
        } else { continue; };
        sd.set_button_text(10, &font, &TextPosition::Absolute { x: 0, y: 0 }, &status, &TextOptions::new(
            Colour { r: 0xFF, g: 0x00, b: 0x00 },
            Colour { r: 0, g: 0, b: 0 },
            Scale { x: 16.0, y: 16.0 },
            1.0,
        ))?;
    }
    // TODO watch buttons
    // TODO number of saved items? all / today?
    //let btn = sd.read_buttons(None)?;
    //dbg!(btn);
    //dbg!(&sd);
    Ok(())
}
