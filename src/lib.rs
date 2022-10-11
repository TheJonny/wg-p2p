use std::net::IpAddr;

pub mod parse;

pub fn unmap_addr(a: IpAddr) -> IpAddr {
    if let IpAddr::V6(v6) = a {
        let x: u128 = v6.into();
        if x & 0xffff_ffff_ffff_ffff_ffff_ffff_0000_0000 == 0xffff_0000_0000 {
            let v4 = std::net::Ipv4Addr::from(x as u32);
            return IpAddr::V4(v4);
        }
    }
    return a;
}
