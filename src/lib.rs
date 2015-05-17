extern crate xdg;
extern crate toml;
extern crate rustc_serialize;
extern crate openssl;

use rustc_serialize::Decodable;
use std::fs::File;
use std::io::Read;

pub fn load_config<C: Decodable>(filename: &str) -> Option<C> {
    xdg::get_config_dirs().into_iter()
        .filter_map(|p| File::open(p.join(filename)).ok()).next()
        .and_then(|mut f| {
            let mut buf = String::new();
            match f.read_to_string(&mut buf) {
                Ok(_) => toml::decode_str(&buf),
                _ => None
            }
        })
}

pub fn permissive_ssl_checker(ctx: &mut openssl::ssl::SslContext) {
    ctx.set_verify(openssl::ssl::SslVerifyMode::all(), None);
}
