pub mod data;
pub mod ui;

use druid::{AppLauncher, LocalizedString, WindowDesc};

use data::{AppData, Mod};
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

    fn parse_args<I>(args: I) -> Self
    where
        I: Iterator<Item = String>,
    {
        let args: Vec<_> = args.collect();
        if args.len() < 3 {
            return Config::DEFAULT_CONFIG.to_owned();
        }

        if args[1] != Config::OPTION_UPDATE {
            return Config::DEFAULT_CONFIG.to_owned();
        }

        match args[2].parse::<bool>() {
            Err(_) => Config::DEFAULT_CONFIG.to_owned(),
            Ok(b) => Config { check_update: b },
        }
    }
}

pub fn main() {
    // Get config from command line args
    let config = Config::parse_args(env::args());

    let main_window = WindowDesc::new(ui::ui_builder())
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
        .log_to_console()
        .launch(AppData::new(modlist))
        .expect("launch failed");
}

fn update_modlist() {
    match reqwest::blocking::get(data::URL_MODLIST).and_then(|r| r.text()) {
        Ok(text) => {
            if fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(data::PATH_MODLIST)
                .and_then(|mut f| f.write_all(text.as_bytes()))
                .is_err()
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
    let file = fs::read_to_string(data::PATH_MODLIST)?;

    match data::modlist_from_toml(&file) {
        Err(e) => {
            eprintln!("{}", e);
            Err(Error::new(ErrorKind::InvalidData, e)) //Failed to deserialize file
        }
        Ok(modlist) => Ok(modlist),
    }
}
