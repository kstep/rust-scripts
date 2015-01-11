#![feature(slicing_syntax)]
#![allow(unstable)]

extern crate xdg;
extern crate toml;
extern crate "rustc-serialize" as rustc_serialize;
extern crate openssl;

use rustc_serialize::Decodable;
use std::io::File;
use xdg::XdgDirs;

pub fn load_config<C: Decodable>(filename: &str) -> Option<C> {
    XdgDirs::new().want_read_config(filename)
        .and_then(|ref p| File::open(p).ok())
        .and_then(|mut f| f.read_to_string().ok())
        .and_then(|s| toml::decode_str(&*s))
}

#[allow(unused_variables)]
pub fn permissive_ssl_checker(p: bool, ctx: &openssl::x509::X509StoreContext) -> bool {
    true
}
