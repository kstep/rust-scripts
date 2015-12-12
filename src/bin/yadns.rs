#![feature(custom_derive, plugin)]
#![feature(custom_attribute)]
#![plugin(serde_macros)]

// TODO
#![allow(dead_code, unused_variables)]

extern crate xml;
extern crate pb;
extern crate hyper;
extern crate script_utils as utils;
extern crate url;
extern crate time;
extern crate serde;
extern crate serde_json;

use std::io::{Read, Error as IoError};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::borrow::{Cow, Borrow};
use std::convert::AsRef;
use std::env;
use std::fmt;
use std::error::Error as StdError;
use std::ops::{Deref, DerefMut};
use std::result::Result as StdResult;
use hyper::{Client, Error as HttpError};
use hyper::method::Method;
use hyper::header::{Header, HeaderFormat, ContentType};
use url::form_urlencoded;

#[derive(Debug, Clone)]
struct Config {
    domain: String,
    token: String
}

#[derive(Debug, Clone)]
struct PbConfig {
    domain: String,
    access_token: String
}

static BASE_URL: &'static str = "https://pddimp.yandex.ru/api2/admin/dns";

macro_rules! as_str {
    ($key:ident) => { stringify!($key) };
    ($key:expr) => { $key };
}
macro_rules! qs {
    ($($key:tt => $value:expr),* $(,)*) => {
        &[$((as_str!($key), $value)),*]
    }
}

#[derive(Debug, Clone)]
struct PddToken(String);

impl Header for PddToken {
    fn header_name() -> &'static str {
        "PddToken"
    }

    fn parse_header(raw: &[Vec<u8>]) -> StdResult<PddToken, HttpError> {
        Ok(PddToken(String::from_utf8_lossy(&*raw[0]).into_owned()))
    }
}

impl HeaderFormat for PddToken {
    fn fmt_header(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        let PddToken(ref value) = *self;
        fmt.write_str(&**value)
    }
}

#[derive(Debug)]
enum Error {
    /// HTTP error
    Http(HttpError),
    /// JSON decode error
    Json(serde_json::error::Error),
    /// Yandex API error
    Api(ErrorReplyDTO),
    /// IO error
    Io(IoError)
}

impl StdError for Error {
    fn description(&self) -> &str {
        use Error::*;
        match *self {
            Http(ref err) => err.description(),
            Json(ref err) => err.description(),
            Api(ref err) => err.description(),
            Io(ref err) => err.description(),
        }
    }

    fn cause(&self) -> Option<&StdError> {
        use Error::*;
        Some(match *self {
            Http(ref err) => err,
            Json(ref err) => err,
            Api(ref err) => err,
            Io(ref err) => err,
        })
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;
        match *self {
            Http(ref err) => err.fmt(f),
            Json(ref err) => err.fmt(f),
            Api(ref err) => err.fmt(f),
            Io(ref err) => err.fmt(f),
        }
    }
}

impl From<HttpError> for Error {
    fn from(value: HttpError) -> Error {
        match value {
            HttpError::Io(err) => Error::Io(err),
            _ => Error::Http(value)
        }
    }
}
impl From<::serde_json::error::Error> for Error {
    fn from(value: ::serde_json::error::Error) -> Error {
        Error::Json(value)
    }
}
impl From<ErrorReplyDTO> for Error {
    fn from(value: ErrorReplyDTO) -> Error {
        Error::Api(value)
    }
}
impl From<IoError> for Error {
    fn from(value: IoError) -> Error {
        Error::Io(value)
    }
}

