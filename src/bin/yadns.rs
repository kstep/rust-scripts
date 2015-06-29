#![feature(ip_addr)]

// TODO
#![allow(dead_code, unused_variables)]

extern crate xml;
extern crate pb;
extern crate hyper;
extern crate script_utils as utils;
extern crate url;
extern crate time;
extern crate rustc_serialize;

use std::io::Read;
use std::net::IpAddr;
use hyper::Client;
use hyper::Result as HttpResult;
use hyper::Error as HttpError;
use hyper::header::{Header, HeaderFormat};
use hyper::header::ContentType;
use url::form_urlencoded;
use rustc_serialize::{json, Decodable, Decoder};
use time::Duration;

#[derive(Debug, Clone)]
struct Config {
    domain: String,
    token: String
}

#[derive(Debug, Clone)]
struct PbConfig {
    access_token: String
}

static BASE_URL: &'static str = "https://pddimp.yandex.ru/api2/admin/dns";

macro_rules! qs {
    ($($key:expr => $value:expr),*) => {
        vec![$(($key, $value)),*]
    }
}

#[derive(Debug, Clone)]
struct PddToken(String);

impl Header for PddToken {
    fn header_name() -> &'static str {
        "PddToken"
    }

    fn parse_header(raw: &[Vec<u8>]) -> Result<PddToken, HttpError> {
        Ok(PddToken(String::from_utf8_lossy(&*raw[0]).into_owned()))
    }
}

impl HeaderFormat for PddToken {
    fn fmt_header(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        let PddToken(ref value) = *self;
        fmt.write_str(&**value)
    }
}

struct YandexDNS {
    id: String,
    domain: String,
    token: PddToken,
    client: Client
}

impl YandexDNS {
    pub fn new(domain: &str, token: &str) -> YandexDNS {
        YandexDNS {
            id: String::new(),
            domain: domain.to_owned(),
            token: PddToken(token.to_owned()),
            client: Client::new()
        }
    }

    fn call(&mut self, method: &str, args: &[(&str, &str)]) -> HttpResult<NSReply> {
        let url;
        let params;

        let mut response = try!(
            if args.len() == 0 {
                url = format!("{}/{}?domain={}", BASE_URL, method, self.domain);
                self.client.get(&*url)
            } else {
                url = format!("{}/{}", BASE_URL, method);
                params = format!("{}&domain={}", form_urlencoded::serialize(args), self.domain);
                self.client.post(&*url)
                    .body(&*params)
            }
            .header(self.token.clone())
            .header(ContentType("application/x-www-form-urlencoded".parse().unwrap()))
            .send());

        let json : NSReply = {
            let mut buf = String::new();
            try!(response.read_to_string(&mut buf));
            json::decode(&*buf).unwrap()
        };

        Ok(json)
    }

    pub fn list(&mut self) -> HttpResult<Vec<NSRecord>> {
        let reply = try!(self.call("list", &[]));
        Ok(reply.records.unwrap())
    }

/*
    pub fn add(&mut self, record: NSRecord) -> HttpResult<NSReply> {
        let reply = try!(self.call("add", &*qs![
            "type" => record.data.get_type(),
            //"admin_mail" => &*record.data.admin_mail,
            //"content" => &*record.data.get_content(),
            //"priority" => record.data.priority,
            //"weight" => record.data.weight,
            //"port" => record.data.port,
            "target" => &*record.fqdn,
            "subdomain" => &*record.subdomain,
            "ttl" => &*record.ttl.num_seconds().to_string()
        ]));
        Ok(reply)
    }
    */
}

#[derive(Debug, Clone)]
enum NSData {
    A { address: IpAddr },
    AAAA { address: IpAddr },
    PTR { hostname: String },
    CNAME { hostname: String },
    NS { nsserver: String },
    MX { hostname: String, priority: u16 },
    TXT { payload: String },
    SOA {
        refresh: Duration,
        retry: Duration,
        expire: Duration,
        minttl: Duration,
        admin_mail: String,
        nsserver: String
    },
    SRV {
        weight: u16,
        hostname: String,
        port: u16,
        priority: u16
    },
}

impl NSData {
    fn get_content(&self) -> String {
        match *self {
            NSData::A { ref address } => address.to_string(),
            NSData::AAAA { ref address } => address.to_string(),
            NSData::PTR { ref hostname } => hostname.clone(),
            NSData::CNAME { ref hostname } => hostname.clone(),
            NSData::NS { ref nsserver } => nsserver.clone(),
            NSData::MX { ref hostname, .. } => hostname.clone(),
            NSData::TXT { ref payload } => payload.clone(),
            NSData::SOA { ref nsserver, .. } => nsserver.clone(),
            NSData::SRV { ref hostname, .. } => hostname.clone()
        }
    }
    fn get_type(&self) -> &'static str {
        match *self {
            NSData::A { .. } => "A",
            NSData::AAAA { .. } => "AAAA",
            NSData::PTR { .. } => "PTR",
            NSData::CNAME { .. } => "CNAME",
            NSData::NS { .. } => "NS",
            NSData::MX { .. } => "MX",
            NSData::TXT { .. } => "TXT",
            NSData::SOA { .. } => "SOA",
            NSData::SRV { .. } => "SRV",
        }
    }
}

