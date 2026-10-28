//! Exact QQ media CDN reads with pinned public addresses; never read a file path.
use super::media_image::MAX_SOURCE_BYTES;
use futures_util::StreamExt;
use lain_wire::media::MediaFailure;
use std::{net::SocketAddr, time::Duration};
use url::Url;

pub(super) fn media_url(raw: &str) -> Result<Url, MediaFailure> {
    if raw.len() > 8192 {
        return Err(MediaFailure::UnsafeLocation);
    }
    let mut url = Url::parse(raw).map_err(|_| MediaFailure::UnsafeLocation)?;
    let host = url.host_str().ok_or(MediaFailure::UnsafeLocation)?;
    let host_allowed = matches!(host, "multimedia.nt.qq.com.cn" | "gxh.vip.qq.com")
        || ["qpic.cn", "qlogo.cn"]
            .iter()
            .any(|suffix| host == *suffix || host.ends_with(&format!(".{suffix}")));
    if !host_allowed
        || !matches!(url.scheme(), "https" | "http")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || url.port().is_some_and(|p| p != 443)
    {
        return Err(MediaFailure::UnsafeLocation);
    }
    url.set_scheme("https")
        .map_err(|_| MediaFailure::UnsafeLocation)?;
    Ok(url)
}
async fn fetch_pinned(url: Url, addresses: &[SocketAddr]) -> Result<Vec<u8>, MediaFailure> {
    let client = reqwest::Client::builder()
        .retry(reqwest::retry::never())
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .resolve_to_addrs(
            url.host_str().ok_or(MediaFailure::UnsafeLocation)?,
            addresses,
        )
        .connect_timeout(Duration::from_secs(4))
        .timeout(Duration::from_secs(12))
        .build()
        .map_err(|_| MediaFailure::SourceUnavailable)?;
    let response = client
        .get(url)
        .header("Accept-Encoding", "identity")
        .send()
        .await
        .map_err(|_| MediaFailure::SourceUnavailable)?;
    if response.status() != reqwest::StatusCode::OK {
        return Err(MediaFailure::HttpStatus {
            code: response.status().as_u16(),
        });
    }
    if response.headers().contains_key("content-encoding") {
        return Err(MediaFailure::UnsupportedFormat);
    }
    if response
        .content_length()
        .is_some_and(|n| n > MAX_SOURCE_BYTES as u64)
    {
        return Err(MediaFailure::TooLarge);
    }
    let mut stream = response.bytes_stream();
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| MediaFailure::SourceUnavailable)?;
        if bytes.len().saturating_add(chunk.len()) > MAX_SOURCE_BYTES {
            return Err(MediaFailure::TooLarge);
        }
        bytes.extend_from_slice(&chunk);
    }
    if bytes.is_empty() {
        return Err(MediaFailure::InvalidImage);
    }
    Ok(bytes)
}
pub(super) async fn download(raw: &str) -> Result<Vec<u8>, MediaFailure> {
    tokio::time::timeout(Duration::from_secs(15), async {
        let url = media_url(raw)?;
        let host = url.host_str().ok_or(MediaFailure::UnsafeLocation)?;
        let addresses: Vec<_> = tokio::net::lookup_host((host, 443))
            .await
            .map_err(|_| MediaFailure::SourceUnavailable)?
            .take(17)
            .collect();
        if addresses.is_empty()
            || addresses.len() > 16
            || addresses.iter().any(|a| !crate::reading::public_ip(a.ip()))
        {
            return Err(MediaFailure::UnsafeLocation);
        }
        fetch_pinned(url, &addresses).await
    })
    .await
    .map_err(|_| MediaFailure::Timeout)?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn pinned_http_contract_bounds_body_and_rejects_redirects_and_encoded_bodies() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        for (response,expected) in [
            ("HTTP/1.1 200 OK\r\nContent-Length: 3\r\nConnection: close\r\n\r\nabc".to_string(),Ok(b"abc".to_vec())),
            ("HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1/private\r\nContent-Length: 0\r\n\r\n".to_string(),Err(MediaFailure::HttpStatus{code:302})),
            (format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n",MAX_SOURCE_BYTES+1),Err(MediaFailure::TooLarge)),
            ("HTTP/1.1 200 OK\r\nContent-Encoding: gzip\r\nContent-Length: 0\r\n\r\n".to_string(),Err(MediaFailure::UnsupportedFormat)),
        ] {
            let listener=tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address=listener.local_addr().unwrap();
            let server=tokio::spawn(async move {let (mut socket,_)=listener.accept().await.unwrap();let mut request=[0;4096];let n=socket.read(&mut request).await.unwrap();
                let request=std::str::from_utf8(&request[..n]).unwrap();assert!(request.contains("GET /fixture"));
                socket.write_all(response.as_bytes()).await.unwrap();});
            // Test the transport helper only: production download validates HTTPS/public DNS first.
            let url=Url::parse(&format!("http://qpic.cn:{}/fixture",address.port())).unwrap();
            assert_eq!(fetch_pinned(url,&[address]).await,expected);server.await.unwrap();
        }
    }
    #[test]
    fn only_exact_qq_cdns_are_upgraded_and_transport_tokens_are_not_domain_data() {
        for raw in [
            "https://multimedia.nt.qq.com.cn/download?rkey=transport-only",
            "http://gchat.qpic.cn/picture",
            "https://gxh.vip.qq.com/club/item/sticker.gif",
        ] {
            assert_eq!(media_url(raw).unwrap().scheme(), "https");
        }
        for raw in [
            "file:///C:/private/image.png",
            "https://127.0.0.1/image",
            "https://qpic.cn.attacker.invalid/x",
            "https://attackerqpic.cn/x",
            "https://qpic.cn:8443/x",
            "https://user:pass@qpic.cn/x",
            "https://qpic.cn/x#fragment",
        ] {
            assert!(media_url(raw).is_err());
        }
        assert!(!crate::reading::public_ip("127.0.0.1".parse().unwrap()));
        assert!(!crate::reading::public_ip(
            "169.254.169.254".parse().unwrap()
        ));
        assert!(!crate::reading::public_ip(
            "::ffff:127.0.0.1".parse().unwrap()
        ));
    }
}
