#![feature(macro_rules)]
#![feature(phase)]
#![feature(slicing_syntax)]

extern crate encoding;

extern crate curl;
extern crate url;
extern crate regex;
extern crate "rustc-serialize" as rustc_serialize;
extern crate "script-utils" as utils;
extern crate xml;
extern crate pb;

#[phase(plugin)]
extern crate regex_macros;

use encoding::{Encoding, DecoderTrap};
use encoding::all::WINDOWS_1251;

use url::{Url, form_urlencoded};
use xml::reader::EventReader;
use xml::reader::events::XmlEvent;
use xml::name::OwnedName;
use std::collections::BTreeMap;
use std::path::Path;
use std::str::from_utf8;
use pb::api::PbAPI;
use pb::messages::{PushMsg, TargetIden};
use pb::objects::{Push, PushData};

static USER_AGENT: &'static str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/33.0.1750.152 Safari/537.36";
static TORRENTS_DIR: &'static str = ".";
static TRANSMISSION_URL: &'static str = "http://localhost:9091/transmission/rpc";
static BASE_URL: &'static str = "http://www.lostfilm.tv/";
static LOGIN_URL: &'static str = "http://login1.bogi.ru/login.php";
static COOKIE_JAR: &'static str = "/tmp/lostfilm.cookies";

#[deriving(RustcDecodable)]
struct Config {
    include: Vec<String>,
    exclude: Vec<String>,
    username: String,
    password: String
}

#[deriving(RustcDecodable)]
struct PbConfig {
    access_token: String
}

fn notify(api: &PbAPI, url: &str, title: &str) {
    println!("added torrent {}: {}",  title, url);

    let push = PushMsg {
        title: Some(format!("New LostFilm release: {}", title)),
        body: None,
        target: TargetIden::CurrentUser,
        data: PushData::Url(Url::parse(url).ok()),
        source_device_iden: None
    };

    if let Ok(result @ Push {..}) = api.send(&push) {
        println!("notified with push {}", result.iden);
    }
}

type Cookies = BTreeMap<String, String>;

macro_rules! qs {
    ($($key:expr -> $value:expr),*) => {
        vec![$(($key, $value)),*]
    }
}

macro_rules! mime {
    ($t:ident / $s:expr) => {
        MediaType::new(stringify!($t).to_string(), $s.to_string(), vec![])
    }
}

fn login(login: &str, password: &str) {
    let mut url = Url::parse(LOGIN_URL).unwrap();
    url.set_query_from_pairs(vec![("referer", BASE_URL)].into_iter());

    let data = form_urlencoded::serialize(qs![
        "login" -> login,
        "password" -> password,
        "module" -> "1",
        "target" -> BASE_URL,
        "repage" -> "user",
        "act" -> "login"
    ].into_iter());

    let input_re = regex!("<input .*?name=\"(\\w+)\" .*?value=\"([^\"]*)\"");
    let action_re = regex!("action=\"([^\"]+)\"");

    let cookie_jar = Path::new("/tmp/lostfilm.cookies");

    let body = curl::http::handle()
        .cookies(&cookie_jar)
        .post(url.to_string()[], data[])
        .follow_redirects(true)
        .content_type("application/x-www-form-urlencoded")
        .verify_peer(false)
        .header("User-Agent", USER_AGENT)
        .header("Referer", BASE_URL)
        .exec()
        .unwrap()
        .move_body();

    let decoded_body = WINDOWS_1251.decode(body[], DecoderTrap::Replace).unwrap();

    let action = action_re.captures(decoded_body[]).unwrap().at(1).unwrap();
    let form = form_urlencoded::serialize(input_re.captures_iter(decoded_body[]).map(|c| (c.at(1).unwrap(), c.at(2).unwrap())));

    curl::http::handle()
        .cookies(&cookie_jar)
        .post(action, form[])
        .follow_redirects(true)
        .content_type("application/x-www-form-urlencoded")
        .verify_peer(false)
        .header("User-Agent", USER_AGENT)
        .header("Referer", LOGIN_URL)
        .exec()
        .unwrap();
}

enum RssState {
    Init,
    InChannel,
    InItem,
    InTitle,
    InLink
}