struct YandexDNS {
    token: PddToken,
    client: Client
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum DnsType {
    Srv,
    Txt,
    Ns,
    Mx,
    Soa,
    A,
    Aaaa,
    Cname,
}

impl AsRef<str> for DnsType {
    fn as_ref(&self) -> &str {
        use DnsType::*;
        match *self {
            Srv => "SRV",
            Txt => "TXT",
            Ns => "NS",
            Mx => "MX",
            Soa => "SOA",
            A => "A",
            Aaaa => "AAAA",
            Cname => "CNAME",
        }
    }
}

impl serde::Deserialize for DnsType {
    fn deserialize<D: serde::Deserializer>(d: &mut D) -> StdResult<DnsType, D::Error> {
        struct DnsTypeVisitor;

        impl serde::de::Visitor for DnsTypeVisitor {
            type Value = DnsType;
            fn visit_str<E: serde::de::Error>(&mut self, v: &str) -> StdResult<DnsType, E> {
                use self::DnsType::*;
                match v {
                    "SRV" => Ok(Srv),
                    "TXT" => Ok(Txt),
                    "NS" => Ok(Ns),
                    "MX" => Ok(Mx),
                    "SOA" => Ok(Soa),
                    "A" => Ok(A),
                    "AAAA" => Ok(Aaaa),
                    "CNAME" => Ok(Cname),
                    _ => Err(serde::de::Error::unknown_field("unknown record type"))
                }
            }
        }

        d.visit(DnsTypeVisitor)
    }
}

struct SkipErr<T>(Option<T>);

impl<T: fmt::Debug> fmt::Debug for SkipErr<T> where Option<T>: fmt::Debug {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: fmt::Display> fmt::Display for SkipErr<T> where Option<T>: fmt::Display {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: serde::Deserialize> serde::Deserialize for SkipErr<T> {
    fn deserialize<D: serde::Deserializer>(de: &mut D) -> StdResult<SkipErr<T>, D::Error> {
        serde::Deserialize::deserialize(de).map(Some).or_else(|_| Ok(None)).map(SkipErr)
    }
}

trait Tap: Sized {
    fn tap<F, R>(self, f: F) -> Self where F: Fn(&Self) -> R {
        f(&self);
        self
    }
}

impl<T> Tap for T {}

impl<T> Into<Option<T>> for SkipErr<T> {
    fn into(self) -> Option<T> {
        self.0
    }
}

impl<T> From<Option<T>> for SkipErr<T> {
    fn from(value: Option<T>) -> SkipErr<T> {
        SkipErr(value)
    }
}

impl<T> Deref for SkipErr<T> {
    type Target = Option<T>;
    fn deref(&self) -> &Option<T> {
        &self.0
    }
}
impl<T> DerefMut for SkipErr<T> {
    fn deref_mut(&mut self) -> &mut Option<T> {
        &mut self.0
    }
}

type Result<T> = StdResult<T, Error>;

#[derive(Debug)]
enum Content {
    Ipv4(Ipv4Addr),
    Ipv6(Ipv6Addr),
    Info(String),
}

impl serde::Deserialize for Content {
    fn deserialize<D: serde::Deserializer>(d: &mut D) -> StdResult<Content, D::Error> {
        use Content::*;
        let info: String = try!(serde::Deserialize::deserialize(d));
        Ok(info.parse::<Ipv4Addr>().map(Ipv4)
           .or_else(|_| info.parse::<Ipv6Addr>().map(Ipv6))
           .unwrap_or_else(|_| Info(info)))
    }
}

#[derive(Debug, Deserialize)]
struct RecordDTO {
    record_id: u64,
    #[serde(rename="type")]
    kind: DnsType,
    domain: String,
    subdomain: String,
    fqdn: String,
    content: Content,
    ttl: u32,

    priority: SkipErr<u32>,

    // SOA
    refresh: Option<u32>,
    admin_mail: Option<String>,
    expire: Option<u32>,
    minttl: Option<u32>,
    retry: Option<u32>,

    // SRV
    weight: Option<u32>,
    port: Option<u16>,

