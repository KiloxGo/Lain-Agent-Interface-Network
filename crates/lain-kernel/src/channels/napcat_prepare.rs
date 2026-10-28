//! Owned preparation jobs; the verified OneBot socket only multiplexes read receipts.
use super::{
    ChannelService, media_download, media_image,
    napcat_media::{MediaLocator, ParsedPrivate},
};
use crate::kernel::blocking;
use lain_wire::{
    MediaId,
    media::{MediaFailure, MediaState},
};
use serde_json::Value;
use std::{collections::BTreeSet, sync::Arc, time::Duration};
use tokio::{
    sync::{Semaphore, mpsc, oneshot},
    task::JoinSet,
};
use zeroize::Zeroizing;

pub(super) struct ReadImage {
    pub file: Zeroizing<String>,
    pub result: oneshot::Sender<Result<ReadSource, MediaFailure>>,
}
pub(super) enum ReadSource {
    Url(Zeroizing<String>),
    Bytes(Vec<u8>),
}
pub(super) struct Preparation {
    service: Arc<ChannelService>,
    enabled: bool,
    permits: Arc<Semaphore>,
    pub jobs: JoinSet<(MediaId, Result<media_image::PreparedImage, MediaFailure>)>,
    ids: BTreeSet<MediaId>,
    pub reads: mpsc::Receiver<ReadImage>,
    reader: mpsc::Sender<ReadImage>,
}
impl Preparation {
    pub fn new(service: Arc<ChannelService>, enabled: bool, permits: Arc<Semaphore>) -> Self {
        let (reader, reads) = mpsc::channel(8);
        Self {
            service,
            enabled,
            permits,
            jobs: JoinSet::new(),
            ids: BTreeSet::new(),
            reads,
            reader,
        }
    }
    pub async fn receive(&mut self, parsed: ParsedPrivate) -> anyhow::Result<()> {
        let incoming = self.service.receive(parsed.input).await?;
        for locator in parsed.media {
            let id = locator.id;
            let record = blocking(&self.service.kernel, move |k| k.channel_media(id)).await?;
            if record.incoming_id != incoming.id
                || record.segment_position != locator.position
                || record.visual_kind != locator.kind
                || record.state != MediaState::Pending
                || self.ids.contains(&id)
            {
                continue;
            }
            let failure = if locator.flash {
                Some(MediaFailure::FlashNotRead)
            } else if !self.enabled {
                Some(MediaFailure::ImagesDisabled)
            } else if !self.service.actor.supports_images() {
                Some(MediaFailure::ModelImagesUnavailable)
            } else {
                None
            };
            if let Some(failure) = failure {
                blocking(&self.service.kernel, move |k| {
                    k.channel_media_finish(id, Err(failure))
                })
                .await?;
                continue;
            }
            let Ok(permit) = self.permits.clone().try_acquire_owned() else {
                blocking(&self.service.kernel, move |k| {
                    k.channel_media_finish(id, Err(MediaFailure::Capacity))
                })
                .await?;
                continue;
            };
            self.ids.insert(id);
            let reader = self.reader.clone();
            self.jobs.spawn(async move {
                // The permit follows CPU decoding even if the socket/task is cancelled.
                let work = async move {
                    let bytes = fetch(locator, reader).await?;
                    tokio::task::spawn_blocking(move || {
                        let _permit = permit;
                        media_image::prepare(&bytes)
                    })
                    .await
                    .unwrap_or(Err(MediaFailure::InvalidImage))
                };
                let result = tokio::time::timeout(Duration::from_secs(19), work)
                    .await
                    .unwrap_or(Err(MediaFailure::Timeout));
                (id, result)
            });
        }
        Ok(())
    }
    pub async fn completed(
        &mut self,
        id: MediaId,
        result: Result<media_image::PreparedImage, MediaFailure>,
    ) {
        self.ids.remove(&id);
        // Expiry/revocation/withdrawal wins over late completion.
        let _ = blocking(&self.service.kernel, move |k| {
            k.channel_media_finish(id, result)
        })
        .await;
    }
    pub async fn stop(&mut self) {
        self.jobs.abort_all();
        while self.jobs.join_next().await.is_some() {}
        for id in std::mem::take(&mut self.ids) {
            let _ = blocking(&self.service.kernel, move |k| {
                k.channel_media_finish(id, Err(MediaFailure::Disconnected))
            })
            .await;
        }
    }
}
async fn fetch(
    locator: MediaLocator,
    reader: mpsc::Sender<ReadImage>,
) -> Result<Vec<u8>, MediaFailure> {
    let source = if let Some(url) = locator.url {
        ReadSource::Url(url)
    } else {
        let file = locator.file.ok_or(MediaFailure::SourceUnavailable)?;
        if !file_reference(file.as_str()) {
            return Err(MediaFailure::UnsafeLocation);
        }
        let (result, receive) = oneshot::channel();
        reader
            .try_send(ReadImage { file, result })
            .map_err(|_| MediaFailure::Capacity)?;
        tokio::time::timeout(Duration::from_secs(6), receive)
            .await
            .map_err(|_| MediaFailure::Timeout)?
            .map_err(|_| MediaFailure::Disconnected)??
    };
    match source {
        ReadSource::Url(url) => media_download::download(url.as_str()).await,
        ReadSource::Bytes(bytes) => Ok(bytes),
    }
}
pub(super) fn read_result(value: &Value) -> Result<ReadSource, MediaFailure> {
    if value.get("status").and_then(Value::as_str) != Some("ok")
        || value.get("retcode").and_then(Value::as_i64) != Some(0)
    {
        return Err(MediaFailure::SourceUnavailable);
    }
    // get_image.file is a device path; it is deliberately never opened.
    // Some NapCat configurations return bytes instead of a public URL.
    use base64::Engine;
    if let Some(data) = value
        .get("data")
        .and_then(|v| v.get("base64"))
        .and_then(Value::as_str)
    {
        if data.len() > media_image::MAX_SOURCE_BYTES.div_ceil(3) * 4 {
            return Err(MediaFailure::TooLarge);
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(data)
            .map_err(|_| MediaFailure::InvalidImage)?;
        if bytes.is_empty() || bytes.len() > media_image::MAX_SOURCE_BYTES {
            return Err(MediaFailure::TooLarge);
        }
        return Ok(ReadSource::Bytes(bytes));
    }
    let url = value
        .get("data")
        .and_then(|v| v.get("url"))
        .and_then(Value::as_str)
        .ok_or(MediaFailure::SourceUnavailable)?;
    media_download::media_url(url)?;
    Ok(ReadSource::Url(Zeroizing::new(url.into())))
}
fn file_reference(file: &str) -> bool {
    !file.is_empty()
        && file.len() <= 8192
        && !file.starts_with(['/', '.'])
        && !file.contains("..")
        && file
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"+/=_-.$|".contains(&c))
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn get_image_uses_only_bounded_bytes_or_verified_url_never_a_device_path() {
        let response = |data| json!({"status":"ok","retcode":0,"data":data});
        assert!(read_result(&response(json!({"file":"C:/private/picture.png"}))).is_err());
        assert!(read_result(&response(json!({"url":"http://127.0.0.1/private"}))).is_err());
        assert!(matches!(
            read_result(&response(
                json!({"url":"C:/private/picture.png","base64":"AQID"})
            )),
            Ok(ReadSource::Bytes(_))
        ));
        assert!(
            matches!(read_result(&response(json!({"base64":"AQID"}))),Ok(ReadSource::Bytes(b)) if b==[1,2,3])
        );
        assert!(read_result(&response(json!({"base64":"invalid!"}))).is_err());
        assert!(matches!(
            read_result(&response(json!({"url":"http://gchat.qpic.cn/picture"}))),
            Ok(ReadSource::Url(_))
        ));
        for file in [
            "C:/private/picture.png",
            "file:///private",
            "/etc/passwd",
            "../private",
            "https://qpic.cn/x",
        ] {
            assert!(!file_reference(file));
        }
        for file in ["picture.image", "opaque+base64/with=", "animation"] {
            assert!(file_reference(file));
        }
    }
}