fn get_torrent_urls(include: &[String], exclude: &[String]) -> Vec<(String, String)> {
    let url = format!("{}{}", BASE_URL, "rssdd.xml");
    let body = curl::http::handle()
        .get(url[])
        .header("User-Agent", USER_AGENT)
        .exec()
        .unwrap()
        .move_body();

    let decoded_body = WINDOWS_1251.decode(body[], DecoderTrap::Replace);
    let mut reader = EventReader::new_from_string(decoded_body.unwrap());

    let mut state = RssState::Init;
    let mut result = Vec::new();
    let mut needed = false;
    let mut title = "".to_string();

    for ev in reader.events() {
        match ev {
            XmlEvent::StartElement { name: OwnedName { ref local_name, .. }, .. } => match (&state, local_name[]) {
                (&RssState::Init, "channel") => state = RssState::InChannel,
                (&RssState::InChannel, "item") => state = RssState::InItem,
                (&RssState::InItem, "title") => state = RssState::InTitle,
                (&RssState::InItem, "link") => state = RssState::InLink,
                _ => ()
            },
            XmlEvent::EndElement { name: OwnedName { ref local_name, .. } } => match (&state, local_name[]) {
                (&RssState::InChannel, "channel") => state = RssState::Init,
                (&RssState::InItem, "item") => state = RssState::InChannel,
                (&RssState::InTitle, "title") => state = RssState::InItem,
                (&RssState::InLink, "link") => state = RssState::InItem,
                _ => ()
            },
            XmlEvent::Characters(ref value) => match state {
                RssState::InTitle => {
                    needed = include.iter().find(|v| value.contains(v[])).is_some()
                         && !exclude.iter().find(|v| value.contains(v[])).is_some();
                    if needed {
                        title = value.clone();
                    }
                },
                RssState::InLink if needed => {
                    result.push((title.to_string(), extract_torrent_link(value.replace("/download.php?", "/details.php?").rsplitn(1, '&').last().unwrap())));
                },
                _ => ()
            },
            _ => ()
        }
    }

    result
}

fn extract_torrent_link(details_url: &str) -> String {
    let a_download_tag_re = regex!(r#"<a href="javascript:\{\};" onMouseOver="setCookie\('(\w+)','([a-f0-9]+)'\)" title="Искать" alt="Искать" class="a_download" onClick="ShowAllReleases\('([0-9]+)','([0-9.]+)','([0-9]+)'\)"></a>"#);
    let torrent_link_re = regex!(r#"href="(http://tracktor\.in/td\.php\?s=[^"]+)""#);

    let cookie_jar = Path::new(COOKIE_JAR);
    let body = curl::http::handle()
        .cookies(&cookie_jar)
        .get(details_url)
        .header("User-Agent", USER_AGENT)
        .header("Referer", BASE_URL)
        .exec()
        .unwrap()
        .move_body();
    let decoded_body = WINDOWS_1251.decode(body[], DecoderTrap::Replace).unwrap();

    let a_download_tag = a_download_tag_re.captures(decoded_body[]).unwrap();
    let (href, cookie) = (
        format!("{}nrdr.php?c={}&s={}&e={}", BASE_URL, a_download_tag.at(3).unwrap(), a_download_tag.at(4).unwrap(), a_download_tag.at(5).unwrap()),
        format!("Set-Cookie: {}_2={}", a_download_tag.at(1).unwrap(), a_download_tag.at(2).unwrap()));

    let body = curl::http::handle()
        .cookies(&cookie_jar)
        .cookie(cookie[])
        .get(href[])
        .follow_redirects(true)
        .header("User-Agent", USER_AGENT)
        .header("Referer", details_url)
        .exec()
        .unwrap()
        .move_body();

    let decoded_body = WINDOWS_1251.decode(body[], DecoderTrap::Replace).unwrap();
    torrent_link_re.captures(decoded_body[]).unwrap().at(1).unwrap().to_string()
}

fn add_to_transmission(url: &str) -> bool {
    let mut token = "".to_string();

    loop {
        let resp = curl::http::handle()
            .post(TRANSMISSION_URL, format!(r#"{{"tag":"{}","method":"torrent-add","arguments":{{"filename":"{}"}}}}"#, token, url)[])
            .exec()
            .unwrap();

        match resp.get_code() {
            200 => {
                return from_utf8(resp.get_body()).unwrap().contains("torrent-added");
            },
            409 => {
                token = resp.get_header("X-Transmission-Session-Id")[0].clone();
            },
            code @ _ => {
                panic!("unexpected error code {} for torrent {}", code, url);
            }
        }
    }


}

fn main() {
    let config: Config = utils::load_config("lostfilm/config.toml").unwrap();
    login(config.username[], config.password[]);

    let pbapi = PbAPI::new(utils::load_config::<PbConfig>("pushbullet/creds.toml").unwrap().access_token[]);

    let urls = get_torrent_urls(config.include[], config.exclude[]);
    for &(ref title, ref url) in urls.iter() {
        if add_to_transmission(url[]) {
            notify(&pbapi, title[], url[]);
        }
    }
}
