//! IP address validation: determines whether an address is publicly routable.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Returns `true` if the given IPv4 address is a globally routable public address.
///
/// Rejects: private (RFC 1918), loopback, link-local, broadcast, documentation,
/// benchmarking, reserved, shared (CGN), protocol assignment, and all other
/// non-globally-routable ranges.
fn is_global_ipv4(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    let first = octets[0];
    let second = octets[1];

    // 0.0.0.0/8 - "This" network (RFC 1122)
    if first == 0 {
        return false;
    }

    // 10.0.0.0/8 - Private (RFC 1918)
    if first == 10 {
        return false;
    }

    // 100.64.0.0/10 - Shared address space / CGN (RFC 6598)
    if first == 100 && (second & 0xC0) == 64 {
        return false;
    }

    // 127.0.0.0/8 - Loopback (RFC 1122)
    if first == 127 {
        return false;
    }

    // 169.254.0.0/16 - Link-local (RFC 3927)
    if first == 169 && second == 254 {
        return false;
    }

    // 172.16.0.0/12 - Private (RFC 1918)
    if first == 172 && (second & 0xF0) == 16 {
        return false;
    }

    // 192.0.0.9/32 and 192.0.0.10/32 are globally reachable anycast
    // exceptions inside the IETF protocol-assignment block.
    if first == 192 && second == 0 && octets[2] == 0 && (octets[3] == 9 || octets[3] == 10) {
        return true;
    }

    // 192.0.0.0/24 - IETF protocol assignments (RFC 6890), except above.
    if first == 192 && second == 0 && octets[2] == 0 {
        return false;
    }

    // 192.0.2.0/24 - Documentation TEST-NET-1 (RFC 5737)
    if first == 192 && second == 0 && octets[2] == 2 {
        return false;
    }

    // 192.88.99.0/24 - 6to4 relay anycast (RFC 7526, deprecated)
    if first == 192 && second == 88 && octets[2] == 99 {
        return false;
    }

    // 192.168.0.0/16 - Private (RFC 1918)
    if first == 192 && second == 168 {
        return false;
    }

    // 198.18.0.0/15 - Benchmarking (RFC 2544)
    if first == 198 && (second == 18 || second == 19) {
        return false;
    }

    // 198.51.100.0/24 - Documentation TEST-NET-2 (RFC 5737)
    if first == 198 && second == 51 && octets[2] == 100 {
        return false;
    }

    // 203.0.113.0/24 - Documentation TEST-NET-3 (RFC 5737)
    if first == 203 && second == 0 && octets[2] == 113 {
        return false;
    }

    // 224.0.0.0/4 - Multicast (RFC 5771)
    // 240.0.0.0/4 - Reserved for future use (RFC 1112) + 255.255.255.255 broadcast
    if first >= 224 {
        return false;
    }

    true
}

/// Returns `true` if the given IPv6 address is a globally routable public address.
///
/// Rejects: unspecified (::), loopback (::1), IPv4-mapped, IPv4-compatible,
/// link-local, site-local (deprecated), unique local (ULA), multicast,
/// documentation, discard, Teredo, 6to4, benchmarking, local-use NAT64,
/// ORCHID, SRv6 SID, and all other non-globally-routable ranges.
fn is_global_ipv6(ip: Ipv6Addr) -> bool {
    // Unspecified address (::)
    if ip.is_unspecified() {
        return false;
    }

    // Loopback (::1)
    if ip.is_loopback() {
        return false;
    }

    let segments = ip.segments();
    let first = segments[0];

    // IPv4-mapped (::ffff:0:0/96) is not globally reachable as IPv6.
    if first == 0
        && segments[1] == 0
        && segments[2] == 0
        && segments[3] == 0
        && segments[4] == 0
        && segments[5] == 0xFFFF
    {
        return false;
    }

    // IPv4-compatible (deprecated, ::/96 excluding :: and ::1)
    if first == 0
        && segments[1] == 0
        && segments[2] == 0
        && segments[3] == 0
        && segments[4] == 0
        && segments[5] == 0
    {
        return false;
    }

    // fe80::/10 - Link-local
    if (first & 0xFFC0) == 0xFE80 {
        return false;
    }

    // fec0::/10 - Site-local (deprecated, RFC 3879)
    if (first & 0xFFC0) == 0xFEC0 {
        return false;
    }

    // fc00::/7 - Unique local address (ULA, RFC 4193)
    if (first & 0xFE00) == 0xFC00 {
        return false;
    }

    // ff00::/8 - Multicast
    if (first & 0xFF00) == 0xFF00 {
        return false;
    }

    // 64:ff9b::/96 - Well-known IPv4/IPv6 translation prefix. Treat it as
    // global only when the embedded IPv4 address is global.
    if first == 0x0064
        && segments[1] == 0xFF9B
        && segments[2] == 0
        && segments[3] == 0
        && segments[4] == 0
        && segments[5] == 0
    {
        return is_global_ipv4(ipv4_from_segments(segments[6], segments[7]));
    }

    // 64:ff9b:1::/48 - Local-use IPv4/IPv6 translation prefix.
    if first == 0x0064 && segments[1] == 0xFF9B && segments[2] == 1 {
        return false;
    }

    // 100::/64 - Discard prefix (RFC 6666)
    if first == 0x0100 && segments[1] == 0 && segments[2] == 0 && segments[3] == 0 {
        return false;
    }

    // 100:0:0:1::/64 - Dummy IPv6 prefix.
    if first == 0x0100 && segments[1] == 0 && segments[2] == 0 && segments[3] == 1 {
        return false;
    }

    // Outside 2000::/3, IPv6 addresses are not global unicast unless handled
    // by a globally reachable special-purpose prefix above.
    if (first & 0xE000) != 0x2000 {
        return false;
    }

    // 2001::/23 - IETF protocol assignments. The broad block is not globally
    // reachable unless a more specific IANA allocation says it is.
    if first == 0x2001 && (segments[1] & 0xFE00) == 0 {
        return is_global_2001_special(segments);
    }

    // 2001:db8::/32 - Documentation (RFC 3849)
    if first == 0x2001 && segments[1] == 0x0DB8 {
        return false;
    }

    // 2002::/16 - 6to4 (RFC 3056)
    if first == 0x2002 {
        return false;
    }

    // 3fff::/20 - Documentation
    if first == 0x3FFF && (segments[1] & 0xF000) == 0 {
        return false;
    }

    // 5f00::/16 - Segment Routing (SRv6) SIDs
    if first == 0x5F00 {
        return false;
    }

    true
}

