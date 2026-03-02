const CLI_PREFIXES: &[&str] = &[
    "curl/",
    "Wget/",
    "HTTPie/",
    "xh/",
    "fetch",
    "PowerShell/",
    "WindowsPowerShell/",
];

pub fn is_cli_user_agent(user_agent: &str) -> bool {
    CLI_PREFIXES
        .iter()
        .any(|prefix| user_agent.starts_with(prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_curl() {
        assert!(is_cli_user_agent("curl/8.7.1"));
    }

    #[test]
    fn detects_wget() {
        assert!(is_cli_user_agent("Wget/1.21.4"));
    }

    #[test]
    fn detects_httpie() {
        assert!(is_cli_user_agent("HTTPie/3.2.2"));
    }

    #[test]
    fn detects_xh() {
        assert!(is_cli_user_agent("xh/0.22.0"));
    }

    #[test]
    fn detects_fetch() {
        assert!(is_cli_user_agent("fetch libfetch/2.0"));
    }

    #[test]
    fn detects_powershell() {
        assert!(is_cli_user_agent("PowerShell/7.4.1"));
    }

    #[test]
    fn detects_windows_powershell() {
        assert!(is_cli_user_agent("WindowsPowerShell/5.1"));
    }

    #[test]
    fn rejects_browser() {
        assert!(!is_cli_user_agent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
        ));
    }

    #[test]
    fn rejects_empty() {
        assert!(!is_cli_user_agent(""));
    }
}
