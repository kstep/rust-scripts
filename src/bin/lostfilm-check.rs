#![feature(slicing_syntax)]
#![feature(old_orphan_check)]
#![feature(plugin)]
#![allow(unstable)]

extern crate encoding;

extern crate hyper;
extern crate cookie;
extern crate url;
#[plugin]
extern crate regex_macros;
extern crate regex;
extern crate "rustc-serialize" as rustc_serialize;
extern crate "script-utils" as utils;
extern crate xml;
extern crate pb;

use encoding::{Encoding, DecoderTrap};
use encoding::all::WINDOWS_1251;

use hyper::client::{Client, RedirectPolicy};
use hyper::status::StatusCode;
use hyper::header::common::content_type::ContentType;
use hyper::header::common::user_agent::UserAgent;
use hyper::header::common::cookie::Cookies;
use hyper::header::common::set_cookie::SetCookie;
use hyper::header::{Header, HeaderFormat};

use cookie::{CookieJar, Cookie};

use url::{Url, form_urlencoded};
use xml::reader::EventReader;
use xml::reader::events::XmlEvent;
use xml::name::OwnedName;
use std::fmt::Show;
use pb::api::PbAPI;
use pb::messages::{PushMsg, TargetIden};
use pb::objects::{Push, PushData};

static USER_AGENT: &'static str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/33.0.1750.152 Safari/537.36";
static TORRENTS_DIR: &'static str = ".";
static TRANSMISSION_URL: &'static str = "http://localhost:9091/transmission/rpc";
static BASE_URL: &'static str = "http://www.lostfilm.tv/";
static LOGIN_URL: &'static str = "http://login1.bogi.ru/login.php";

#[derive(RustcDecodable)]
struct Config {
    include: Vec<String>,
    exclude: Vec<String>,
    username: String,
    password: String
}

#[derive(RustcDecodable)]
struct PbConfig {
    access_token: String
}

fn notify(api: &mut PbAPI, title: &str, url: &str) {
    println!("added torrent {}: {}",  title, url);

    let push = PushMsg {
        title: Some(format!("New LostFilm release: {}", title)),
        body: None,
        target: TargetIden::CurrentUser,
        data: PushData::Link(Url::parse(url).ok()),
        source_device_iden: None
    };

    if let Ok(result @ Push {..}) = api.send(&push) {
        println!("notified with push {}", result.iden);
    }
}

macro_rules! qs {
    ($($key:expr => $value:expr),*) => {
        vec![$(($key, $value)),*]
    }
}

#[allow(unused_must_use)]
fn login<'a>(login: &str, password: &str) -> CookieJar<'a> {
    let mut url = Url::parse(LOGIN_URL).unwrap();
    url.set_query_from_pairs(vec![("referer", BASE_URL)].into_iter());

    let data = form_urlencoded::serialize(qs![
        "login" => login,
        "password" => password,
        "module" => "1",
        "target" => BASE_URL,
        "repage" => "user",
        "act" => "login"
    ].into_iter());

    let input_re = regex!("<input .*?name=\"(\\w+)\" .*?value=\"([^\"]*)\"");
    let action_re = regex!("action=\"([^\"]+)\"");

    let mut cookie_jar = CookieJar::new(b"3b53fc89707a78fae45eeafff931f054");

    let mut client = Client::new();

    client.set_redirect_policy(RedirectPolicy::FollowAll);
    let mut response = client.post(url)
        .body(&*data)
        .header(ContentType("application/x-www-form-urlencoded".parse().unwrap()))
        .header(UserAgent(USER_AGENT.to_string()))
        .header(Referer(BASE_URL.to_string()))
        .send()
        .unwrap();

    response.headers.get::<SetCookie>().expect("no login cookies").apply_to_cookie_jar(&mut cookie_jar);

    let decoded_body = WINDOWS_1251.decode(&*response.read_to_end().unwrap(), DecoderTrap::Replace).unwrap();

    let action = action_re.captures(&*decoded_body).expect("no action URL found in login form").at(1).unwrap();
    let form = form_urlencoded::serialize(input_re.captures_iter(&*decoded_body).map(|&: c| (c.at(1).unwrap(), c.at(2).unwrap())));

    client.set_redirect_policy(RedirectPolicy::FollowNone);
    let response = client.post(action)
        .body(&*form)
        .header(Cookies::from_cookie_jar(&cookie_jar))
        .header(ContentType("application/x-www-form-urlencoded".parse().unwrap()))
        .header(UserAgent(USER_AGENT.to_string()))
        .header(Referer(LOGIN_URL.to_string()))
        .send()
        .unwrap();

    response.headers.get::<SetCookie>().expect("not session cookies").apply_to_cookie_jar(&mut cookie_jar);

    cookie_jar
}

