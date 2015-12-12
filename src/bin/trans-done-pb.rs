#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]

extern crate pb;
extern crate serde;
extern crate script_utils as utils;

use pb::{PbAPI, PushMsg, TargetIden, Push, PushData};
use std::env;

#[derive(Deserialize)]
struct Config {
    access_token: String,
    device_iden: Option<String>
}

fn main() {
    let pbcfg = utils::load_config::<Config>("pushbullet/config.toml").unwrap();
    let mut api = PbAPI::new(&*pbcfg.access_token);
    let torrent_name = env::var("TR_TORRENT_NAME").unwrap();
    let torrent_dir = env::var("TR_TORRENT_DIR").unwrap();
    let push = PushMsg {
        title: Some("Torrent download complete".to_string()),
        body: Some(format!("{} downloaded to {}", torrent_name, torrent_dir)),
        target: TargetIden::CurrentUser,
        data: PushData::Note,
        source_device_iden: pbcfg.device_iden
    };

    let result: Push = api.send(&push).unwrap();
    println!("notified with push {}", result.iden);
}

