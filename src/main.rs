pub mod data;
pub mod linker;
pub mod ui;

use druid::{im::Vector, AppLauncher, LocalizedString, WindowDesc};

use data::{AppData, Mod};
use linker::xbe;
use std::io::{Error, ErrorKind, Write};
use std::{env, fs};

const WINDOW_TITLE: &str = "chum_bucket_lab";

#[derive(Clone, Debug)]
struct Config {
    check_update: bool,
}
impl Config {
    const DEFAULT_CONFIG: Config = Config { check_update: true };
    const OPTION_UPDATE: &'static str = "--update";

    fn new(args: &[String]) -> Self {
        if args.len() < 3 {
            return Config::DEFAULT_CONFIG.to_owned();
        }

        if &args[1] != Config::OPTION_UPDATE {
            return Config::DEFAULT_CONFIG.to_owned();
        }

        match &args[2].parse::<bool>() {
            Err(_) => Config::DEFAULT_CONFIG.to_owned(),
            Ok(b) => Config { check_update: *b },
        }
    }
}

pub fn main() {
    //TEMPORARY
    let mut xbe = xbe::XBE::new("baserom/default.xbe");

    linker::test(&mut xbe);
    xbe.write_to_file("output/default.xbe");

    // Get config from command line args
    let args: Vec<String> = env::args().collect();
    let config = Config::new(&args);

    let main_window = WindowDesc::new(ui::ui_builder)
        .title(LocalizedString::new(WINDOW_TITLE).with_placeholder("Chum Bucket Lab"));

    if config.check_update {
        update_modlist();
    }

    //TODO: Error prompt when this fails
    let modlist = parse_modlist().unwrap_or_else(|_| {
        println!("Failed to parse modlist");
        Vec::new()
    });

    AppLauncher::with_window(main_window)
        .delegate(ui::Delegate)
        .use_simple_logger()
        .launch(AppData::new(Vector::from(modlist)))
        .expect("launch failed");
}

fn update_modlist() {
    match reqwest::blocking::get(data::URL_MODLIST).and_then(|r| r.text()) {
        Ok(text) => {
            if let Err(_) = fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(data::PATH_MODLIST)
                .and_then(|mut f| f.write_all(text.as_bytes()))
            {
                println!("Failed to write updated file to disk");
            }
        }
        Err(_) => println!("Failed to retrieve modslist from internet"),
    }
}

fn parse_modlist() -> std::io::Result<Vec<Mod>> {
    // TODO: Consider if saving the modlist locally is even necessary
    // Note: Keeping a local copy enables the app to still function even
    // if we fail to download latest version even though the user has a valid
    // internet connection
    let file = fs::File::open(data::PATH_MODLIST)?;

    match serde_json::from_reader::<_, Vec<Mod>>(file) {
        Err(e) => Err(Error::new(ErrorKind::InvalidData, e)), //Failed to deserialize file
        Ok(modlist) => Ok(modlist),
    }
}
