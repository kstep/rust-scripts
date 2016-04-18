extern crate pb;
extern crate serde;
extern crate script_utils as utils;

use pb::{PbAPI, PushMsg, TargetIden, Push, PushData};
use std::env;

include!(concat!(env!("OUT_DIR"), "/trans-done-pb.rs"));

fn main() {
    let pbcfg = utils::load_config::<Config>("pushbullet/config.toml").unwrap();
    let mut api = PbAPI::new(&*pbcfg.access_token);
    let torrent_name = env::var("TR_TORRENT_NAME").unwrap();
    let torrent_dir = env::var("TR_TORRENT_DIR").unwrap();
    let push = PushMsg {
        title: Some("Torrent download complete".into()),
        body: Some(format!("{} downloaded to {}", torrent_name, torrent_dir).into()),
        target: TargetIden::CurrentUser,
        data: PushData::Note,
        source_device_iden: pbcfg.device_iden,
    };

    let result: Push = api.send(&push).unwrap();
    println!("notified with push {}", result.iden);
}
