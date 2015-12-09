#![feature(ip_addr)]
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

use std::io::Read;
use std::net::IpAddr;
use std::borrow::Cow;
use hyper::Client;
use hyper::Result as HttpResult;
use hyper::Error as HttpError;
use hyper::header::{Header, HeaderFormat};
use hyper::header::ContentType;
use url::form_urlencoded;
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

#[derive(Debug)]
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

impl serde::Deserialize for DnsType {
    fn deserialize<D: serde::Deserializer>(d: &mut D) -> Result<DnsType, D::Error> {
        struct DnsTypeVisitor;

        impl serde::de::Visitor for DnsTypeVisitor {
            type Value = DnsType;
            fn visit_str<E: serde::de::Error>(&mut self, v: &str) -> Result<DnsType, E> {
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

#[derive(Debug, Deserialize)]
enum Priority {
    Empty(String),
    Value(u32),
}

#[derive(Debug, Deserialize)]
struct RecordDTO {
    record_id: u64,
    #[serde(rename="type")]
    kind: DnsType,
    domain: String,
    subdomain: String,
    fqdn: String,
    content: String,
    ttl: u32,

    priority: Priority,

    // SOA
    refresh: Option<u32>,
    admin_mail: Option<String>,
    expire: Option<u32>,
    minttl: Option<u32>,

    // SRV
    weight: Option<u32>,
    port: Option<u16>,

    // edit
    operation: Option<String>,
}

#[derive(Deserialize)]
struct ListReplyDTO {
    records: Vec<RecordDTO>,
    domain: String,
    success: String,
}

#[derive(Deserialize)]
struct EditReplyDTO {
    domain: String,
    record_id: u64,
    record: RecordDTO,
    success: String,
}

#[derive(Deserialize)]
struct AddReplyDTO {
    domain: String,
    record: RecordDTO,
    success: String,
}

#[derive(Deserialize)]
struct DeleteReplyDTO {
    domain: String,
    record_id: u64,
    success: String,
}

#[derive(Deserialize)]
struct ErrorReplyDTO {
    domain: String,
    success: String,
    error: ErrorCode,
}

#[derive(Debug, Clone)]
enum ErrorCode {
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
    NoReply,
}

impl serde::Deserialize for ErrorCode {
    fn deserialize<D: serde::Deserializer>(d: &mut D) -> Result<ErrorCode, D::Error> {
        struct ErrorCodeVisitor;

        impl serde::de::Visitor for ErrorCodeVisitor {
            type Value = ErrorCode;
            fn visit_str<E: serde::de::Error>(&mut self, v: &str) -> Result<ErrorCode, E> {
                use self::ErrorCode::*;
                match v {
                    "unknown" => Ok(Unknown),
                    "no_token" => Ok(NoToken),
                    "no_domain" => Ok(NoDomain),
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

        d.visit(ErrorCodeVisitor)
    }
}

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

struct DeleteRequestDTO<'a> {
    domain: Cow<'a, str>,
    record_id: u64,
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

    fn call<T: serde::Deserialize>(&mut self, method: &str, args: &[(&str, &str)]) -> HttpResult<T> {
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

        let data = {
            let mut buf = String::new();
            try!(response.read_to_string(&mut buf));
            buf
        };
        println!("reply: {:?}", data);
        Ok(serde_json::from_str(data.trim()).unwrap())
    }

    pub fn list(&mut self) -> HttpResult<Vec<RecordDTO>> {
        let reply: ListReplyDTO = try!(self.call("list", &[]));
        Ok(reply.records)
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
