# RustyIP

[![CI](https://github.com/NetworkCats/rustyip/actions/workflows/ci.yml/badge.svg)](https://github.com/NetworkCats/rustyip/actions/workflows/ci.yml)
[![CodeQL](https://github.com/NetworkCats/rustyip/actions/workflows/github-code-scanning/codeql/badge.svg)](https://github.com/NetworkCats/rustyip/actions/workflows/github-code-scanning/codeql/)
[![codecov](https://codecov.io/gh/NetworkCats/rustyip/branch/main/graph/badge.svg)](https://codecov.io/gh/NetworkCats/rustyip)

Live demo: [ip.nc.gy](ip.nc.gy)

### What

RustyIP is a lightweight IP lookup service with both a web interface and an API. The frontend is minimal and works without JavaScript. The backend is written in Rust and handles high traffic reliably.

This is the only free IP lookup service with accurate proxy detection. We maintain our own exclusive proxy detection database, which is free and open source: https://github.com/NetworkCats/OpenProxyDB

We also improve geolocation coverage by merging multiple free geolocation databases into one. That's open source too: https://github.com/NetworkCats/Merged-IP-Data

RustyIP uses the best open source IP data available, delivers a clean user experience, and has zero ads or tracking.

### Why

There are plenty of IP lookup sites and APIs out there, but most of them rely on the free GeoLite2 database and don't include any proxy information. On top of that, they're bloated with JavaScript and ads.

Since we already had OpenProxyDB, an accurate proxy detection database, it made sense to build a site where users can easily look up IP info. So we did.

We've privately benchmarked OpenProxyDB against commercial databases like IPinfo, and the accuracy holds up. It even outperforms them when it comes to detecting residential proxies. If you have an IPinfo or MaxMind API subscription, feel free to run your own tests.

### How

You can use curl, HTTPie, wget, or any HTTP client. Here are some curl examples:

Get your current IP:
```
curl ip.nc.gy
```

Look up info about your own IP:
```
curl ip.nc.gy/asn

curl ip.nc.gy/org

curl ip.nc.gy/country

curl ip.nc.gy/city

curl ip.nc.gy/proxy

curl ip.nc.gy/vpn

curl ip.nc.gy/hosting

curl ip.nc.gy/tor
```

Or get everything as JSON:
```
curl ip.nc.gy/json
{
  "ip": "203.0.113.42",
  "city": {
    "geoname_id": 5128581,
    "names": {
      "en": "New York"
    }
  },
  "continent": {
    "code": "NA",
    "geoname_id": 6255149,
    "names": {
      "en": "North America"
    }
  },
  "country": {
    "geoname_id": 6252001,
    "iso_code": "US",
    "names": {
      "en": "United States"
    }
  },
  "location": {
    "accuracy_radius": 1000,
    "latitude": 40.7128,
    "longitude": -74.006,
    "metro_code": 501,
    "time_zone": "America/New_York"
  },
  "postal": {
    "code": "10001"
  },
  "subdivisions": [
    {
      "geoname_id": 5128638,
      "iso_code": "NY",
      "names": {
        "en": "New York"
      }
    }
  ],
  "asn": {
    "autonomous_system_number": 13335,
    "autonomous_system_organization": "Cloudflare, Inc.",
    "as_domain": "cloudflare.com"
  },
  "proxy": {
    "is_proxy": false,
    "is_vpn": false,
    "is_tor": false,
    "is_hosting": false,
    "is_cdn": false,
    "is_school": false,
    "is_anonymous": false
  }
}
```

Look up a specific IP:
```
curl ip.nc.gy/json?ip=1.1.1.1
{
  "ip": "1.1.1.1",
  "continent": {
    "code": "OC",
    "geoname_id": 6255151,
    "names": {
      "en": "Oceania"
    }
  },
  "country": {
    "geoname_id": 2077456,
    "iso_code": "AU",
    "names": {
      "en": "Australia"
    }
  },
  "location": {
    "accuracy_radius": 1000,
    "latitude": -33.494,
    "longitude": 143.2104,
    "time_zone": "Australia/Sydney"
  },
  "asn": {
    "autonomous_system_number": 13335,
    "autonomous_system_organization": "Cloudflare, Inc.",
    "as_domain": "cloudflare.com"
  },
  "proxy": {
    "is_proxy": false,
    "is_vpn": false,
    "is_tor": false,
    "is_hosting": false,
    "is_cdn": true,
    "is_school": false,
    "is_anonymous": false
  }
}
```

Use `curl -4` or `curl -6` to force IPv4 or IPv6.

See `openapi.json` in this repo for the full API spec.

### Data Sources

Geolocation data is sourced from MaxMind and DB-IP free databases. AS data comes from IPinfo's free database. Proxy data comes from our own [OpenProxyDB](https://github.com/NetworkCats/OpenProxyDB).

Attributions:

- Maxmind GeoLite2: [Creative Commons Attribution-ShareAlike 4.0 International License](https://creativecommons.org/licenses/by-sa/4.0/)
- IPinfo Lite: [Creative Commons Attribution-ShareAlike 4.0 International License](https://creativecommons.org/licenses/by-sa/4.0/)
- DB-IP: [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/)
- RouteViews ASN: [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/)
- OpenProxyDB: [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/)