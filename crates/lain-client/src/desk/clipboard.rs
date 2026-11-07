//! Local-only clipboard delivery. No terminal escape sequences or cloud calls.
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
    mpsc,
};
pub(super) struct Ticket {
    pub epoch: u64,
    pub source_epoch: Arc<AtomicU64>,
}
impl Ticket {
    pub fn valid(&self) -> bool {
        self.source_epoch.load(Ordering::SeqCst) == self.epoch
    }
}
struct Job {
    text: String,
    ticket: Ticket,
}
pub(super) struct Worker {
    send: mpsc::SyncSender<Job>,
    receive: mpsc::Receiver<(u64, Result<(), String>)>,
    pub busy: bool,
}
impl Worker {
    pub fn new() -> Result<Self, String> {
        let mut native: Option<arboard::Clipboard> = None;
        Self::with_backend(move |text, ticket| {
            if native.is_none() {
                native =
                    Some(arboard::Clipboard::new().map_err(|_| "系统剪贴板暂不可用".to_string())?);
            }
            if !ticket.valid() {
                return Err("来源已更新，复制已取消".into());
            }
            native
                .as_mut()
                .unwrap()
                .set_text(text.to_owned())
                .map_err(|_| "剪贴板写入失败，请重试".to_string())?;
            #[cfg(test)]
            if let Some(path) = TEST_RECEIPTS.get() {
                use std::io::Write;
                let actual = native
                    .as_mut()
                    .unwrap()
                    .get_text()
                    .map_err(|_| "测试剪贴板回读失败".to_string())?;
                let mut file = std::fs::OpenOptions::new()
                    .append(true)
                    .open(path)
                    .map_err(|_| "测试回执不可写".to_string())?;
                writeln!(file, "{}", serde_json::to_string(&actual).unwrap())
                    .map_err(|_| "测试回执写入失败".to_string())?;
            }
            Ok(())
        })
    }
    fn with_backend(
        mut write: impl FnMut(&str, &Ticket) -> Result<(), String> + Send + 'static,
    ) -> Result<Self, String> {
        let (send, jobs) = mpsc::sync_channel::<Job>(1);
        let (results, receive) = mpsc::channel();
        std::thread::Builder::new()
            .name("lain-local-clipboard".into())
            .spawn(move || {
                while let Ok(job) = jobs.recv() {
                    let result = if job.ticket.valid() {
                        write(&job.text, &job.ticket)
                    } else {
                        Err("来源已更新，复制已取消".into())
                    };
                    if results.send((job.ticket.epoch, result)).is_err() {
                        break;
                    }
                }
            })
            .map_err(|_| "无法启动本机剪贴板服务".to_string())?;
        Ok(Self {
            send,
            receive,
            busy: false,
        })
    }
    pub fn submit(&mut self, text: String, ticket: Ticket) -> Result<(), String> {
        if self.busy {
            return Err("上一条复制尚未完成".into());
        }
        if text.is_empty() || text.len() > 131072 {
            return Err("没有可复制的正文，或文字超过本地上限".into());
        }
        self.send
            .try_send(Job { text, ticket })
            .map_err(|_| "剪贴板繁忙，请重试".to_string())?;
        self.busy = true;
        Ok(())
    }
    pub fn poll(&mut self) -> Option<(u64, Result<(), String>)> {
        let result = self.receive.try_recv().ok()?;
        self.busy = false;
        Some(result)
    }
}
#[cfg(test)]
static TEST_RECEIPTS: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
#[cfg(test)]
mod tests {
    use super::*;
    // winuser.h WINSTA_ALL_ACCESS; windows-sys does not export this composite.
    #[cfg(windows)]
    const WINSTA_ALL_ACCESS: u32 = 0x037f;
    #[cfg(windows)]
    fn private_station() {
        use windows_sys::Win32::System::StationsAndDesktops::*;
        let name = format!("LainClipboardFixture-{}\0", uuid::Uuid::new_v4())
            .encode_utf16()
            .collect::<Vec<_>>();
        let station =
            unsafe { CreateWindowStationW(name.as_ptr(), 0, WINSTA_ALL_ACCESS, std::ptr::null()) };
        assert!(!station.is_null());
        assert_ne!(unsafe { SetProcessWindowStation(station) }, 0);
        let desktop_name = "Default\0".encode_utf16().collect::<Vec<_>>();
        let desktop = unsafe {
            CreateDesktopW(
                desktop_name.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                0,
                0x01ff,
                std::ptr::null(),
            )
        };
        assert!(!desktop.is_null());
        assert_ne!(unsafe { SetThreadDesktop(desktop) }, 0);
    }
    #[cfg(windows)]
    #[test]
    #[ignore = "native ConPTY test harness only; private clipboard station, no production override"]
    fn native_terminal_private_clipboard() {
        assert_eq!(
            std::env::var("LAIN_TERMINAL_COPY_FIXTURE").as_deref(),
            Ok("isolated-fixture")
        );
        let path =
            std::path::PathBuf::from(std::env::var_os("LAIN_TERMINAL_COPY_RECEIPT").unwrap());
        let parent = path.parent().unwrap().canonicalize().unwrap();
        let temp = std::env::temp_dir().canonicalize().unwrap();
        assert!(
            parent.starts_with(&temp)
                && parent
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("lain-ui-terminal-")
        );
        assert_eq!(path.file_name().unwrap(), "clipboard-receipt.jsonl");
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        private_station();
        TEST_RECEIPTS.set(path).unwrap();
        if let Ok(text) = std::env::var("LAIN_TERMINAL_PASTE_FIXTURE") {
            assert!(text.len() <= 8192);
            arboard::Clipboard::new().unwrap().set_text(text).unwrap();
        }
        let client = crate::api::Client::new(
            &std::env::var("LAIN_URL").unwrap(),
            &std::env::var("LAIN_API_TOKEN").unwrap(),
        )
        .unwrap();
        let conversation = std::env::var("LAIN_TERMINAL_COPY_CONVERSATION")
            .unwrap()
            .parse()
            .unwrap();
        let portrait = std::env::var_os("LAIN_PORTRAIT_FILE").map(std::path::PathBuf::from);
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(super::super::run(client, Some(conversation), portrait))
            .unwrap();
    }
    #[cfg(windows)]
    #[test]
    #[ignore = "explicit native Windows private-window-station clipboard acceptance"]
    fn native_clipboard_in_private_station() {
        if std::env::var_os("LAIN_ISOLATED_CLIPBOARD_CHILD").is_none() {
            let result = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "desk::clipboard::tests::native_clipboard_in_private_station",
                    "--ignored",
                    "--nocapture",
                ])
                .env("LAIN_ISOLATED_CLIPBOARD_CHILD", "1")
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "isolated clipboard acceptance failed: {}",
                String::from_utf8_lossy(&result.stderr)
            );
            assert!(
                String::from_utf8_lossy(&result.stdout)
                    .contains("private clipboard roundtrip verified")
            );
            return;
        }
        use windows_sys::Win32::System::StationsAndDesktops::*;
        let name = format!("LainClipboardFixture-{}\0", uuid::Uuid::new_v4())
            .encode_utf16()
            .collect::<Vec<_>>();
        // Only this dedicated child process changes its window station. The
        // interactive station and its clipboard are never opened or read.
        let station =
            unsafe { CreateWindowStationW(name.as_ptr(), 0, WINSTA_ALL_ACCESS, std::ptr::null()) };
        assert!(!station.is_null(), "cannot create private station");
        assert_ne!(unsafe { SetProcessWindowStation(station) }, 0);
        let desktop_name = "Default\0".encode_utf16().collect::<Vec<_>>();
        let desktop = unsafe {
            CreateDesktopW(
                desktop_name.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                0,
                0x01ff,
                std::ptr::null(),
            )
        };
        assert!(!desktop.is_null(), "cannot create private desktop");
        assert_ne!(unsafe { SetThreadDesktop(desktop) }, 0);
        let epoch = Arc::new(AtomicU64::new(1));
        let mut worker = Worker::new().unwrap();
        let payload = "isolated fixture: 中文 👩‍💻\n\t│ | preserved\n";
        worker
            .submit(
                payload.into(),
                Ticket {
                    epoch: 1,
                    source_epoch: epoch,
                },
            )
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            if let Some((_, result)) = worker.poll() {
                result.unwrap();
                break;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let mut reader = arboard::Clipboard::new().unwrap();
        assert_eq!(reader.get_text().unwrap(), payload);
        println!("private clipboard roundtrip verified");
        // Process exit releases this noninteractive station/desktop and the
        // worker. Do not attach any thread back to the user's interactive station.
    }
    #[test]
    fn pending_clipboard_setup_cannot_write_after_source_retirement() {
        let epoch = Arc::new(AtomicU64::new(1));
        let (ready, entered) = mpsc::channel();
        let (release, wait) = mpsc::channel();
        let mut worker = Worker::with_backend(move |_, ticket| {
            ready.send(()).unwrap();
            wait.recv().unwrap();
            if !ticket.valid() {
                Err("cancelled".into())
            } else {
                panic!("retired text must not be written")
            }
        })
        .unwrap();
        worker
            .submit(
                "fixture-only-copy".into(),
                Ticket {
                    epoch: 1,
                    source_epoch: epoch.clone(),
                },
            )
            .unwrap();
        entered
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        epoch.store(2, Ordering::SeqCst);
        release.send(()).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            if let Some((_, result)) = worker.poll() {
                assert!(result.is_err());
                break;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
    }
}
