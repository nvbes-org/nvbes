use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

pub fn parse_ip(value: &str) -> Option<IpAddr> {
    value.trim().parse::<IpAddr>().ok()
}

pub fn is_private_or_special_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => is_private_or_special_ipv4(ip),
        IpAddr::V6(ip) => is_private_or_special_ipv6(ip),
    }
}

fn is_private_or_special_ipv4(ip: Ipv4Addr) -> bool {
    ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_documentation()
        || ip.is_unspecified()
        || ip.octets()[0] == 0
        || ip.octets()[0] >= 224
}

fn is_private_or_special_ipv6(ip: Ipv6Addr) -> bool {
    let first = ip.segments()[0];
    ip.is_loopback()
        || ip.is_unspecified()
        || (first & 0xfe00) == 0xfc00
        || (first & 0xffc0) == 0xfe80
        || (first & 0xff00) == 0xff00
        || (ip.segments()[0] == 0x2001 && ip.segments()[1] == 0x0db8)
}

#[cfg(test)]
mod tests {
    use super::{is_private_or_special_ip, parse_ip};

    #[test]
    fn detects_private_and_special_addresses() {
        assert!(is_private_or_special_ip(parse_ip("10.0.0.1").unwrap()));
        assert!(is_private_or_special_ip(parse_ip("127.0.0.1").unwrap()));
        assert!(is_private_or_special_ip(parse_ip("2001:db8::1").unwrap()));
        assert!(!is_private_or_special_ip(parse_ip("8.8.8.8").unwrap()));
    }
}
