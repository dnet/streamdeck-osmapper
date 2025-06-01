use std::fs::File;
use std::time::Duration;
use std::io::prelude::*;

use configparser::ini::Ini;
use sqlite::{Connection, State};
use unbounded_gpsd::{GpsdConnection, types::{TpvResponse, Response}};
use anyhow::{Result, Context, anyhow};
use streamdeck::{StreamDeck, TextPosition, TextOptions, Colour};
use rusttype::{Font, Scale};
use chrono::{DateTime, Utc};
use askama::Template;

const BUTTON_COUNT: usize = 19;
const BUTTONS_PER_PAGE: usize = 13;

const BUTTONS: [&str; BUTTON_COUNT] = ["fire_hydrant.png", "bicycle-parking.png",
    "corrosion.png", "bump.png", "szaglocso.png", "stop.png", "speed_display.png",
    "waste-basket.png", "parking-ticket-vending.png", "kick-scooter-parking.png",
    "bench.png", "hunting-stand.png", "post-box.png",
    "camera.png", "kotras.png", "zebra.png", "taxi.png", "recycling.png", "substation.png"];
const TIMEOUT: Duration = Duration::from_millis(10);
const TOP_LEFT: TextPosition = TextPosition::Absolute { x: 0, y: 0 };
const RED: Colour = Colour { r: 0xFF, g: 0x00, b: 0x00 };
const GREEN: Colour = Colour { r: 0x00, g: 0xFF, b: 0x00 };
const BLACK: Colour = Colour { r: 0, g: 0, b: 0 };

struct GpsInfo {
    lat: f64,
    lon: f64,
    time: DateTime<Utc>,
    speed: f64,
}

struct Node {
    lat: String,
    lon: String,
    id: String,
    created: String,
    rules: Vec<(String, String)>,
}

#[derive(Template)]
#[template(path = "osm.xml")]
struct OsmExport {
    minlat: String,
    minlon: String,
    maxlat: String,
    maxlon: String,
    nodes: Vec<Node>,
}

fn main() -> Result<()> {
    let mut args = std::env::args();
    args.next(); // exec
    let db = sqlite::open("db.sqlite3")?;
    if let Some(output_file) = args.next() {
        dump_mode(&output_file, db)
    } else {
        streamdeck_mode(db)
    }
}

fn dump_mode(output_file: &str, db: Connection) -> Result<()> {
    let mut config = Ini::new();
    config.set_comment_symbols(&[]);
    let rules = config.load("osm.ini").map_err(|e| anyhow!(e))?;
    let mut nodes = Vec::new();
    let mut statement = db.prepare("SELECT MIN(lat), MIN(lon), MAX(lat), MAX(lon) FROM pois;")?;
    if statement.next()? == State::Row {
        let minlat = statement.read::<String, _>(0)?;
        let minlon = statement.read::<String, _>(1)?;
        let maxlat = statement.read::<String, _>(2)?;
        let maxlon = statement.read::<String, _>(3)?;
        let mut statement = db.prepare("SELECT id, lat, lon, created, poi FROM pois;")?;
        while statement.next()? == State::Row {
            nodes.push(Node {
                id: statement.read::<String, _>(0)?,
                lat: statement.read::<String, _>(1)?,
                lon: statement.read::<String, _>(2)?,
                created: statement.read::<String, _>(3)?,
                rules: rules[&statement.read::<String, _>(4)?].iter().map(|(k, v)| (k.clone(), v.clone().unwrap())).collect(),
            });
        }
        let export = OsmExport { minlat, minlon, maxlat, maxlon, nodes };
        let output = export.render()?;
        let mut file = File::create(output_file)?;
        file.write_all(output.as_bytes())?;
    }
    Ok(())
}