    // edit
    operation: Option<String>,
}

impl RecordDTO {
    fn as_add_req(&self) -> AddRequestDTO {
        AddRequestDTO {
            domain: (&*self.domain).into(),
            kind: self.kind,

            admin_mail: self.admin_mail.as_ref().map(|v| (&**v).into()).unwrap_or("".into()),
            content: match self.content {
                Content::Ipv4(ref ip) => ip.to_string().into(),
                Content::Ipv6(ref ip) => ip.to_string().into(),
                Content::Info(ref info) => (&**info).into(),
            },
            priority: self.priority.unwrap_or(10),
            weight: self.weight.unwrap_or(0),
            port: self.port.unwrap_or(0),
            target: "".into(),

            subdomain: (&*self.subdomain).into(),
            ttl: self.ttl,
        }
    }
    fn as_edit_req(&self) -> EditRequestDTO {
        EditRequestDTO {
            domain: (&*self.domain).into(),
            record_id: self.record_id,

            subdomain: Some((&*self.subdomain).into()),
            ttl: Some(self.ttl),
            refresh: self.refresh,
            retry: self.retry,
            expire: self.expire,
            neg_cache: None,
            admin_mail: self.admin_mail.as_ref().map(|v| (&**v).into()),
            content: Some(match self.content {
                Content::Ipv4(ref ip) => ip.to_string().into(),
                Content::Ipv6(ref ip) => ip.to_string().into(),
                Content::Info(ref info) => (&**info).into(),
            }),
            priority: self.priority.clone(),
            port: self.port,
            weight: self.weight,
            target: None,
        }
    }
    fn as_delete_req(&self) -> DeleteRequestDTO {
        DeleteRequestDTO {
            domain: (&*self.domain).into(),
            record_id: self.record_id,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ListReplyDTO {
    records: Vec<RecordDTO>,
    domain: String,
    success: ResultCode,
}

#[derive(Debug, Deserialize)]
struct EditReplyDTO {
    domain: String,
    record_id: u64,
    record: RecordDTO,
    success: ResultCode,
}

#[derive(Debug, Deserialize)]
struct AddReplyDTO {
    domain: String,
    record: RecordDTO,
    success: ResultCode,
}

#[derive(Debug, Deserialize)]
struct DeleteReplyDTO {
    domain: String,
    record_id: u64,
    success: ResultCode,
}

#[derive(Debug, Deserialize)]
struct ErrorReplyDTO {
    domain: String,
    record_id: Option<u64>,
    success: ResultCode,
    error: ErrorCode,
}

impl StdError for ErrorReplyDTO {
    fn description(&self) -> &str {
        self.error.description()
    }
}

impl fmt::Display for ErrorReplyDTO {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.description().fmt(f)
    }
}

#[derive(Debug)]
enum ResultCode {
    Ok,
    Err,
}

impl serde::Deserialize for ResultCode {
    fn deserialize<D: serde::Deserializer>(d: &mut D) -> StdResult<ResultCode, D::Error> {
        struct ResultCodeVisitor;

        impl serde::de::Visitor for ResultCodeVisitor {
            type Value = ResultCode;
            fn visit_str<E: serde::de::Error>(&mut self, v: &str) -> StdResult<ResultCode, E> {
                match v {
                    "ok" => Ok(ResultCode::Ok),
                    "error" => Ok(ResultCode::Err),
                    _ => Err(serde::de::Error::unknown_field("invalid result code"))
                }
            }
        }

        d.visit_str(ResultCodeVisitor)
    }
}

#[derive(Debug, Clone)]
enum ErrorCode {
    Unknown,
    NoToken,
    NoDomain,
    NoContent,
    NoType,
    NoIp,
    BadDomain,
    Prohibited,
    BadToken,
    BadLogin,
    BadPasswd,
    NoAuth,
    NotAllowed,
    Blocked,
    Occupied,
    DomainLimitReached,
    NoReply,
}

impl StdError for ErrorCode {
    fn description(&self) -> &str {
        use ErrorCode::*;
        match *self {
            Unknown => "unknown error",
            NoToken => "access token missing",
            NoDomain => "domain name missing",
            NoContent => "content missing",
            NoType => "type missing",
            NoIp => "IP address missing",
            BadDomain => "invalid domain name",
            Prohibited => "domain name forbidden",
            BadToken => "invalid token",
            BadLogin => "invalid login",
            BadPasswd => "invalid password",
            NoAuth => "authorization missing",
            NotAllowed => "access denied",
            Blocked => "domain name blocked",
            Occupied => "domain name occupied",
            DomainLimitReached => "max number of domains exceeded",
            NoReply => "server access error",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.description().fmt(f)
    }
}

impl serde::Deserialize for ErrorCode {
    fn deserialize<D: serde::Deserializer>(d: &mut D) -> StdResult<ErrorCode, D::Error> {
        struct ErrorCodeVisitor;

        impl serde::de::Visitor for ErrorCodeVisitor {
            type Value = ErrorCode;
            fn visit_str<E: serde::de::Error>(&mut self, v: &str) -> StdResult<ErrorCode, E> {
                use self::ErrorCode::*;
                match v {
                    "unknown" => Ok(Unknown),
                    "no_token" => Ok(NoToken),
                    "no_domain" => Ok(NoDomain),
                    "no_content" => Ok(NoContent),
                    "no_type" => Ok(NoType),
                    "no_ip" => Ok(NoIp),
                    "bad_domain" => Ok(BadDomain),
                    "prohibited" => Ok(Prohibited),
                    "bad_token" => Ok(BadToken),
                    "bad_login" => Ok(BadLogin),
                    "bad_password" => Ok(BadPasswd),
                    "no_auth" => Ok(NoAuth),
                    "not_allowed" => Ok(NotAllowed),
                    "blocked" => Ok(Blocked),
                    "occupied" => Ok(Occupied),
                    "domain_limit_reached" => Ok(DomainLimitReached),
                    "no_reply" => Ok(NoReply),
                    _ => Err(serde::de::Error::unknown_field("invalid error code"))
                }
            }
        }

        d.visit_str(ErrorCodeVisitor)
    }
}

#[derive(Debug)]
struct ListRequestDTO<'a> {
    domain: Cow<'a, str>,
}

#[derive(Debug)]
struct AddRequestDTO<'a> {
    domain: Cow<'a, str>,
    kind: DnsType,

    admin_mail: Cow<'a, str>, // required for SOA
    content: Cow<'a, str>, // Ipv4 for A, Ipv6 for AAAA, string for CNAME, MX, NS, TXT
    priority: u32, // required for SRV and MX, default: 10
    weight: u32, // required for SRV
    port: u16, // required for SRV
    target: Cow<'a, str>, // required for SRV

    subdomain: Cow<'a, str>,
    ttl: u32, // default: 21600
}

impl<'a> AddRequestDTO<'a> {
    fn new<T: Into<Cow<'a, str>>>(kind: DnsType, domain: T) -> AddRequestDTO<'a> {
        AddRequestDTO {
            domain: domain.into(),
            kind: kind,

            admin_mail: "".into(),
            content: "".into(),
            priority: 10,
            weight: 0,
            port: 0,
            target: "".into(),
            subdomain: "@".into(),
            ttl: 21600,
        }
    }

