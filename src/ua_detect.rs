//! User-agent detection for distinguishing CLI/HTTP-library clients from browsers.

const CLI_PREFIXES: &[&str] = &[
    "curl/",
    "Wget/",
    "HTTPie/",
    "xh/",
    "fetch",
    "PowerShell/",
    "WindowsPowerShell/",
];

const HTTP_LIB_PREFIXES: &[&str] = &[
    // Python
    "python-requests/",
    "python-httpx/",
    "python-urllib3/",
    "aiohttp/",
    "httplib2/",
    // Go
    "Go-http-client/",
    // Ruby
    "rest-client/",
    "Ruby",
    "Faraday v",
    "HTTParty/",
    // Java / JVM
    "Java/",
    "Apache-HttpClient/",
    "okhttp/",
    // .NET / C#
    "HttpClient/",
    // Node.js
    "node-fetch/",
    "axios/",
    "undici",
    "got (",
    // PHP
    "GuzzleHttp/",
    "Symfony HttpClient/",
    // Rust
    "reqwest/",
    "hyper/",
    "ureq/",
    // Dart / Flutter
    "Dart/",
    // Perl
    "libwww-perl/",
    "LWP::Simple/",
    // R
    "libcurl/",
    "httr/",
    // Elixir
    "hackney/",
    "mint/",
];

fn matches_any_prefix(user_agent: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|prefix| user_agent.starts_with(prefix))
}

fn is_cli_user_agent(user_agent: &str) -> bool {
    matches_any_prefix(user_agent, CLI_PREFIXES)
}

fn is_http_lib_user_agent(user_agent: &str) -> bool {
    matches_any_prefix(user_agent, HTTP_LIB_PREFIXES)
}

pub fn is_plain_text_agent(user_agent: &str) -> bool {
    is_cli_user_agent(user_agent) || is_http_lib_user_agent(user_agent)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- CLI detection ---

    #[test]
    fn detects_curl() {
        assert!(is_plain_text_agent("curl/8.7.1"));
    }

    #[test]
    fn detects_wget() {
        assert!(is_plain_text_agent("Wget/1.21.4"));
    }

    #[test]
    fn detects_httpie() {
        assert!(is_plain_text_agent("HTTPie/3.2.2"));
    }

    #[test]
    fn detects_xh() {
        assert!(is_plain_text_agent("xh/0.22.0"));
    }

    #[test]
    fn detects_fetch() {
        assert!(is_plain_text_agent("fetch libfetch/2.0"));
    }

    #[test]
    fn detects_powershell() {
        assert!(is_plain_text_agent("PowerShell/7.4.1"));
    }

    #[test]
    fn detects_windows_powershell() {
        assert!(is_plain_text_agent("WindowsPowerShell/5.1"));
    }

    #[test]
    fn rejects_browser() {
        assert!(!is_plain_text_agent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
        ));
    }

    #[test]
    fn rejects_empty() {
        assert!(!is_plain_text_agent(""));
    }

    #[test]
    fn cli_matching_is_case_sensitive() {
        assert!(!is_plain_text_agent("CURL/8.7.1"));
        assert!(!is_plain_text_agent("Curl/8.7.1"));
        assert!(!is_plain_text_agent("WGET/1.21.4"));
        assert!(!is_plain_text_agent("httpie/3.2.2"));
        assert!(!is_plain_text_agent("XH/0.22.0"));
        assert!(!is_plain_text_agent("powershell/7.4.1"));
    }

    // --- HTTP library detection ---

    #[test]
    fn detects_python_requests() {
        assert!(is_plain_text_agent("python-requests/2.31.0"));
    }

    #[test]
    fn detects_python_httpx() {
        assert!(is_plain_text_agent("python-httpx/0.27.0"));
    }

    #[test]
    fn detects_python_urllib3() {
        assert!(is_plain_text_agent("python-urllib3/2.2.1"));
    }

    #[test]
    fn detects_aiohttp() {
        assert!(is_plain_text_agent("aiohttp/3.9.5"));
    }

    #[test]
    fn detects_httplib2() {
        assert!(is_plain_text_agent("httplib2/0.22.0"));
    }

    #[test]
    fn detects_go_http_client() {
        assert!(is_plain_text_agent("Go-http-client/2.0"));
    }

    #[test]
    fn detects_ruby_rest_client() {
        assert!(is_plain_text_agent("rest-client/2.1.0 (linux x86_64)"));
    }

    #[test]
    fn detects_ruby_default() {
        assert!(is_plain_text_agent("Ruby"));
    }

    #[test]
    fn detects_faraday() {
        assert!(is_plain_text_agent("Faraday v2.9.0"));
    }

    #[test]
    fn detects_httparty() {
        assert!(is_plain_text_agent("HTTParty/0.21.0"));
    }

    #[test]
    fn detects_java() {
        assert!(is_plain_text_agent("Java/17.0.6"));
    }

    #[test]
    fn detects_apache_http_client() {
        assert!(is_plain_text_agent(
            "Apache-HttpClient/4.5.14 (Java/17.0.6)"
        ));
    }

    #[test]
    fn detects_okhttp() {
        assert!(is_plain_text_agent("okhttp/4.12.0"));
    }

    #[test]
    fn detects_dotnet_http_client() {
        assert!(is_plain_text_agent("HttpClient/8.0"));
    }

    #[test]
    fn detects_node_fetch() {
        assert!(is_plain_text_agent("node-fetch/3.3.2"));
    }

    #[test]
    fn detects_axios() {
        assert!(is_plain_text_agent("axios/1.7.2"));
    }

    #[test]
    fn detects_undici() {
        assert!(is_plain_text_agent("undici"));
    }

    #[test]
    fn detects_got() {
        assert!(is_plain_text_agent(
            "got (https://github.com/sindresorhus/got)"
        ));
    }

    #[test]
    fn detects_guzzle() {
        assert!(is_plain_text_agent("GuzzleHttp/7.8.1 curl/8.4.0 PHP/8.3.3"));
    }

    #[test]
    fn detects_symfony_http_client() {
        assert!(is_plain_text_agent("Symfony HttpClient/Curl"));
    }

    #[test]
    fn detects_reqwest() {
        assert!(is_plain_text_agent("reqwest/0.12.4"));
    }

    #[test]
    fn detects_hyper() {
        assert!(is_plain_text_agent("hyper/1.3.1"));
    }

    #[test]
    fn detects_ureq() {
        assert!(is_plain_text_agent("ureq/2.9.7"));
    }

    #[test]
    fn detects_dart() {
        assert!(is_plain_text_agent("Dart/3.3 (dart:io)"));
    }

    #[test]
    fn detects_libwww_perl() {
        assert!(is_plain_text_agent("libwww-perl/6.72"));
    }

    #[test]
    fn detects_lwp_simple() {
        assert!(is_plain_text_agent("LWP::Simple/6.72"));
    }

    #[test]
    fn detects_libcurl() {
        assert!(is_plain_text_agent("libcurl/8.4.0"));
    }

    #[test]
    fn detects_httr() {
        assert!(is_plain_text_agent("httr/1.4.7"));
    }

    #[test]
    fn detects_hackney() {
        assert!(is_plain_text_agent("hackney/1.20.1"));
    }

    #[test]
    fn detects_mint() {
        assert!(is_plain_text_agent("mint/1.5.2"));
    }
}