fn ipv4_from_segments(high: u16, low: u16) -> Ipv4Addr {
    Ipv4Addr::new((high >> 8) as u8, high as u8, (low >> 8) as u8, low as u8)
}

fn is_global_2001_special(segments: [u16; 8]) -> bool {
    match (segments[1], segments[2]) {
        // 2001:1::1/128, 2001:1::2/128, 2001:1::3/128
        (0x0001, 0)
            if segments[3] == 0
                && segments[4] == 0
                && segments[5] == 0
                && segments[6] == 0
                && matches!(segments[7], 1..=3) =>
        {
            true
        }
        // 2001:3::/32 - AMT
        (0x0003, _) => true,
        // 2001:4:112::/48 - AS112-v6
        (0x0004, 0x0112) => true,
        // 2001:20::/28 - ORCHIDv2
        (0x0020..=0x002F, _) => true,
        // 2001:30::/28 - Drone Remote ID Protocol Entity Tags
        (0x0030..=0x003F, _) => true,
        _ => false,
    }
}

/// Returns `true` if the IP address is a globally routable public address
/// suitable for geolocation lookup. Works for both IPv4 and IPv6.
pub fn is_global_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_global_ipv4(v4),
        IpAddr::V6(v6) => is_global_ipv6(v6),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- IPv4 tests ---

    #[test]
    fn public_ipv4_accepted() {
        let public_ips = [
            "1.1.1.1",
            "8.8.8.8",
            "45.77.77.77",
            "104.16.132.229",
            "223.255.255.255",
            "100.0.0.1",
            "100.63.255.255",
            "100.128.0.0",
            "198.17.255.255",
            "198.20.0.0",
        ];
        for s in public_ips {
            let ip: IpAddr = s.parse().unwrap();
            assert!(is_global_ip(ip), "{s} should be accepted as public");
        }
    }

    #[test]
    fn this_network_rejected() {
        let ips = ["0.0.0.0", "0.0.0.1", "0.255.255.255"];
        for s in ips {
            let ip: IpAddr = s.parse().unwrap();
            assert!(!is_global_ip(ip), "{s} should be rejected (this network)");
        }
    }

    #[test]
    fn private_rfc1918_rejected() {
        let ips = [
            "10.0.0.0",
            "10.0.0.1",
            "10.255.255.255",
            "172.16.0.0",
            "172.16.0.1",
            "172.31.255.255",
            "192.168.0.0",
            "192.168.0.1",
            "192.168.255.255",
        ];
        for s in ips {
            let ip: IpAddr = s.parse().unwrap();
            assert!(!is_global_ip(ip), "{s} should be rejected (RFC 1918)");
        }
    }

    #[test]
    fn private_boundary_not_rejected() {
        let ips = ["172.15.255.255", "172.32.0.0"];
        for s in ips {
            let ip: IpAddr = s.parse().unwrap();
            assert!(
                is_global_ip(ip),
                "{s} should be accepted (outside RFC 1918)"
            );
        }
    }

    #[test]
    fn loopback_rejected() {
        let ips = ["127.0.0.0", "127.0.0.1", "127.255.255.255"];
        for s in ips {
            let ip: IpAddr = s.parse().unwrap();
            assert!(!is_global_ip(ip), "{s} should be rejected (loopback)");
        }
    }

    #[test]
    fn link_local_rejected() {
        let ips = ["169.254.0.0", "169.254.0.1", "169.254.255.255"];
        for s in ips {
            let ip: IpAddr = s.parse().unwrap();
            assert!(!is_global_ip(ip), "{s} should be rejected (link-local)");
        }
    }

    #[test]
    fn cgn_shared_rejected() {
        let ips = ["100.64.0.0", "100.64.0.1", "100.127.255.255"];
        for s in ips {
            let ip: IpAddr = s.parse().unwrap();
            assert!(!is_global_ip(ip), "{s} should be rejected (CGN/shared)");
        }
    }

    #[test]
    fn documentation_nets_rejected() {
        let ips = [
            "192.0.2.0",
            "192.0.2.1",
            "192.0.2.255",
            "198.51.100.0",
            "198.51.100.255",
            "203.0.113.0",
            "203.0.113.255",
        ];
        for s in ips {
            let ip: IpAddr = s.parse().unwrap();
            assert!(!is_global_ip(ip), "{s} should be rejected (documentation)");
        }
    }

    #[test]
    fn benchmarking_rejected() {
        let ips = ["198.18.0.0", "198.18.0.1", "198.19.255.255"];
        for s in ips {
            let ip: IpAddr = s.parse().unwrap();
            assert!(!is_global_ip(ip), "{s} should be rejected (benchmarking)");
        }
    }

    #[test]
    fn ietf_protocol_rejected() {
        let ips = ["192.0.0.0", "192.0.0.1", "192.0.0.255"];
        for s in ips {
            let ip: IpAddr = s.parse().unwrap();
            assert!(!is_global_ip(ip), "{s} should be rejected (IETF protocol)");
        }
    }

    #[test]
    fn ietf_anycast_exceptions_accepted() {
        let ips = ["192.0.0.9", "192.0.0.10"];
        for s in ips {
            let ip: IpAddr = s.parse().unwrap();
            assert!(
                is_global_ip(ip),
                "{s} should be accepted (IETF anycast exception)"
            );
        }
    }

    #[test]
    fn relay_6to4_rejected() {
        let ips = ["192.88.99.0", "192.88.99.1", "192.88.99.255"];
        for s in ips {
            let ip: IpAddr = s.parse().unwrap();
            assert!(!is_global_ip(ip), "{s} should be rejected (6to4 relay)");
        }
    }

    #[test]
    fn multicast_rejected() {
        let ips = ["224.0.0.0", "224.0.0.1", "239.255.255.255"];
        for s in ips {
            let ip: IpAddr = s.parse().unwrap();
            assert!(!is_global_ip(ip), "{s} should be rejected (multicast)");
        }
    }

    #[test]
    fn reserved_and_broadcast_rejected() {
        let ips = [
            "240.0.0.0",
            "240.0.0.1",
            "255.255.255.254",
            "255.255.255.255",
        ];
        for s in ips {
            let ip: IpAddr = s.parse().unwrap();
            assert!(
                !is_global_ip(ip),
                "{s} should be rejected (reserved/broadcast)"
            );
        }
    }

    // --- IPv6 tests ---

    #[test]
    fn public_ipv6_accepted() {
        let public_ips = [
            "2606:4700:4700::1111",
            "2001:4860:4860::8888",
            "2400:cb00:2048:1::c629:d7a2",
            "64:ff9b::808:808",
            "2001:1::1",
            "2001:1::2",
            "2001:1::3",
            "2001:3::1",
            "2001:4:112::1",
            "2001:20::1",
            "2001:30::1",
            "2620:4f:8000::1",
        ];
        for s in public_ips {
            let ip: IpAddr = s.parse().unwrap();
            assert!(is_global_ip(ip), "{s} should be accepted as public");
        }
    }

    #[test]
    fn ipv6_unspecified_rejected() {
        let ip: IpAddr = "::".parse().unwrap();
        assert!(!is_global_ip(ip), ":: should be rejected (unspecified)");
    }

    #[test]
    fn ipv6_loopback_rejected() {
        let ip: IpAddr = "::1".parse().unwrap();
        assert!(!is_global_ip(ip), "::1 should be rejected (loopback)");
    }

    #[test]
    fn ipv6_link_local_rejected() {
        let ips = ["fe80::", "fe80::1", "fe80::ffff:ffff:ffff:ffff", "febf::1"];
        for s in ips {
            let ip: IpAddr = s.parse().unwrap();
            assert!(!is_global_ip(ip), "{s} should be rejected (link-local)");
        }
    }

    #[test]
    fn ipv6_site_local_rejected() {
        let ips = ["fec0::", "fec0::1", "feff::1"];
        for s in ips {
            let ip: IpAddr = s.parse().unwrap();
            assert!(!is_global_ip(ip), "{s} should be rejected (site-local)");
        }
    }

    #[test]
    fn ipv6_ula_rejected() {
        let ips = [
            "fc00::",
            "fc00::1",
            "fd00::1",
            "fdff:ffff:ffff:ffff:ffff:ffff:ffff:ffff",
        ];
        for s in ips {
            let ip: IpAddr = s.parse().unwrap();
            assert!(!is_global_ip(ip), "{s} should be rejected (ULA)");
        }
    }

    #[test]
    fn ipv6_multicast_rejected() {
        let ips = ["ff00::", "ff02::1", "ff0e::1"];
        for s in ips {
            let ip: IpAddr = s.parse().unwrap();
            assert!(!is_global_ip(ip), "{s} should be rejected (multicast)");
        }
    }

    #[test]
    fn ipv6_documentation_rejected() {
        let ips = [
            "2001:db8::",
            "2001:db8::1",
            "2001:db8:ffff:ffff:ffff:ffff:ffff:ffff",
            "3fff::",
            "3fff::1",
            "3fff:0fff:ffff:ffff:ffff:ffff:ffff:ffff",
        ];
        for s in ips {
            let ip: IpAddr = s.parse().unwrap();
            assert!(!is_global_ip(ip), "{s} should be rejected (documentation)");
        }
    }

    #[test]
    fn ipv6_teredo_rejected() {
        let ips = [
            "2001::",
            "2001::1",
            "2001:0000:ffff:ffff:ffff:ffff:ffff:ffff",
        ];
        for s in ips {
            let ip: IpAddr = s.parse().unwrap();
            assert!(!is_global_ip(ip), "{s} should be rejected (Teredo)");
        }
    }

    #[test]
    fn ipv6_discard_rejected() {
        let ips = [
            "100::",
            "100::1",
            "100::ffff:ffff:ffff:ffff",
            "100:0:0:1::",
            "100:0:0:1::1",
            "100:0:0:1:ffff:ffff:ffff:ffff",
        ];
        for s in ips {
            let ip: IpAddr = s.parse().unwrap();
            assert!(!is_global_ip(ip), "{s} should be rejected (discard)");
        }
    }

    #[test]
    fn ipv6_ipv4_mapped_rejected() {
        let ips = [
            "::ffff:127.0.0.1",
            "::ffff:10.0.0.1",
            "::ffff:192.168.1.1",
            "::ffff:0.0.0.0",
            "::ffff:1.1.1.1",
            "::ffff:8.8.8.8",
        ];
        for s in ips {
            let ip: IpAddr = s.parse().unwrap();
            assert!(!is_global_ip(ip), "{s} should be rejected (IPv4-mapped)");
        }
    }

    #[test]
    fn ipv6_ipv4_compatible_rejected() {
        let ips = ["::1.1.1.1", "::8.8.8.8"];
        for s in ips {
            let ip: IpAddr = s.parse().unwrap();
            assert!(
                !is_global_ip(ip),
                "{s} should be rejected (IPv4-compatible)"
            );
        }
    }

    #[test]
    fn ipv6_6to4_with_private_ipv4_rejected() {
        // 2002:0a00:0001:: embeds 10.0.0.1
        let ip: IpAddr = "2002:0a00:0001::".parse().unwrap();
        assert!(
            !is_global_ip(ip),
            "6to4 with private IPv4 should be rejected"
        );
    }

    #[test]
    fn ipv6_6to4_with_public_ipv4_rejected() {
        // 2002:0101:0101:: embeds 1.1.1.1
        let ip: IpAddr = "2002:0101:0101::".parse().unwrap();
        assert!(
            !is_global_ip(ip),
            "6to4 with public IPv4 should still be rejected"
        );
    }

    #[test]
    fn ipv6_special_purpose_non_global_rejected() {
        let ips = [
            "64:ff9b::10.0.0.1",
            "64:ff9b:1::",
            "64:ff9b:1::1",
            "2001:1::",
            "2001:1::4",
            "2001:2::1",
            "2001:10::1",
            "5f00::1",
            "4000::1",
        ];
        for s in ips {
            let ip: IpAddr = s.parse().unwrap();
            assert!(
                !is_global_ip(ip),
                "{s} should be rejected (special-purpose non-global)"
            );
        }
    }
}