    fn subdomain<T: Into<Cow<'a, str>>>(&mut self, value: T) -> &mut Self {
        self.subdomain = value.into();
        self
    }

    fn content<T: Into<Cow<'a, str>>>(&mut self, value: T) -> &mut Self {
        self.content = value.into();
        self
    }
}

#[derive(Debug)]
struct EditRequestDTO<'a> {
    domain: Cow<'a, str>,
    record_id: u64,

    subdomain: Option<Cow<'a, str>>, // default: "@"
    ttl: Option<u32>, // default: 21600, 900...21600
    refresh: Option<u32>, // for SOA, default: 10800, 900...86400
    retry: Option<u32>, // for SOA, default: 900, 90...3600
    expire: Option<u32>, // for SOA, default: 900, 90...3600
    neg_cache: Option<u32>, // for SOA, default: 10800, 90...86400
    admin_mail: Option<Cow<'a, str>>, // required for SOA
    content: Option<Cow<'a, str>>, // Ipv4 for A, Ipv6 for AAAA, string for CNAME, MX, NS, TXT
    priority: Option<u32>, // required for SRV and MX, default: 10
    port: Option<u16>, // required for SRV
    weight: Option<u32>, // required for SRV
    target: Option<Cow<'a, str>>, // required for SRV
}

impl<'a> EditRequestDTO<'a> {
    fn subdomain<T: Into<Cow<'a, str>>>(&mut self, value: T) -> &mut Self {
        self.subdomain = Some(value.into());
        self
    }

    fn content<T: Into<Cow<'a, str>>>(&mut self, value: T) -> &mut Self {
        self.content = Some(value.into());
        self
    }
}

#[derive(Debug)]
struct DeleteRequestDTO<'a> {
    domain: Cow<'a, str>,
    record_id: u64,
}

impl YandexDNS {
    pub fn new(token: &str) -> YandexDNS {
        YandexDNS {
            token: PddToken(token.to_owned()),
            client: Client::new()
        }
    }

    fn call<R: serde::Deserialize>(&mut self, func: &str, method: Method, args: &[(&str, &str)]) -> Result<R> {
        let url;
        let params = form_urlencoded::serialize(args);

        let mut resp = try! {
            match method {
                Method::Get | Method::Delete => {
                    url = format!("{}/{}?{}", BASE_URL, func, params);
                    self.client.request(method, &*url)
                },
                _ => {
                    url = format!("{}/{}", BASE_URL, func);
                    self.client.request(method, &*url).body(&*params)
                },
            }
            .header(self.token.clone())
            .header(ContentType("application/x-www-form-urlencoded".parse().unwrap()))
            .send()
        };

        let data = {
            let mut buf = String::new();
            try!(resp.read_to_string(&mut buf));
            buf
        };

        //println!("{}", data);

        try!(serde_json::from_str::<R>(&*data).map(Ok).or_else(
                |e| serde_json::from_str::<ErrorReplyDTO>(&*data).map(Err).map_err(|_| e))
        ).map_err(From::from)
    }