#[derive(Debug, Clone)]
struct NSRecord {
    id: u32,
    domain: String,
    subdomain: String,
    fqdn: String,
    ttl: Duration,
    data: NSData,
}

#[derive(Debug, Clone)]
enum NSErrorKind {
    Unknown,
    NoToken,
    NoDomain,
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
    NoReply
}

impl Decodable for NSErrorKind {
    fn decode<D: Decoder>(d: &mut D) -> Result<NSErrorKind, D::Error> {
        match &*try!(d.read_str()) {
            "unknown" => Ok(NSErrorKind::Unknown),
            "no_token" => Ok(NSErrorKind::NoToken),
            "no_domain" => Ok(NSErrorKind::NoDomain),
            "no_ip" => Ok(NSErrorKind::NoIp),
            "bad_domain" => Ok(NSErrorKind::BadDomain),
            "prohibited" => Ok(NSErrorKind::Prohibited),
            "bad_token" => Ok(NSErrorKind::BadToken),
            "bad_login" => Ok(NSErrorKind::BadLogin),
            "bad_password" => Ok(NSErrorKind::BadPasswd),
            "no_auth" => Ok(NSErrorKind::NoAuth),
            "not_allowed" => Ok(NSErrorKind::NotAllowed),
            "blocked" => Ok(NSErrorKind::Blocked),
            "occupied" => Ok(NSErrorKind::Occupied),
            "domain_limit_reached" => Ok(NSErrorKind::DomainLimitReached),
            "no_reply" => Ok(NSErrorKind::NoReply),
            _ => Err(d.error("invalid error code"))
        }
    }
}

#[derive(Debug)]
enum NSError {
    Proto(NSErrorKind),
    Http(HttpError)
}

#[derive(RustcDecodable, Debug, Clone)]
struct NSReply {
    domain: String,
    record_id: Option<u32>,
    record: Option<NSRecord>,
    records: Option<Vec<NSRecord>>,
    error: Option<NSErrorKind>,
    success: NSReplyStatus
}

#[derive(Debug, Clone, Copy)]
enum NSReplyStatus {
    Ok,
    Err
}

impl Decodable for NSReplyStatus {
    fn decode<D: Decoder>(d: &mut D) -> Result<NSReplyStatus, D::Error> {
        match d.read_str() {
            Ok(s) => match &*s {
                "ok" => Ok(NSReplyStatus::Ok),
                "error" => Ok(NSReplyStatus::Err),
                _ => Err(d.error("unknown reply status"))
            },
            Err(e) => Err(e)
        }
    }
}

impl Decodable for NSRecord {
    fn decode<D: Decoder>(d: &mut D) -> Result<NSRecord, D::Error> {
        d.read_struct("NSRecord", 12, |d| {
            let data = match &*try!(d.read_struct_field("type", 0, |d| d.read_str())) {
                "A" => NSData::A {
                    address: match try!(d.read_struct_field("content", 0, |d| d.read_str())).parse() {
                        Ok(a) => a,
                        Err(_) => return Err(d.error("invalid ipv4 address"))
                    }
                },
                "AAAA" => NSData::AAAA {
                    address: match try!(d.read_struct_field("content", 0, |d| d.read_str())).parse() {
                        Ok(a) => a,
                        Err(_) => return Err(d.error("invalid ipv6 address"))
                    }
                },
                "CNAME" => NSData::CNAME {
                    hostname: try!(d.read_struct_field("content", 0, |d| d.read_str()))
                },
                "PTR" => NSData::PTR {
                    hostname: try!(d.read_struct_field("content", 0, |d| d.read_str()))
                },
                "MX" => NSData::MX {
                    hostname: try!(d.read_struct_field("content", 0, |d| d.read_str())),
                    priority: try!(d.read_struct_field("priority", 0, |d| d.read_u16()))
                },
                "NS" => NSData::NS {
                    nsserver: try!(d.read_struct_field("content", 0, |d| d.read_str()))
                },
                "SRV" => NSData::SRV {
                    hostname: try!(d.read_struct_field("content", 0, |d| d.read_str())),
                    weight: try!(d.read_struct_field("weight", 0, |d| d.read_u16())),
                    port: try!(d.read_struct_field("port", 0, |d| d.read_u16())),
                    priority: try!(d.read_struct_field("priority", 0, |d| d.read_u16()))
                },
                "TXT" => NSData::TXT {
                    payload: try!(d.read_struct_field("content", 0, |d| d.read_str()))
                },
                "SOA" => NSData::SOA {
                    nsserver: try!(d.read_struct_field("content", 0, |d| d.read_str())),
                    refresh: Duration::seconds(try!(d.read_struct_field("refresh", 0, |d| d.read_i64()))),
                    retry: Duration::seconds(try!(d.read_struct_field("retry", 0, |d| d.read_i64()))),
                    expire: Duration::seconds(try!(d.read_struct_field("expire", 0, |d| d.read_i64()))),
                    minttl: Duration::seconds(try!(d.read_struct_field("minttl", 0, |d| d.read_i64()))),
                    admin_mail: try!(d.read_struct_field("admin_mail", 0, |d| d.read_str()))
                },
                _ => return Err(d.error("unknown record type"))
            };
            Ok(NSRecord {
                data: data,
                id: try!(d.read_struct_field("record_id", 0, |d| d.read_u32())),
                domain: try!(d.read_struct_field("domain", 0, |d| d.read_str())),
                fqdn: try!(d.read_struct_field("fqdn", 0, |d| d.read_str())),
                subdomain: try!(d.read_struct_field("subdomain", 0, |d| d.read_str())),
                ttl: Duration::seconds(try!(d.read_struct_field("ttl", 0, |d| d.read_i64()))),
            })
        })
    }
}

