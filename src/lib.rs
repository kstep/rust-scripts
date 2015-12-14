extern crate xdg_basedir as xdg;
extern crate toml;
extern crate serde;
extern crate openssl;

use serde::Deserialize;
use std::fs::File;
use std::io::Read;

pub fn load_config<C: Deserialize>(filename: &str) -> Option<C> {
    xdg::get_config_dirs()
        .into_iter()
        .filter_map(|p| File::open(p.join(filename)).ok())
        .next()
        .and_then(|mut f| {
            let mut buf = String::new();
            match f.read_to_string(&mut buf) {
                Ok(_) => toml::decode_str(&buf),
                _ => None,
            }
        })
}

pub fn permissive_ssl_checker(ctx: &mut openssl::ssl::SslContext) {
    ctx.set_verify(openssl::ssl::SSL_VERIFY_NONE, None);
}
