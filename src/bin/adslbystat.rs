#![feature(phase)]
#![feature(slicing_syntax)]
#![feature(old_orphan_check)]

#[cfg(test)]
extern crate test;
extern crate encoding;
extern crate toml;
extern crate hyper;
extern crate url;
extern crate "rustc-serialize" as rustc_serialize;
extern crate core;
#[phase(plugin)]
extern crate regex_macros;
extern crate regex;
extern crate "script-utils" as utils;

use hyper::client::Client;
use hyper::header::common::authorization::{Authorization, Basic};
use rustc_serialize::base64::ToBase64;
use rustc_serialize::base64::STANDARD;
use url::Url;
use encoding::{Encoding, DecoderTrap};
use encoding::all::WINDOWS_1251;

#[cfg(test)]
use test::Bencher;

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

    let mut client = Client::new();
    client.set_ssl_verifier(utils::permissive_ssl_checker);

    let cont = WINDOWS_1251.decode(client.get("https://www.adsl.by/001.htm")
        .header(Authorization(Basic { username: config.username, password: Some(config.password) }))
        .send()
        .unwrap()
        .read_to_end()
        .unwrap()[], DecoderTrap::Replace)
        .unwrap();

    let acct = AcctInfo {
            enabled: state_re.is_match(cont[]),
            account: account_re.captures(cont[]).and_then(|c| c.at(1).and_then(|v| v.replace(" ", "").parse())).unwrap_or(0),
            days: days_re.captures(cont[]).and_then(|c| c.at(1).and_then(|v| v.parse())).unwrap_or(0),
            price: price_re.captures(cont[]).and_then(|c| c.at(1).and_then(|v| v.parse())).unwrap_or(0),
            credit: credit_re.captures(cont[]).and_then(|c| c.at(1).and_then(|v| v.parse()))
        };

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
