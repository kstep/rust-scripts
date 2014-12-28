#![feature(slicing_syntax)]

extern crate pb;
extern crate xdg;
extern crate toml;
extern crate "rustc-serialize" as rustc_serialize;

use pb::api::PbAPI;
use pb::messages::{PushMsg, TargetIden};
use pb::objects::{Push, PushData};
use std::os::getenv;
use std::io::File;
use xdg::XdgDirs;
use rustc_serialize::Decodable;

#[deriving(RustcDecodable)]
struct Config {
    access_token: String
}

fn load_config<C>() -> Option<C>
    where C: Decodable<toml::Decoder, toml::DecodeError> {
    XdgDirs::new().want_read_config("pushbullet/creds.toml")
        .and_then(|ref p| File::open(p).ok())
        .and_then(|mut f| f.read_to_string().ok())
        .and_then(|s| toml::decode_str(s[]))
}

fn main() {
    let api = PbAPI::new(load_config::<Config>().unwrap().access_token[]);
    let torrent_name = getenv("TR_TORRENT_NAME").unwrap();
    let torrent_dir = getenv("TR_TORRENT_DIR").unwrap();
    let push = PushMsg {
        title: Some("Torrent download complete".to_string()),
        body: Some(format!("{} downloaded to {}", torrent_name, torrent_dir)),
        target: TargetIden::CurrentUser,
        data: PushData::Note,
        source_device_iden: None
    };

    api.send::<Push, PushMsg>(&push).unwrap();
}