    pub fn send<V: YaVisitor>(&mut self, visitor: &V) -> Result<V::Reply> {
        visitor.visit(self)
    }

}

trait YaVisitor {
    type Reply: serde::Deserialize;
    fn visit(&self, api: &mut YandexDNS) -> Result<Self::Reply>;
}

impl<'a> YaVisitor for ListRequestDTO<'a> {
    type Reply = ListReplyDTO;
    fn visit(&self, api: &mut YandexDNS) -> Result<Self::Reply> {
        api.call("list", Method::Get, qs! {
            domain => self.domain.borrow(),
        })
    }
}

impl<'a> YaVisitor for AddRequestDTO<'a> {
    type Reply = AddReplyDTO;
    fn visit(&self, api: &mut YandexDNS) -> Result<Self::Reply> {
        api.call("add", Method::Post, qs! {
            domain => self.domain.borrow(),
            type => self.kind.as_ref(),

            admin_mail => self.admin_mail.borrow(),
            content => self.content.borrow(),
            priority => &*self.priority.to_string(),
            weight => &*self.weight.to_string(),
            port => &*self.port.to_string(),
            target => self.target.borrow(),

            subdomain => self.subdomain.borrow(),
            ttl => &*self.ttl.to_string(),
        })
    }
}

macro_rules! opt_borrow {
    ($val:expr) => {
        match $val { None => "", Some(ref val) => &**val }
    };
    (str $val:expr) => {
        match $val { None => "", Some(ref val) => &*val.to_string() }
    };
}

impl<'a> YaVisitor for EditRequestDTO<'a> {
    type Reply = EditReplyDTO;
    fn visit(&self, api: &mut YandexDNS) -> Result<Self::Reply> {
        let record_id = self.record_id.to_string();
        let refresh = self.refresh.map(|v| v.to_string());
        let retry = self.retry.map(|v| v.to_string());
        let expire = self.expire.map(|v| v.to_string());
        let neg_cache = self.neg_cache.map(|v| v.to_string());
        let priority = self.priority.map(|v| v.to_string());
        let port = self.port.map(|v| v.to_string());
        let weight = self.weight.map(|v| v.to_string());
        let ttl = self.ttl.map(|v| v.to_string());

        api.call("edit", Method::Post, qs! {
            domain => self.domain.borrow(),
            record_id => &*record_id,

            subdomain => opt_borrow!(self.subdomain),
            ttl => opt_borrow!(ttl),
            refresh => opt_borrow!(refresh),
            retry => opt_borrow!(retry),
            expire => opt_borrow!(expire),
            neg_cache => opt_borrow!(neg_cache),
            admin_mail => opt_borrow!(self.admin_mail),
            content => opt_borrow!(self.content),
            priority => opt_borrow!(priority),
            port => opt_borrow!(port),
            weight => opt_borrow!(weight),
            target => opt_borrow!(self.target),
        })
    }
}

impl<'a> YaVisitor for DeleteRequestDTO<'a> {
    type Reply = DeleteReplyDTO;
    fn visit(&self, api: &mut YandexDNS) -> Result<Self::Reply> {
        api.call("delete", Method::Post, qs! {
            domain => self.domain.borrow(),
            record_id => &*self.record_id.to_string(),
        })
    }
}

fn get_my_ip_address() -> Option<Ipv4Addr> {
    use std::net::{TcpStream, SocketAddr};
    let addr = TcpStream::connect("8.8.8.8:53").and_then(|s| s.local_addr());
    match addr {
        Ok(SocketAddr::V4(addr)) => Some(*addr.ip()),
        _ => None,
    }
}

fn main() {
    let token = option_env!("YANDEX_PDD_TOKEN").unwrap();
    let my_ip_addr = env::args().nth(4).or_else(|| get_my_ip_address().map(|v| v.to_string()));
    let mut yadns = YandexDNS::new(token);
    match yadns.send(&ListRequestDTO { domain: "kstep.me".into() })
        .unwrap().records.into_iter()
        .find(|rec| rec.kind == DnsType::A && rec.subdomain == "home") {
        Some(rec) => { yadns.send(rec.as_edit_req().content(&*my_ip_addr.unwrap())).unwrap(); },
        None => { yadns.send(AddRequestDTO::new(DnsType::A, "kstep.me").subdomain("home").content("127.0.0.1")).unwrap(); },
    }
}

