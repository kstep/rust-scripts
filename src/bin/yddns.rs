#![allow(unstable)]

extern crate xml;
extern crate pb;
extern crate hyper;
extern crate "script-utils" as utils;
extern crate url;
extern crate time;

use hyper::{Client, HttpResult};
use url::form_urlencoded;
use xml::reader::EventReader;
use xml::reader::events::XmlEvent;
use std::time::Duration;
use time::Timespec;

struct Config {
    domain: String,
    token: String
}

struct PbConfig {
    access_token: String
}

struct NSRecord {
    kind: String,
    subdomain: String
}

static BASE_URL: &'static str = "https://pddimp.yandex.ru/nsapi/";

macro_rules! qs {
    ($($key:expr => $value:expr),*) => {
        vec![$(($key, $value)),*]
    }
}


struct YandexDNS {
    id: String,
    domain: String,
    token: String,
    client: Client
}

impl YandexDNS {
    pub fn new(domain: &str, token: &str) -> YandexDNS {
        YandexDNS {
            domain: domain.to_string(),
            token: token.to_string(),
            client: Client::new()
        }
    }

    fn call(method: &str, args: &[(&str, &str)]) -> HttpResult<()> {
        client.get(format!("https://pddimp.yandex.ru/nsapi/{}.xml?{}", method, form_urlencoded::serialize(args)))
    }

    pub fn load(&mut self) -> HttpResult<Vec<NSRecord>> {
    }
}

enum NSData {
    A { address: IpAddr },
    AAAA { address: IpAddr },
    PTR { hostname: String },
    CNAME { hostname: String },
    NS { nsserver: String },
    MX { hostname: String },
    TXT { payload: String },
    SOA {
        refresh: Duration,
        retry: Duration,
        expire: Timespec,
        minttl: Duration,
        admin_mail: String,
        nsserver: String
    },
    SRV {
        weight: u16,
        hostname: String,
        port: u16
    },
}

struct NSRecord {
    id: u32,
    domain: String,
    subdomain: String,
    priority: u16,
    ttl: Duration,
    data: NSData,
}

trait FromXml {
    fn from_xml(event: &XmlEvent, reader: &mut EventReader) -> Option<Self>;
}

impl FromXml for NSData {
    fn from_xml(event: &XmlEvent, reader: &mut EventReader) -> Option<NSData> {
    }
}


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
