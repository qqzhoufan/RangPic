//! Outbound URL checks for user-supplied and seed image fetches.
//!
//! Blocks non-HTTP schemes, credentials in the URL, and destinations that
//! resolve to loopback, link-local, private, or documentation ranges.
//! Redirects are followed manually so each hop is checked again.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use url::{Host, Url};

use crate::error::{AppError, AppResult};

const MAX_REDIRECTS: usize = 5;

pub async fn guarded_get(client: &reqwest::Client, raw: &str) -> AppResult<reqwest::Response> {
    let mut current = validate_fetch_url(raw).await?;
    for _ in 0..MAX_REDIRECTS {
        let resp = client
            .get(current.clone())
            .send()
            .await
            .map_err(|e| AppError::BadRequest(format!("无法获取远程图片: {e}")))?;
        if !resp.status().is_redirection() {
            return Ok(resp);
        }
        let Some(loc) = resp
            .headers()
            .get(http::header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string)
        else {
            return Ok(resp);
        };
        let next = current
            .join(&loc)
            .map_err(|_| AppError::BadRequest("重定向地址无效".into()))?;
        current = validate_fetch_url(next.as_str()).await?;
    }
    Err(AppError::BadRequest("重定向次数过多".into()))
}

pub async fn validate_fetch_url(raw: &str) -> AppResult<Url> {
    let url = parse_http_url(raw)?;
    ensure_public_host(&url).await?;
    Ok(url)
}

fn parse_http_url(raw: &str) -> AppResult<Url> {
    let url = Url::parse(raw).map_err(|_| AppError::BadRequest("URL 无效".into()))?;
    match url.scheme() {
        "http" | "https" => {}
        _ => return Err(AppError::BadRequest("仅支持 http/https 地址".into())),
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(AppError::BadRequest("不允许在地址中携带账号信息".into()));
    }
    if url.host().is_none() {
        return Err(AppError::BadRequest("URL 无效".into()));
    }
    Ok(url)
}

async fn ensure_public_host(url: &Url) -> AppResult<()> {
    match url.host() {
        Some(Host::Ipv4(ip)) => {
            if is_blocked_ip(IpAddr::V4(ip)) {
                return Err(blocked());
            }
        }
        Some(Host::Ipv6(ip)) => {
            if is_blocked_ip(IpAddr::V6(ip)) {
                return Err(blocked());
            }
        }
        Some(Host::Domain(name)) => {
            if is_blocked_host_name(name) {
                return Err(blocked());
            }
            let port = url.port_or_known_default().unwrap_or(80);
            let addrs = tokio::net::lookup_host((name, port))
                .await
                .map_err(|_| AppError::BadRequest("无法解析图片地址".into()))?;
            let mut saw = false;
            for addr in addrs {
                saw = true;
                if is_blocked_ip(addr.ip()) {
                    return Err(blocked());
                }
            }
            if !saw {
                return Err(blocked());
            }
        }
        None => return Err(AppError::BadRequest("URL 无效".into())),
    }
    Ok(())
}

fn blocked() -> AppError {
    AppError::BadRequest("不允许访问该地址".into())
}

pub fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_blocked_v4(v4),
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped().or_else(|| v6.to_ipv4()) {
                return is_blocked_v4(v4);
            }
            is_blocked_v6(v6)
        }
    }
}

fn is_blocked_v4(v4: Ipv4Addr) -> bool {
    let o = v4.octets();
    v4.is_private()
        || v4.is_loopback()
        || v4.is_link_local()
        || v4.is_broadcast()
        || v4.is_unspecified()
        || v4.is_multicast()
        || v4.is_documentation()
        || o[0] == 0
        || o[0] >= 240
        || (o[0] == 100 && (o[1] & 0b1100_0000) == 64)
        || (o[0] == 198 && (o[1] == 18 || o[1] == 19))
}

fn is_blocked_v6(v6: Ipv6Addr) -> bool {
    let s = v6.segments();
    v6.is_loopback()
        || v6.is_unspecified()
        || v6.is_multicast()
        || v6.is_unicast_link_local()
        || (s[0] & 0xfe00) == 0xfc00
        || (s[0] & 0xffc0) == 0xfec0
        || (s[0] == 0x2001 && s[1] == 0x0db8)
}

fn is_blocked_host_name(host: &str) -> bool {
    let h = host.trim_end_matches('.').to_ascii_lowercase();
    h == "localhost"
        || h == "localhost.localdomain"
        || h.ends_with(".localhost")
        || h == "metadata.google.internal"
        || h.ends_with(".metadata.google.internal")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_non_public_ranges() {
        let blocked = [
            "127.0.0.1",
            "10.1.2.3",
            "172.16.0.1",
            "192.168.1.1",
            "169.254.169.254",
            "0.0.0.0",
            "100.64.0.1",
            "192.0.2.1",
            "198.18.0.1",
            "224.0.0.1",
            "255.255.255.255",
            "::1",
            "fc00::1",
            "fe80::1",
            "2001:db8::1",
            "::ffff:127.0.0.1",
            "::ffff:10.0.0.1",
        ];
        for s in blocked {
            let ip: IpAddr = s.parse().unwrap();
            assert!(is_blocked_ip(ip), "{s} should be blocked");
        }
    }

    #[test]
    fn allows_public_addresses() {
        let allowed = ["1.1.1.1", "8.8.8.8", "172.15.0.1", "2606:4700:4700::1111"];
        for s in allowed {
            let ip: IpAddr = s.parse().unwrap();
            assert!(!is_blocked_ip(ip), "{s} should be allowed");
        }
    }

    #[tokio::test]
    async fn rejects_unsafe_urls() {
        for raw in [
            "file:///etc/passwd",
            "ftp://1.1.1.1/a",
            "http://user:pw@1.1.1.1/a",
            "http://127.0.0.1/a",
            "http://[::1]/a",
            "http://169.254.169.254/latest/meta-data",
            "http://localhost/a",
            "http://metadata.google.internal/",
            "http://2130706433/",
        ] {
            let err = validate_fetch_url(raw).await.expect_err(raw);
            assert!(matches!(err, AppError::BadRequest(_)), "{raw}: {err}");
        }
    }

    #[tokio::test]
    async fn allows_public_ip_literal() {
        let url = validate_fetch_url("https://1.1.1.1/img.png").await.unwrap();
        assert_eq!(url.host_str(), Some("1.1.1.1"));
    }
}