fn streamdeck_mode(db: Connection) -> Result<()> {
    let font_data: &[u8] = include_bytes!("/Users/dnet/Library/Fonts/SourceSansPro-Bold.otf");
    let font: Font<'static> = Font::try_from_bytes(font_data).context("font from bytes")?;
    let text16 = TextOptions::new(
        RED, BLACK, Scale { x: 16.0, y: 16.0 }, 1.0);
    let text32 = TextOptions::new(
        GREEN, BLACK, Scale { x: 32.0, y: 32.0 }, 1.0);
    let mut page = 0usize;
    let mut display = [None; BUTTONS_PER_PAGE];
    let pages = BUTTON_COUNT.div_ceil(BUTTONS_PER_PAGE);

    db.execute("CREATE TABLE IF NOT EXISTS pois (id INTEGER PRIMARY KEY AUTOINCREMENT,
        created DEFAULT CURRENT_TIMESTAMP, poi, lat, lon, gpstime);")?;
    let mut statement = db.prepare("INSERT INTO pois (poi, lat, lon, gpstime) VALUES (?, ?, ?, ?);")?;
    let mut count_all = db.prepare("SELECT COUNT(*) AS c FROM pois;")?;
    let mut count_today = db.prepare("SELECT COUNT(*) AS c FROM pois WHERE date(created) = date();")?;

    let mut sd = StreamDeck::connect(0x0fd9, 0x006d, None)?;
    sd.reset()?;
    refresh_page(page, &mut display, &mut sd)?;

    let mut gpsd = GpsdConnection::new("127.0.0.1:2947").map_err(|e| anyhow!(e.to_string()))?;
    gpsd.set_read_timeout(Some(TIMEOUT)).map_err(|e| anyhow!(e.to_string()))?;
    gpsd.watch(true).map_err(|e| anyhow!(e.to_string()))?;

    update_counters(&mut count_all, &mut count_today, &mut sd, &font, &text32)?;

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
                format!("{} km/h\n{}\n<<", (details.speed * 3.6).round(), details.time.format("%H:%M:%S"))
            } else {
                if let Some(addr) = dbg!(interfaces::Interface::get_by_name("en0"))?.unwrap()
                        .addresses.iter().find(|a| a.kind == interfaces::Kind::Ipv4) {
                    dbg!(addr).addr.unwrap().ip().to_string().replace('.', ".\n")
                } else {
                    "NO FIX\nNO ADDR".to_string()
                }
            };
            sd.set_button_text(13, &font, &TOP_LEFT, &status, &text16)?;
        }
        if let Some(ref details) = last_fix {
            if let Ok(btn) = sd.read_buttons(Some(TIMEOUT)) {
                for (key, image) in display.iter().enumerate() {
                    if let Some(name) = image {
                        if *btn.get(key).context("button by BUTTONS index")? == 1u8 {
                            statement.bind((1, **name))?;
                            statement.bind((2, details.lat))?;
                            statement.bind((3, details.lon))?;
                            statement.bind((4, details.time.timestamp()))?;
                            while statement.next()? != State::Done { }
                            statement.reset()?;
                            update_counters(&mut count_all, &mut count_today, &mut sd, &font, &text32)?;
                        }
                    }
                }
                if *btn.get(13).context("back button")? == 1u8 {
                    page = if page == 0 { pages - 1 } else { page - 1 };
                    refresh_page(page, &mut display, &mut sd)?;
                }
                if *btn.get(14).context("forward button")? == 1u8 {
                    page = if page == pages - 1 { 0 } else { page + 1 };
                    refresh_page(page, &mut display, &mut sd)?;
                }
            }
        }
    }
}

fn refresh_page(page: usize, display: &mut [Option<&&str>; 13], sd: &mut StreamDeck) -> Result<()> {
    let first = page * BUTTONS_PER_PAGE;
    let next = (page + 1) * BUTTONS_PER_PAGE;
    let last = std::cmp::min(next, BUTTON_COUNT);
    for (key, image) in BUTTONS[first..last].iter().enumerate() {
        display[key] = Some(image);
        sd.set_button_file(key as u8, image, &streamdeck::images::ImageOptions::default())?;
    }
    let offset = last - first;
    for key in offset..(offset + next - last) {
        display[key] = None;
        sd.set_button_rgb(key as u8, &BLACK)?;
    }
    Ok(())
}

fn update_counters(count_all: &mut sqlite::Statement<'_>, count_today: &mut sqlite::Statement<'_>,
                   sd: &mut StreamDeck, font: &Font<'_>, text32: &TextOptions) -> Result<()> {
    if count_all.next()? == State::Row && count_today.next()? == State::Row {
        let num_all = count_all.read::<i64, _>(0)?;
        let num_today = count_today.read::<i64, _>(0)?;
        let status = format!("{num_today}\n{num_all}\n>>");
        sd.set_button_text(14, font, &TOP_LEFT, &status, &text32)?;
    }
    count_all.reset()?;
    count_today.reset()?;
    Ok(())
}
