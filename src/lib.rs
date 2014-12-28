#![feature(slicing_syntax)]

extern crate xdg;
extern crate toml;
extern crate "rustc-serialize" as rustc_serialize;

use rustc_serialize::Decodable;
use std::io::File;
use xdg::XdgDirs;

pub fn load_config<C>() -> Option<C>
    where C: Decodable<toml::Decoder, toml::DecodeError> {
    XdgDirs::new().want_read_config("pushbullet/creds.toml")
        .and_then(|ref p| File::open(p).ok())
        .and_then(|mut f| f.read_to_string().ok())
        .and_then(|s| toml::decode_str(s[]))
}

