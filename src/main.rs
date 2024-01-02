use unbounded_gpsd::GpsdConnection;
use anyhow::Result;
use streamdeck::StreamDeck;

const BUTTONS: [&str; 6] = ["fire_hydrant.png", "bicycle-parking.png",
    "corrosion.png", "bump.png", "szaglocso.png", "stop.png"];

fn main() -> Result<()> {
    //let gpsd = GpsdConnection::new("127.0.0.1:2947")?;
    println!("Hello, world!");
    let mut sd = StreamDeck::connect(0x0fd9, 0x006d, None)?;
    sd.reset()?;
    for (key, image) in BUTTONS.iter().enumerate() {
        sd.set_button_file(key as u8, image, &streamdeck::images::ImageOptions::default())?;
    }
    // TODO time, number of sats, speed into text tiles
    // TODO watch buttons
    // TODO poll gpsd
    // TODO number of saved items? all / today?
    let btn = sd.read_buttons(None)?;
    dbg!(btn);
    //dbg!(&sd);
    Ok(())
}
