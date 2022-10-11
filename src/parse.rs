use crate::unmap_addr;
use ini::Ini;
use std::collections::HashMap;
use std::net::IpAddr;

fn first_ip(allowed_ips: &str) -> Option<IpAddr> {
    let ippre = match allowed_ips.find(',') {
        Some(i) => &allowed_ips[..i],
        None => allowed_ips,
    };
    let ippre = ippre.trim();
    let (ip, pre) = match ippre.find(',') {
        Some(i) => (&ippre[..i], ippre[i + 1..].parse::<i32>().unwrap()),
        None => (ippre, -1),
    };

    if let Ok(addr) = ip.parse::<IpAddr>() {
        if pre == -1 || (addr.is_ipv4() && pre == 32) || (addr.is_ipv6() && pre == 128) {
            return Some(unmap_addr(addr));
        }
    }
    return None;
}

pub fn parse_ini(fname: &str) -> anyhow::Result<HashMap<IpAddr, String>> {
    let ini = Ini::load_from_file(fname)?;
    let mut peers = HashMap::<IpAddr, String>::new();
    for (name, section) in ini.iter() {
        let mut ip: Option<IpAddr> = None;
        let mut pubkey = None;
        if name.filter(|&s| &s.to_lowercase() == "peer").is_some() {
            for (k, v) in section.iter() {
                if ip.is_none() && &k.to_lowercase() == "allowedips" {
                    ip = first_ip(v);
                }
                if &k.to_lowercase() == "publickey" {
                    pubkey = Some(v.to_owned());
                }
            }
            match (ip, pubkey) {
                (Some(ip), Some(pubkey)) => {
                    peers.insert(ip, pubkey);
                }
                (ip, pubkey) => {
                    eprintln!("skipping section: {:?}, {:?}", ip, pubkey);
                }
            }
        }
    }
    return Ok(peers);
}
