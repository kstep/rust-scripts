#![feature(slicing_syntax)]

extern crate pb;
extern crate "rustc-serialize" as rustc_serialize;
extern crate "script-utils" as utils;

use pb::api::PbAPI;
use pb::messages::{PushMsg, TargetIden};
use pb::objects::{Push, PushData};
use std::os::getenv;

#[deriving(RustcDecodable)]
struct Config {
    access_token: String
}

fn main() {
    let api = PbAPI::new(utils::load_config::<Config>().unwrap().access_token[]);
    let torrent_name = getenv("TR_TORRENT_NAME").unwrap();
    let torrent_dir = getenv("TR_TORRENT_DIR").unwrap();
    let push = PushMsg {
        title: Some("Torrent download complete".to_string()),
        body: Some(format!("{} downloaded to {}", torrent_name, torrent_dir)),
        target: TargetIden::CurrentUser,
        data: PushData::Note,
        source_device_iden: None
    };

    let result: Push = api.send(&push).unwrap();
    println!("notified with push {}", result.iden);
}