fn main() {
    let token = option_env!("YANDEX_PDD_TOKEN").unwrap();
    let mut yadns = YandexDNS::new("kstep.me", token);
    println!("{:?}", yadns.list());
}

/*
<?xml version="1.0" encoding="utf-8"?>
<page>

    <domains>
        <domain>
            <name>kstep.me</name>
            <response>
                <record domain="home.kstep.me" priority="" ttl="21600" subdomain="home" type="A" id="19709628">81.25.37.180</record>
                <record domain="kstep.me" priority="" ttl="21600" subdomain="@" type="A" id="19707758">192.30.252.153</record>
                <record domain="kstep.me" priority="" ttl="21600" subdomain="@" type="A" id="19707772">192.30.252.154</record>
                <record domain="mail.kstep.me" priority="" ttl="21600" subdomain="mail" type="CNAME" id="6028797">domain.mail.yandex.net.</record>
                <record domain="www.kstep.me" priority="" ttl="21600" subdomain="www" type="CNAME" id="6028792">kstep.me.</record>
                <record domain="_xmpp-client._tcp.conference.kstep.me" priority="20" ttl="21600" subdomain="_xmpp-client._tcp.conference" type="SRV" id="6028798" weight="0" port="5222">domain-xmpp.ya.ru.</record>
                <record domain="_xmpp-client._tcp.kstep.me" priority="20" ttl="21600" subdomain="_xmpp-client._tcp" type="SRV" id="6028795" weight="0" port="5222">domain-xmpp.ya.ru.</record>
                <record domain="_xmpp-server._tcp.conference.kstep.me" priority="20" ttl="21600" subdomain="_xmpp-server._tcp.conference" type="SRV" id="6028796" weight="0" port="5269">domain-xmpp.ya.ru.</record>
                <record domain="_xmpp-server._tcp.kstep.me" priority="20" ttl="21600" subdomain="_xmpp-server._tcp" type="SRV" id="6028794" weight="0" port="5269">domain-xmpp.ya.ru.</record>
                <record domain="kstep.me" priority="" ttl="21600" subdomain="@" type="TXT" id="6028791">v=spf1 redirect=_spf.yandex.ru</record>
                <record domain="mail._domainkey.kstep.me" priority="" ttl="21600" subdomain="mail._domainkey" type="TXT" id="10580569">v=DKIM1; k=rsa; t=s; p=MIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQDFWtyxRo8fvax96VU6QoG0qCRjACufIKCopDeWqnzH+bytU4+zyqS9rC/32wy7pZjY3CLPakqQF7Uu1BXY7jMVHI8teiELVg+MJVj2ic9ZYsy7dDDX5iov2cXRJn2bQauhxw1W/HnYIPVMV5mxpF4U0FVsALpQqKFPr9jKpyu/qQIDAQAB</record>
                <record domain="kstep.me" priority="" ttl="21600" subdomain="@" type="SOA" id="6028799" refresh="14400" retry="900" expire="1209600" minttl="14400" admin_mail="milezv.yandex.ru">dns1.yandex.ru.</record>
                <record domain="kstep.me" priority="" ttl="21600" subdomain="@" type="NS" id="6028788">dns1.yandex.ru.</record>
                <record domain="kstep.me" priority="" ttl="21600" subdomain="@" type="NS" id="6028789">dns2.yandex.ru.</record>
                <record domain="home.kstep.me" priority="10" ttl="21600" subdomain="home" type="MX" id="19870652">home.kstep.me.</record>
                <record domain="kstep.me" priority="10" ttl="21600" subdomain="@" type="MX" id="6028793">mx.yandex.ru.</record>
            </response>
            <nsdelegated/>
        </domain>
        <error>ok</error>
    </domains>
</page>
*/
