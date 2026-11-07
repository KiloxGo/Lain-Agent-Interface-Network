use super::*;
use reqwest::{Method, Url};
use serde::de::DeserializeOwned;

#[derive(Debug, thiserror::Error)]
pub(super) enum CloudError {
    #[error("device authority denied")]
    Denied,
    #[error("cloud request conflicts with current state")]
    Conflict,
    #[error("cloud transport unavailable")]
    Transport,
    #[error("invalid cloud response")]
    Protocol,
}
pub(super) struct Cloud {
    http: reqwest::Client,
    base: Url,
    key: zeroize::Zeroizing<String>,
}
impl Cloud {
    pub fn new(endpoint: &str, key: &str, id: DeviceId) -> Result<Self> {
        let mut url =
            Url::parse(endpoint).map_err(|_| anyhow::anyhow!("invalid device endpoint"))?;
        let loopback = url.host_str().is_some_and(|s| {
            s == "localhost"
                || s.trim_matches(['[', ']'])
                    .parse::<std::net::IpAddr>()
                    .is_ok_and(|ip| ip.is_loopback())
        });
        ensure!(
            (url.scheme() == "https" || (url.scheme() == "http" && loopback))
                && url.host_str().is_some()
                && url.username().is_empty()
                && url.password().is_none()
                && url.query().is_none()
                && url.fragment().is_none(),
            "device endpoint requires HTTPS, or loopback HTTP, without credentials/query/fragment"
        );
        let path = format!(
            "{}/v1/device-agent/{id}/commands/",
            url.path().trim_end_matches('/')
        );
        url.set_path(&path);
        let http = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(28))
            .build()?;
        Ok(Self {
            http,
            base: url,
            key: zeroize::Zeroizing::new(key.into()),
        })
    }
    async fn call<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<&impl Serialize>,
    ) -> Result<T> {
        let mut request = self
            .http
            .request(method, self.base.join(path)?)
            .bearer_auth(self.key.as_str());
        if let Some(b) = body {
            request = request.json(b);
        }
        let mut r = request.send().await.map_err(|_| CloudError::Transport)?;
        let status = r.status();
        if !status.is_success() {
            return Err(match status.as_u16() {
                401 | 403 => CloudError::Denied,
                400..=499 => CloudError::Conflict,
                _ => CloudError::Transport,
            }
            .into());
        }
        let mut bytes = zeroize::Zeroizing::new(Vec::new());
        while let Some(chunk) = r.chunk().await.map_err(|_| CloudError::Transport)? {
            // A command includes both its current evidence and immutable last
            // event. Each contains at most 256 small exact decision reports.
            ensure!(bytes.len() + chunk.len() <= 262144, CloudError::Protocol);
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&bytes).map_err(|_| CloudError::Protocol.into())
    }
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.call(Method::GET, path, None::<&bool>).await
    }
    pub async fn post<T: DeserializeOwned>(&self, path: &str, body: &impl Serialize) -> Result<T> {
        self.call(Method::POST, path, Some(body)).await
    }
}
