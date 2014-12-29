#![feature(phase)]
#![feature(slicing_syntax)]

#[cfg(test)]
extern crate test;
extern crate encoding;
extern crate toml;
extern crate http;
extern crate url;
extern crate "rustc-serialize" as rustc_serialize;
extern crate core;
extern crate regex;
extern crate "script-utils" as utils;
#[phase(plugin)]
extern crate regex_macros;

use http::client::RequestWriter;
use http::method::Get;
//use http::status;
use rustc_serialize::base64::ToBase64;
use rustc_serialize::base64::STANDARD;
use url::Url;
use encoding::{Encoding, DecoderTrap};
use encoding::all::WINDOWS_1251;

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

#[deriving(RustcDecodable, Show)]
struct Creds {
    username: String,
    password: String
}

fn main() {
    let state_re = regex!(r">Аккаунт</td>\s*<td class='right'><b>Включен<");
    let account_re = regex!(r"Осталось трафика на сумму</td>\s*<td class='right'><b>(-?[0-9 ]+)");
    let days_re = regex!(r"осталось <b>(-?\d+) д");
    let price_re = regex!(r"тариф</td>\s*<td class='right'><b>(\d+) ");
    let credit_re = regex!(r"кредит</td>\s*<td class='right'><b>(\d+)%");

    let config: Creds = utils::load_config("adslby/creds.toml").expect("config file load error");

    let acct = Url::parse("https://www.adsl.by/001.htm").map_err(to_str_err)
        .and_then(|url| RequestWriter::new(Get, url).map_err(to_str_err))
        .and_then(|mut req: RequestWriter| {
            req.headers.authorization = Some(format!("Basic {}", format!("{}:{}", config.username, config.password).as_bytes().to_base64(STANDARD)));
            req.read_response().map_err(req_to_str_err)
        })
        .and_then(|mut resp| resp.read_to_end().map_err(to_str_err))
        .and_then(|cont| WINDOWS_1251.decode(cont[], DecoderTrap::Replace).map_err(to_str_err))
        .map(|cont| AcctInfo {
            enabled: state_re.is_match(cont[]),
            account: account_re.captures(cont[]).and_then(|c| c.at(1).and_then(|v| v.replace(" ", "").parse())).unwrap_or(0),
            days: days_re.captures(cont[]).and_then(|c| c.at(1).and_then(|v| v.parse())).unwrap_or(0),
            price: price_re.captures(cont[]).and_then(|c| c.at(1).and_then(|v| v.parse())).unwrap_or(0),
            credit: credit_re.captures(cont[]).and_then(|c| c.at(1).and_then(|v| v.parse()))
        })
        .unwrap_or_else(|err| panic!("ERROR: {}", err));

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
