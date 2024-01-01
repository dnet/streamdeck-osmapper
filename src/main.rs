use unbounded_gpsd::GpsdConnection;
use anyhow::Result;
use streamdeck::StreamDeck;

fn main() -> Result<()> {
    //let gpsd = GpsdConnection::new("127.0.0.1:2947")?;
    println!("Hello, world!");
    let mut sd = StreamDeck::connect(0x0fd9, 0x006d, None)?;
    sd.reset()?;
    sd.set_button_file(0, "fire_hydrant.png", &streamdeck::images::ImageOptions::default())?;
    sd.set_button_file(1, "bicycle-parking.png", &streamdeck::images::ImageOptions::default())?;
    sd.set_button_file(2, "corrosion.png", &streamdeck::images::ImageOptions::default())?;
    sd.set_button_file(3, "bump.png", &streamdeck::images::ImageOptions::default())?;
    sd.set_button_file(4, "szaglocso.png", &streamdeck::images::ImageOptions::default())?;
    sd.set_button_file(5, "stop.png", &streamdeck::images::ImageOptions::default())?;
    // TODO time, number of sats, speed into text tiles
    // TODO watch buttons
    // TODO poll gpsd
    // TODO number of saved items? all / today?
    let btn = sd.read_buttons(None)?;
    dbg!(btn);
    //dbg!(&sd);
    Ok(())
}
