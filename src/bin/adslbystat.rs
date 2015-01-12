#![feature(slicing_syntax)]
#![feature(plugin)]
#![allow(unstable)]

#[cfg(test)]
extern crate test;
extern crate encoding;
extern crate toml;
extern crate hyper;
extern crate url;
extern crate "rustc-serialize" as rustc_serialize;
extern crate core;
#[plugin]
extern crate regex_macros;
extern crate regex;
extern crate "script-utils" as utils;

use hyper::client::Client;
use hyper::header::common::authorization::{Authorization, Basic};
use encoding::{Encoding, DecoderTrap};
use encoding::all::WINDOWS_1251;
use std::fmt;
use std::os::set_exit_status;

#[cfg(test)]
use test::Bencher;

#[derive(Show)]
struct AcctInfo {
    enabled: bool,
    account: i32,
    days: i32,
    price: i32,
    credit: Option<i32>
}

impl fmt::String for AcctInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        try!(writeln!(f, "Enabled: {}", self.enabled));
        try!(writeln!(f, "Account: {} rub", self.account));
        try!(writeln!(f, "Days left: {}", self.days));
        try!(writeln!(f, "Price per Mib: {} rub", self.price));
        if let Some(ref c) = self.credit {
            try!(writeln!(f, "Allowed credit: {}%", c));
        }
        Ok(())
    }
}

#[derive(RustcDecodable, Show)]
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

    let cont = WINDOWS_1251.decode(&*client.get("https://www.adsl.by/001.htm")
        .header(Authorization(Basic { username: config.username, password: Some(config.password) }))
        .send()
        .unwrap()
        .read_to_end()
        .unwrap(), DecoderTrap::Replace)
        .unwrap();

    let acct = AcctInfo {
            enabled: state_re.is_match(&*cont),
            account: account_re.captures(&*cont).and_then(|c| c.at(1).and_then(|v| v.replace(" ", "").parse())).unwrap_or(0),
            days: days_re.captures(&*cont).and_then(|c| c.at(1).and_then(|v| v.parse())).unwrap_or(0),
            price: price_re.captures(&*cont).and_then(|c| c.at(1).and_then(|v| v.parse())).unwrap_or(0),
            credit: credit_re.captures(&*cont).and_then(|c| c.at(1).and_then(|v| v.parse()))
        };

    println!("{}", acct);
    set_exit_status(if acct.enabled { 0 } else { 1 });
}

#[bench]
#[ignore]
fn bench_main(b: &mut Bencher) {
    b.iter(|| {
        main()
    });
}
