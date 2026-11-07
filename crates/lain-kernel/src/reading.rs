//! Exact-URL public UTF-8 text acquisition. No model, redirect, proxy or ambient auth.
use crate::{
    backend::ControlError,
    kernel::{Kernel, blocking},
    redaction::Redactor,
};
use anyhow::{Result, ensure};
use futures_util::StreamExt;
use lain_wire::{ReadingId, reading::*};
use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::Duration,
};
use url::Url;

pub(crate) fn day_bounds(now: u64, offset_minutes: i16) -> (i128, i128, i128) {
    let offset = i128::from(offset_minutes) * 60_000;
    let start = (i128::from(now) + offset).div_euclid(86_400_000) * 86_400_000 - offset;
    (
        (start - 86_400_000).max(0),
        start.max(0),
        start + 86_400_000,
    )
}

/// Strict local `YYYY-MM-DD HH:MM` to UTC milliseconds; never guesses a partial time.
pub(crate) fn local_ms(text: &str, offset_minutes: i16) -> Option<u64> {
    let (date, clock) = text.split_once(' ')?;
    if date.len() != 10 || clock.len() != 5 {
        return None;
    }
    let mut parts = date.split('-');
    let date = time::Date::from_calendar_date(
        parts.next()?.parse().ok()?,
        time::Month::try_from(parts.next()?.parse::<u8>().ok()?).ok()?,
        parts.next()?.parse().ok()?,
    )
    .ok()?;
    let (hour, minute) = clock.split_once(':')?;
    let at = date
        .with_hms(hour.parse().ok()?, minute.parse().ok()?, 0)
        .ok()?
        .assume_utc()
        .unix_timestamp()
        .checked_sub(i64::from(offset_minutes) * 60)?;
    u64::try_from(at).ok()?.checked_mul(1000)
}

pub(crate) fn validate_url(value: &str) -> Result<Url> {
    let u = Url::parse(value).map_err(|_| ControlError::InvalidInput)?;
    ensure!(
        value.len() <= 2048
            && u.as_str() == value
            && u.scheme() == "https"
            && u.host_str().is_some()
            && u.port_or_known_default() == Some(443)
            && u.username().is_empty()
            && u.password().is_none()
            && u.query().is_none()
            && u.fragment().is_none()
            && !value.chars().any(char::is_control),
        ControlError::InvalidInput
    );
    crate::profiles::check_sensitive(&value)?;
    if let Some(url::Host::Ipv4(ip)) = u.host() {
        ensure!(public_ip(ip.into()), ControlError::InvalidInput);
    }
    if let Some(url::Host::Ipv6(ip)) = u.host() {
        ensure!(public_ip(ip.into()), ControlError::InvalidInput);
    }
    Ok(u)
}
pub(crate) fn validate_scope(scope: Option<&ReadingScope>, now: u64) -> Result<()> {
    if let Some(s) = scope {
        const MONTH: u64 = 31 * 24 * 60 * 60 * 1000;
        ensure!(
            (1..=8).contains(&s.urls.len())
                && (1..=100).contains(&s.max_requests)
                && (1..=65536).contains(&s.max_bytes)
                && (1..=MONTH).contains(&s.retention_ms)
                && s.expires_at_ms > now
                && s.expires_at_ms <= now.saturating_add(MONTH)
                && s.expires_at_ms <= i64::MAX as u64,
            ControlError::InvalidInput
        );
        let mut seen = std::collections::BTreeSet::new();
        for url in &s.urls {
            validate_url(url)?;
            ensure!(seen.insert(url), ControlError::InvalidInput);
        }
        if let Some(p) = &s.processing {
            ensure!(
                (1..=100).contains(&p.max_requests) && (-720..=840).contains(&p.utc_offset_minutes),
                ControlError::InvalidInput
            );
        }
    }
    Ok(())
}

pub(crate) fn public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let [a, b, c, _] = ip.octets();
            !ip.is_private()
                && !ip.is_loopback()
                && !ip.is_link_local()
                && a != 0
                && a < 224
                && a != 127
                && !(a == 100 && (64..=127).contains(&b))
                && !(a == 192 && ((b == 0 && (c == 0 || c == 2)) || (b == 88 && c == 99)))
                && !(a == 198 && (b == 18 || b == 19 || (b == 51 && c == 100)))
                && !(a == 203 && b == 0 && c == 113)
        }
        IpAddr::V6(ip) => {
            let s = ip.segments();
            (s[0] & 0xe000) == 0x2000
                && !(s[0] == 0x2001 && s[1] < 0x0200)
                && !(s[0] == 0x2001 && s[1] == 0x0db8)
                && s[0] != 0x2002
                && !(s[0] == 0x3fff && s[1] < 0x1000)
        }
    }
}

