#![feature(path_ext)]

extern crate pocket;
extern crate inotify;
extern crate xdg_basedir as xdg;
extern crate script_utils as utils;
extern crate rustc_serialize;

use pocket::Pocket;
use inotify::{INotify, ffi};
use std::fs::{File, PathExt};
use std::io::{BufReader, BufRead};

#[derive(RustcDecodable)]
struct Creds {
    consumer_key: String,
    access_token: String,
}

fn main() {
    let config: Creds = utils::load_config("pocket/creds.toml").expect("config file load error");
    let mut pocket = Pocket::new(&*config.consumer_key, Some(&*config.access_token));

    let queue = xdg::get_config_dirs().into_iter().filter(|p| p.exists()).next().expect("no vimb config found");

    let mut inotify = INotify::init().unwrap();
    inotify.add_watch(&queue, ffi::IN_CLOSE_WRITE).unwrap();

    println!("watching {} for changes...", queue.display());
    loop {
        inotify.wait_for_events().unwrap();
        let reader = BufReader::new(File::open(&queue).unwrap());
        for line in reader.lines() {
            match pocket.push(&*line.unwrap().trim()) {
                Ok(item) => println!("added url {}", item.given_url),
                Err(err) => println!("error: {:?}", err)
            }
        }
    }
}