enum RssState {
    Init,
    InChannel,
    InItem,
    InTitle,
    InLink
}

fn get_torrent_urls(cookie_jar: &CookieJar, include: &[String], exclude: &[String]) -> Vec<(String, String)> {
    let url = format!("{}{}", BASE_URL, "rssdd.xml");
    let body = Client::new()
        .get(&*url)
        .header(UserAgent(USER_AGENT.to_string()))
        .send()
        .unwrap()
        .read_to_end()
        .unwrap();

    let decoded_body = WINDOWS_1251.decode(&*body, DecoderTrap::Replace);
    let mut reader = EventReader::new_from_string(decoded_body.unwrap());

    let mut state = RssState::Init;
    let mut result = Vec::new();
    let mut needed = false;
    let mut title = "".to_string();

    for ev in reader.events() {
        match ev {
            XmlEvent::StartElement { name: OwnedName { ref local_name, .. }, .. } => match (&state, &**local_name) {
                (&RssState::Init, "channel") => state = RssState::InChannel,
                (&RssState::InChannel, "item") => state = RssState::InItem,
                (&RssState::InItem, "title") => state = RssState::InTitle,
                (&RssState::InItem, "link") => state = RssState::InLink,
                _ => ()
            },
            XmlEvent::EndElement { name: OwnedName { ref local_name, .. } } => match (&state, &**local_name) {
                (&RssState::InChannel, "channel") => state = RssState::Init,
                (&RssState::InItem, "item") => state = RssState::InChannel,
                (&RssState::InTitle, "title") => state = RssState::InItem,
                (&RssState::InLink, "link") => state = RssState::InItem,
                _ => ()
            },
            XmlEvent::Characters(ref value) => match state {
                RssState::InTitle => {
                    needed = include.iter().find(|v| value.contains(&***v)).is_some()
                         && !exclude.iter().find(|v| value.contains(&***v)).is_some();

                    if needed {
                        title = value.clone();
                    }
                },
                RssState::InLink if needed => {
                    result.push((title.clone(), extract_torrent_link(cookie_jar, value.replace("/download.php?", "/details.php?").rsplitn(1, '&').last().expect("torrent URL parse failed"))));
                },
                _ => ()
            },
            _ => ()
        }
    }

    result
}

