mod ips;
use ips::Ips;

use druid::{im::Vector, Data, Lens};
use serde::Deserialize;
use sha1::{Digest, Sha1};
use std::sync::Arc;

pub const PATH_MODLIST: &str = "mods.toml";
pub const URL_MODLIST: &str =
    "https://raw.githubusercontent.com/BfBBModdingTools/chum_bucket_lab/master/mods.toml";

pub const PATH_ROM: &str = "baserom/default.xbe";
const PATH_OUTPUT: &str = "output";

#[derive(Clone, Data, Lens)]
pub struct AppData {
    #[data(ignore)]
    pub modlist: Arc<ModList>,
    pub enabled_mods: Vector<bool>,
    pub selected_mod: Option<usize>,
    pub response: String,
}

impl AppData {
    pub fn new(modlist: ModList) -> AppData {
        AppData {
            selected_mod: if modlist.mods.is_empty() {
                None
            } else {
                Some(0)
            },
            enabled_mods: Vector::from(vec![false; modlist.mods.len()]),
            modlist: Arc::new(modlist),
            response: String::with_capacity(256),
        }
    }
}

#[derive(Deserialize)]
pub struct ModList {
    pub mods: Vec<Mod>,
}

impl std::ops::Deref for ModList {
    type Target = Vec<Mod>;

    fn deref(&self) -> &Self::Target {
        &self.mods
    }
}

// TOOD: Consider not needing PartialEq
#[derive(Debug, Data, Clone, PartialEq, Lens, Deserialize)]
pub struct Mod {
    pub name: String,
    pub author: String,
    pub description: String,
    pub website_url: String,
    pub download_url: String,
}

impl Mod {
    // TODO: This returns failure status codes (Like 404)
    pub fn download(&self) -> Result<Vec<u8>, reqwest::Error> {
        let response = reqwest::blocking::get(&self.download_url)?
            .bytes()?
            .to_vec();
        Ok(response)
    }
}

pub struct Rom {
    pub bytes: Vec<u8>,
}

impl Rom {
    const XBE_SHA1: &'static [u8] = &[
        0xa9, 0xac, 0x85, 0x5c, 0x4e, 0xe8, 0xb4, 0x1b, 0x66, 0x1c, 0x35, 0x78, 0xc9, 0x59, 0xc0,
        0x24, 0xf1, 0x06, 0x8c, 0x47,
    ];

    pub fn new() -> Result<Self, std::io::Error> {
        let bytes = std::fs::read(PATH_ROM)?;
        if !Rom::verify_hash(&bytes) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Hash Does Not Match",
            ));
        }

        Ok(Rom { bytes })
    }

    pub fn verify_hash(bytes: &[u8]) -> bool {
        let mut hasher = Sha1::new();
        hasher.update(&bytes);
        let hash = hasher.finalize();
        *Rom::XBE_SHA1 == hash[..]
    }

    pub fn export(&self) -> Result<(), std::io::Error> {
        std::fs::create_dir_all(PATH_OUTPUT)?;
        std::fs::write(PATH_OUTPUT.to_owned() + "/default.xbe", &self.bytes)
    }
}

pub struct Patch {
    ips_file: Ips,
}

impl Patch {
    pub fn new(bytes: Vec<u8>) -> Self {
        Patch {
            ips_file: Ips::new(bytes),
        }
    }

    // TODO: return Result of some sort
    pub fn apply_to(&mut self, rom: &mut Rom) -> Result<(), std::io::Error> {
        self.ips_file.apply_to(&mut rom.bytes)?;
        Ok(())
    }
}
