#![feature(custom_derive, plugin)]
#![feature(custom_attribute)]
#![plugin(serde_macros)]

extern crate pb;
extern crate script_utils as utils;
extern crate yadns;
extern crate serde;
extern crate lettre;

use std::env;
use std::net::Ipv4Addr;
use yadns::{YandexDNS, ListRequest, AddRequest, DnsType};
use pb::{PbAPI, PushMsg, TargetIden, Push, PushData};
use lettre::transport::smtp::SmtpTransportBuilder;
use lettre::transport::EmailTransport;
use lettre::email::EmailBuilder;

#[derive(Debug, Clone, Deserialize)]
struct Config {
    domain: String,
    subdomain: String,
    token: String,
}

#[derive(Debug, Clone, Deserialize)]
struct PbConfig {
    access_token: String,
    device_iden: Option<String>,
}

fn get_my_ip_address() -> Option<Ipv4Addr> {
    use std::net::{TcpStream, SocketAddr};
    let addr = TcpStream::connect(("8.8.8.8", 53)).and_then(|s| s.local_addr());
    match addr {
        Ok(SocketAddr::V4(addr)) => Some(*addr.ip()),
        _ => None,
    }
}

fn main() {
    let pbcfg = utils::load_config::<PbConfig>("pushbullet/config.toml").unwrap();
    let mut pbapi = PbAPI::new(&*pbcfg.access_token);

    let config = utils::load_config::<Config>("yadns/config.toml").unwrap();

    let my_ip_addr = env::args()
                         .nth(4)
                         .or_else(|| get_my_ip_address().map(|v| v.to_string()))
                         .unwrap();

    let mut yadns = YandexDNS::new(&*config.token);
    let home_record = yadns.send(&ListRequest::new(&*config.domain))
                           .unwrap()
                           .records
                           .into_iter()
                           .find(|rec| rec.kind == DnsType::A && rec.subdomain == config.subdomain);

    match home_record {
        Some(rec) => {
            yadns.send(rec.as_edit_req()
                          .content(&*my_ip_addr))
                 .unwrap();
        }
        None => {
            yadns.send(AddRequest::new(DnsType::A, &*config.domain)
                           .subdomain(&*config.subdomain)
                           .content(&*my_ip_addr))
                 .unwrap();
        }
    }

    let push = PushMsg {
        title: Some(String::new("New home IP address")),
        // TODO: this clone is not really necessary most of time
        body: Some(my_ip_addr.clone()),
        target: TargetIden::CurrentUser,
        data: PushData::Note,
        source_device_iden: pbcfg.device_iden,
    };

    match pbapi.send(&push) {
        Ok(Push { iden, .. }) => println!("notified with push {}", iden),
        Err(err) => {
            println!("push notification failed with error: {}", err);
            println!("trying to send email...");
            if let Ok(mut mailer) = SmtpTransportBuilder::localhost().map(|t| t.build()) {
                if let Ok(email) = EmailBuilder::new()
                                       .from("greybook@home.kstep.me")
                                       .to(("me@kstep.me", "Master"))
                                       .subject("New external IP address")
                                       .body(&*format!("Hi, Master!

Just for your \
                                                        information, my new external IP \
                                                        address is {}.

Regards,
Greybook.",
                                                       my_ip_addr))
                                       .build() {
                    match mailer.send(email) {
                        Ok(_) => println!("notified with email to me@kstep.me"),
                        Err(err) => println!("email notification failed with error: {}", err),
                    }
                }
            }
        }
    }

}