fn extract_torrent_link(cookie_jar: &CookieJar, details_url: &str) -> String {
    let a_download_tag_re = regex!(r#"<a href="javascript:\{\};" onMouseOver="setCookie\('(\w+)','([a-f0-9]+)'\)" class="a_download" onClick="ShowAllReleases\('([0-9]+)','([0-9.]+)','([0-9]+)'\)"></a>"#);
    let torrent_link_re = regex!(r#"href="(http://tracktor\.in/td\.php\?s=[^"]+)""#);
    
    let mut client = Client::new();
    client.set_redirect_policy(RedirectPolicy::FollowAll);

    let body = client.get(details_url)
        .header(Cookies::from_cookie_jar(cookie_jar))
        .header(UserAgent(USER_AGENT.to_string()))
        .header(Referer(BASE_URL.to_string()))
        .send()
        .unwrap()
        .read_to_end()
        .unwrap();

    let decoded_body = WINDOWS_1251.decode(&*body, DecoderTrap::Replace).unwrap();

    let a_download_tag = a_download_tag_re.captures(&*decoded_body).unwrap();
    let (href, cookie_name, cookie_value) = (
        format!("{}nrdr.php?c={}&s={}&e={}", BASE_URL, a_download_tag.at(3).unwrap(), a_download_tag.at(4).unwrap(), a_download_tag.at(5).unwrap()),
        format!("{}_2", a_download_tag.at(1).unwrap()),
        a_download_tag.at(2).unwrap().to_string());

    cookie_jar.add(Cookie::new(cookie_name, cookie_value));

    let body = client.get(&*href)
        .header(Cookies::from_cookie_jar(cookie_jar))
        .header(UserAgent(USER_AGENT.to_string()))
        .header(Referer(details_url.to_string()))
        .send()
        .unwrap()
        .read_to_end()
        .unwrap();

    let decoded_body = WINDOWS_1251.decode(&*body, DecoderTrap::Replace).unwrap();
    torrent_link_re.captures(&*decoded_body).unwrap().at(1).unwrap().to_string()
}

struct TransmissionAPI {
    token: TransmissionSessionId,
    tag: u32
}

#[derive(Clone)]
struct TransmissionSessionId(pub String);

impl Header for TransmissionSessionId {
    #[allow(unused_variables)]
    fn header_name(marker: Option<Self>) -> &'static str {
        "X-Transmission-Session-Id"
    }

    fn parse_header(raw: &[Vec<u8>]) -> Option<TransmissionSessionId> {
        Some(TransmissionSessionId(String::from_utf8_lossy(&*raw[0]).into_owned()))
    }
}

impl HeaderFormat for TransmissionSessionId {
    fn fmt_header(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        let TransmissionSessionId(ref value) = *self;
        value.fmt(fmt)
    }
}

#[derive(Clone)]
struct Referer(pub String);

impl Header for Referer {
    #[allow(unused_variables)]
    fn header_name(marker: Option<Self>) -> &'static str {
        "Referer"
    }

    fn parse_header(raw: &[Vec<u8>]) -> Option<Referer> {
        Some(Referer(String::from_utf8_lossy(&*raw[0]).into_owned()))
    }
}

impl HeaderFormat for Referer {
    fn fmt_header(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        let Referer(ref value) = *self;
        fmt.write_str(&**value)
    }
}

impl TransmissionAPI {
    pub fn new() -> TransmissionAPI {
        TransmissionAPI {
            token: TransmissionSessionId("".to_string()),
            tag: 0
        }
    }

    pub fn add_torrent(&mut self, url: &str) -> bool {
        let mut client = Client::new();

        loop {
            self.tag = self.tag + 1;
            let mut resp = client.post(TRANSMISSION_URL)
                .body(&*format!(r#"{{"tag":"{}","method":"torrent-add","arguments":{{"filename":"{}"}}}}"#, self.tag, url))
                .header(self.token.clone())
                .header(ContentType("application/json".parse().unwrap()))
                .send()
                .unwrap();

            match resp.status {
                StatusCode::Ok => {
                    return resp.read_to_string().unwrap().contains("torrent-added");
                },
                StatusCode::Conflict => {
                    self.token = resp.headers.get::<TransmissionSessionId>().unwrap().clone();
                },
                code @ _ => {
                    panic!("unexpected error code {} for torrent {}", code, url);
                }
            }
        }
    }
}

fn main() {
    let config: Config = utils::load_config("lostfilm/config.toml").expect("config file missing");
    let cookie_jar = login(&*config.username, &*config.password);

    let mut pbapi = PbAPI::new(&*utils::load_config::<PbConfig>("pushbullet/creds.toml").unwrap().access_token);
    let mut trans = TransmissionAPI::new();

    let urls = get_torrent_urls(&cookie_jar, &*config.include, &*config.exclude);
    for &(ref title, ref url) in urls.iter() {
        if trans.add_torrent(&**url) {
            notify(&mut pbapi, &**title, &**url);
        }
    }
}