async fn fetch(
    url: &Url,
    max: u32,
    redactor: &Redactor,
) -> std::result::Result<String, ReadingFailure> {
    let host = url.host_str().ok_or(ReadingFailure::AddressDenied)?;
    let addresses: Vec<_> = tokio::net::lookup_host((host.trim_matches(['[', ']']), 443))
        .await
        .map_err(|_| ReadingFailure::Network)?
        .take(17)
        .collect();
    if addresses.is_empty() || addresses.len() > 16 || addresses.iter().any(|a| !public_ip(a.ip()))
    {
        return Err(ReadingFailure::AddressDenied);
    }
    fetch_pinned(url, max, redactor, &addresses).await
}

async fn fetch_pinned(
    url: &Url,
    max: u32,
    redactor: &Redactor,
    addresses: &[SocketAddr],
) -> std::result::Result<String, ReadingFailure> {
    let client = reqwest::Client::builder()
        .retry(reqwest::retry::never())
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .resolve_to_addrs(
            url.host_str().ok_or(ReadingFailure::AddressDenied)?,
            addresses,
        )
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|_| ReadingFailure::Network)?;
    let response = client
        .get(url.clone())
        .header("Accept", "text/plain")
        .send()
        .await
        .map_err(|_| ReadingFailure::Network)?;
    if response.status() != reqwest::StatusCode::OK {
        return Err(ReadingFailure::HttpStatus);
    }
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    let mut parts = content_type.split(';').map(str::trim);
    if parts.next() != Some("text/plain")
        || parts.any(|p| {
            !matches!(
                p,
                "charset=utf-8" | "charset=\"utf-8\"" | "charset=us-ascii"
            )
        })
        || response.headers().contains_key("content-encoding")
    {
        return Err(ReadingFailure::UnsupportedContent);
    }
    if response
        .content_length()
        .is_some_and(|n| n > u64::from(max))
    {
        return Err(ReadingFailure::TooLarge);
    }
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| ReadingFailure::Network)?;
        if bytes.len().saturating_add(chunk.len()) > max as usize {
            return Err(ReadingFailure::TooLarge);
        }
        bytes.extend_from_slice(&chunk);
    }
    let text = String::from_utf8(bytes).map_err(|_| ReadingFailure::UnsupportedContent)?;
    if text.trim().is_empty()
        || text
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t' | '\u{000c}'))
    {
        return Err(ReadingFailure::UnsupportedContent);
    }
    if redactor.text(&text) != text {
        return Err(ReadingFailure::Sensitive);
    }
    Ok(text)
}

pub(crate) async fn run(
    kernel: Arc<Kernel>,
    id: ReadingId,
    url: String,
    redactor: Redactor,
) -> Result<()> {
    let mut changes = kernel.subscribe_reading();
    let mut shutdown = kernel.subscribe_shutdown();
    let Some(scope) = blocking(&kernel, move |k| k.claim_reading(id)).await? else {
        return Ok(());
    };
    let parsed = validate_url(&url)?;
    let operation = tokio::time::timeout(
        Duration::from_secs(20),
        fetch(&parsed, scope.max_bytes, &redactor),
    );
    tokio::pin!(operation);
    loop {
        if *shutdown.borrow() || !blocking(&kernel, move |k| k.reading_running(id)).await? {
            return Ok(());
        }
        tokio::select! {
            outcome = &mut operation => {
                let outcome = outcome.unwrap_or(Err(ReadingFailure::Timeout));
                blocking(&kernel, move |k| k.finish_reading(id, outcome)).await?;
                return Ok(());
            }
            _ = changes.recv() => {}
            _ = shutdown.changed() => {}
        }
    }
}

/// One finite timer for the next persisted retention deadline; no model polling.
pub async fn maintain(kernel: Arc<Kernel>) {
    let mut changes = kernel.subscribe_reading();
    let mut shutdown = kernel.subscribe_shutdown();
    loop {
        if *shutdown.borrow() {
            return;
        }
        let deadline = match blocking(&kernel, Kernel::reading_deadline).await {
            Ok(deadline) => deadline,
            Err(_) => {
                tracing::error!("reading retention maintenance stopped; storage unavailable");
                return;
            }
        };
        let timer = async {
            if let Some(deadline) = deadline {
                let now = crate::domain::now_ms().unwrap_or(0);
                tokio::time::sleep(Duration::from_millis(deadline.saturating_sub(now))).await;
            } else {
                std::future::pending::<()>().await;
            }
        };
        tokio::select! { _ = timer => {}, _ = changes.recv() => {}, _ = shutdown.changed() => {} }
    }
}

#[cfg(test)]
mod experience_tests;
#[cfg(test)]
mod tests;
