#![feature(phase)]

#[cfg(test)]
extern crate test;
extern crate encoding;
extern crate toml;
extern crate http;
extern crate url;
extern crate serialize;
extern crate core;
extern crate regex;
extern crate xdg;
#[phase(plugin)]
extern crate regex_macros;

use http::client::RequestWriter;
use http::method::Get;
//use http::status;
use serialize::base64::ToBase64;
use serialize::base64::STANDARD;
use serialize::Decodable;
use url::Url;
use encoding::{Encoding, DecodeReplace};
use encoding::all::WINDOWS_1251;
use std::str::replace;
use std::io::File;
use xdg::XdgDirs;

#[cfg(test)]
use test::Bencher;

fn to_str_err<E: ToString>(e: E) -> String {
    e.to_string()
}

fn req_to_str_err<W, E: ToString>(e: (W, E)) -> String {
    let (_, s) = e;
    s.to_string()
}

#[deriving(Show)]
struct AcctInfo {
    enabled: bool,
    account: int,
    days: int,
    price: int,
    credit: Option<int>
}

#[deriving(Decodable, Show)]
struct Creds {
    username: String,
    password: String
}

fn main() {
    let state_re = regex!(r">Аккаунт</td>\s*<td class='right'><b>Включен<");
    let account_re = regex!(r"Осталось трафика на сумму</td>\s*<td class='right'><b>([0-9 ]+)");
    let days_re = regex!(r"осталось <b>(\d+) д");
    let price_re = regex!(r"тариф</td>\s*<td class='right'><b>(\d+) ");
    let credit_re = regex!(r"кредит</td>\s*<td class='right'><b>(\d+)%");

    let config_file = XdgDirs::new().want_read_config("adslby/creds.toml").unwrap_or_else(|| fail!("Config file not found!"));

    let config: Creds = File::open(&config_file).map_err(to_str_err)
        .and_then(|mut f| f.read_to_string().map_err(to_str_err))
        .and_then(|s| match toml::decode_str(s.as_slice()) { Some(v) => Ok(v), None => Err("Invalid TOML file".to_string()) })
        .unwrap_or_else(|err| fail!("ERROR: {}", err));

    let acct = Url::parse("https://www.adsl.by/001.htm").map_err(to_str_err)
        .and_then(|url| RequestWriter::new(Get, url).map_err(to_str_err))
        .and_then(|mut req: RequestWriter| {
            req.headers.authorization = Some(format!("Basic {}", format!("{}:{}", config.username, config.password).as_bytes().to_base64(STANDARD)));
            req.read_response().map_err(req_to_str_err)
        })
        .and_then(|mut resp| resp.read_to_end().map_err(to_str_err))
        .and_then(|cont| WINDOWS_1251.decode(cont.as_slice(), DecodeReplace).map_err(to_str_err))
        .map(|cont| AcctInfo {
            enabled: state_re.is_match(cont.as_slice()),
            account: account_re.captures(cont.as_slice()).and_then(|c| from_str(replace(c.at(1), " ", "").as_slice())).unwrap_or(0),
            days: days_re.captures(cont.as_slice()).and_then(|c| from_str(c.at(1))).unwrap_or(0),
            price: price_re.captures(cont.as_slice()).and_then(|c| from_str(c.at(1))).unwrap_or(0),
            credit: credit_re.captures(cont.as_slice()).and_then(|c| from_str(c.at(1)))
        })
        .unwrap_or_else(|err| fail!("ERROR: {}", err));

    println!("{}", acct);
}

#[test]
fn test_path_if_exists() {
    match path_if_exists("/tmp") {
        Some(p) => assert!(Path::new("/tmp").exists()),
        None => assert!(!Path::new("/tmp").exists()),
    }
    match path_if_exists("/not-exists") {
        Some(p) => assert!(Path::new("/not-exists").exists()),
        None => assert!(!Path::new("/not-exists").exists()),
    }
}

#[bench]
#[ignore]
fn bench_main(b: &mut Bencher) {
    b.iter(|| {
        main()
    });
}

#[bench]
fn bench_path_if_exists(b: &mut Bencher) {
    b.iter(|| {
        path_if_exists("/tmp")
    })
}

#[bench]
fn bench_path_if_not_exists(b: &mut Bencher) {
    b.iter(|| {
        path_if_exists("/not-exists")
    })
}
