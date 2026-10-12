//! FPS ölçümü (RivaTuner yokken): Intel PresentMon. Oyuna hiçbir şey enjekte etmez; Windows'un kendi olay
//! kayıtlarından (ETW) her karenin süresini okur. CapFrameX gibi araçların kullandığı yöntem.
//!
//! - PresentMon (MIT lisanslı, ~1 MB) ilk kurulumda `Araclar\PresentMon` klasörüne indirilir; dosya SHA-256 ile
//!   doğrulanır.
//! - Windows bu kayıtları yönetici olmayanlara yalnız "Performance Log Users" (S-1-5-32-559) grubundaysa açar.
//!   "FPS ölçümünü aç" bir kez yönetici izni ister, kullanıcıyı bu gruba ekler; Windows oturumu bir kez kapatılıp
//!   açılınca (ya da yeniden başlatınca) geçerli olur.
//! - Panel açıkken oyunun süreci için çalışır, panel kapanınca durdurulur.

use crate::util::hide_console;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const VERSION: &str = "2.4.1";
const URL: &str =
    "https://github.com/GameTechDev/PresentMon/releases/download/v2.4.1/PresentMon-2.4.1-x64.exe";
const SHA256: &str = "d74183e7ae630f72cd3690be0373ecbfdc6cbb86578148aab8fa2a7166068f34";
/// Performance Log Users grubunun herkesçe bilinen kimliği (Windows dili fark etmez).
pub const GROUP_SID: &str = "S-1-5-32-559";
const SESSION: &str = "GMIDPanel";

#[derive(Serialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum State {
    /// PresentMon indirilmemiş.
    #[default]
    Missing,
    /// İzin yok (gruba eklenmemiş ya da oturum yenilenmemiş).
    Permission,
    Ready,
    Running,
    Error,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub state: State,
    /// Gruba eklenme istendi ama oturum henüz yenilenmedi.
    pub relogin: bool,
    pub message: Option<String>,
}

/// CSV başlığından kare süresi sütunu; satırlardan milisaniye.
#[derive(Default)]
pub struct Parser {
    col: Option<usize>,
}

impl Parser {
    pub fn feed(&mut self, line: &str) -> Option<f64> {
        let cells: Vec<&str> = line.trim().split(',').collect();
        if cells
            .first()
            .is_some_and(|c| c.eq_ignore_ascii_case("Application"))
        {
            self.col = ["MsBetweenPresents", "msBetweenPresents", "FrameTime"]
                .iter()
                .find_map(|name| cells.iter().position(|c| c.trim() == *name));
            return None;
        }
        let v: f64 = cells.get(self.col?)?.trim().parse().ok()?;
        (v > 0.0 && v < 5000.0).then_some(v)
    }
}

/// Son bir saniyedeki karelerden FPS.
pub fn fps_of(frames: &VecDeque<(Instant, f64)>, now: Instant) -> Option<f32> {
    let recent: Vec<f64> = frames
        .iter()
        .filter(|(t, _)| now.duration_since(*t) <= Duration::from_millis(1200))
        .map(|(_, ms)| *ms)
        .collect();
    if recent.len() < 2 {
        return None;
    }
    let avg = recent.iter().sum::<f64>() / recent.len() as f64;
    Some((1000.0 / avg) as f32)
}

#[derive(Default)]
pub struct PresentMon {
    run: Mutex<Option<(u32, Child)>>,
    frames: Arc<Mutex<VecDeque<(Instant, f64)>>>,
    status: Arc<Mutex<Status>>,
}

pub fn exe(root: &Path) -> PathBuf {
    root.join("Araclar")
        .join("PresentMon")
        .join("PresentMon.exe")
}

fn installed(root: &Path) -> bool {
    exe(root).is_file()
}

/// Bu oturumun belirteci gruba dahil mi (`whoami /groups`).
fn in_group() -> bool {
    let mut cmd = Command::new("whoami");
    cmd.args(["/groups", "/fo", "csv", "/nh"]);
    hide_console(&mut cmd);
    cmd.output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains(GROUP_SID))
        .unwrap_or(false)
}

/// Bu kullanıcının kimliği (`whoami /user`): "S-1-5-21-…".
fn user_sid() -> Option<String> {
    let mut cmd = Command::new("whoami");
    cmd.args(["/user", "/fo", "csv", "/nh"]);
    hide_console(&mut cmd);
    let out = String::from_utf8_lossy(&cmd.output().ok()?.stdout).into_owned();
    out.split(',')
        .map(|c| c.trim().trim_matches('"').to_string())
        .find(|c| c.starts_with("S-1-5-"))
}

impl PresentMon {
    pub fn status(&self, root: &Path, requested: bool) -> Status {
        let mut s = self
            .status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        if !installed(root) {
            s.state = State::Missing;
        } else if s.state == State::Missing {
            s.state = if in_group() {
                State::Ready
            } else {
                State::Permission
            };
        }
        s.relogin = requested && s.state == State::Permission;
        s
    }

    /// İlk kurulum: indir (gerekirse), izni ayarla (gerekirse, bir kez yönetici izni).
    pub fn setup(&self, root: &Path) -> Result<Status, String> {
        let path = exe(root);
        if !path.is_file() {
            let dir = path.parent().ok_or("klasör yok")?;
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            let tmp = path.with_extension("part");
            crate::downloader::tools::download(URL, &tmp, |_, _| {})?;
            let bytes = std::fs::read(&tmp).map_err(|e| e.to_string())?;
            let got: String = Sha256::digest(&bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            if got != SHA256 {
                let _ = std::fs::remove_file(&tmp);
                return Err("PresentMon doğrulanamadı (SHA-256 tutmuyor)".into());
            }
            std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
            crate::log!("PresentMon {VERSION} kuruldu");
        }
        let mut st = self.status.lock().unwrap_or_else(|e| e.into_inner());
        if in_group() {
            *st = Status {
                state: State::Ready,
                ..Default::default()
            };
            return Ok(st.clone());
        }
        let sid = user_sid().ok_or("Kullanıcı kimliği okunamadı")?;
        elevate_add_to_group(&sid)?;
        *st = Status {
            state: State::Permission,
            relogin: true,
            message: Some("Windows oturumunu bir kez kapatıp aç (ya da bilgisayarı yeniden başlat), sonra FPS görünür.".into()),
        };
        Ok(st.clone())
    }

    /// Panel açıkken: oyunun süreci için ölçüm (zaten o süreç için çalışıyorsa dokunmaz). FPS'i döner.
    pub fn sample(&self, root: &Path, pid: Option<u32>) -> Option<f32> {
        let pid = pid?;
        if !installed(root) {
            return None;
        }
        let mut run = self.run.lock().unwrap_or_else(|e| e.into_inner());
        let alive = match run.as_mut() {
            Some((p, child)) if *p == pid => child.try_wait().ok().flatten().is_none(),
            _ => false,
        };
        if !alive {
            if let Some((_, mut child)) = run.take() {
                let _ = child.kill();
            }
            let st = self
                .status
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .state
                .clone();
            // İzin yoksa her saniye yeniden denenmez (panel yeniden açılınca denenir).
            if st == State::Permission || st == State::Error {
                return None;
            }
            self.frames
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clear();
            match self.spawn(root, pid) {
                Ok(child) => *run = Some((pid, child)),
                Err(e) => {
                    let mut s = self.status.lock().unwrap_or_else(|e| e.into_inner());
                    s.state = State::Error;
                    s.message = Some(e);
                    return None;
                }
            }
        }
        drop(run);
        fps_of(
            &self.frames.lock().unwrap_or_else(|e| e.into_inner()),
            Instant::now(),
        )
    }

    fn spawn(&self, root: &Path, pid: u32) -> Result<Child, String> {
        let mut cmd = Command::new(exe(root));
        cmd.args(["--process_id", &pid.to_string()])
            .args(["--output_stdout", "--no_console_stats"])
            .args(["--session_name", SESSION, "--stop_existing_session"])
            .args([
                "--terminate_on_proc_exit",
                "--no_track_display",
                "--no_track_gpu",
                "--no_track_input",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        hide_console(&mut cmd);
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("PresentMon açılamadı: {e}"))?;
        let frames = self.frames.clone();
        let status = self.status.clone();
        let out = child.stdout.take();
        let err = child.stderr.take();
        {
            let mut s = status.lock().unwrap_or_else(|e| e.into_inner());
            s.state = State::Running;
            s.message = None;
        }
        std::thread::spawn(move || {
            let mut parser = Parser::default();
            if let Some(out) = out {
                for line in BufReader::new(out).lines().map_while(Result::ok) {
                    if let Some(ms) = parser.feed(&line) {
                        let mut f = frames.lock().unwrap_or_else(|e| e.into_inner());
                        let now = Instant::now();
                        f.push_back((now, ms));
                        while f
                            .front()
                            .is_some_and(|(t, _)| now.duration_since(*t) > Duration::from_secs(3))
                        {
                            f.pop_front();
                        }
                    }
                }
            }
            // Çıktı bitti: izin hatası mı?
            let mut text = String::new();
            if let Some(mut e) = err {
                let _ = e.read_to_string(&mut text);
            }
            let lower = text.to_lowercase();
            let mut s = status.lock().unwrap_or_else(|e| e.into_inner());
            if lower.contains("denied") || lower.contains("performance log users") {
                s.state = State::Permission;
                s.message = Some("FPS için izin gerekiyor: Oyun Modu → FPS ölçümünü aç".into());
            } else if s.state == State::Running {
                // Hata verdiyse panel yeniden açılana dek tekrar denenmez (her saniye başlatılmasın).
                match text.lines().rev().find(|l| l.contains("error")) {
                    Some(l) => {
                        s.state = State::Error;
                        s.message = Some(l.trim().to_string());
                        crate::log!("PresentMon: {}", l.trim());
                    }
                    None => s.state = State::Ready,
                }
            }
        });
        Ok(child)
    }

    /// Panel kapanınca. Süreç kapatılsa da Windows'taki izleme oturumu açık kalır; o da kapatılır.
    pub fn stop(&self, root: &Path) {
        if let Some((_, mut child)) = self.run.lock().unwrap_or_else(|e| e.into_inner()).take() {
            let _ = child.kill();
            let _ = child.wait();
            let mut cmd = Command::new(exe(root));
            cmd.args(["--terminate_existing_session", "--session_name", SESSION])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            hide_console(&mut cmd);
            if let Ok(mut c) = cmd.spawn() {
                std::thread::spawn(move || {
                    let _ = c.wait();
                });
            }
        }
        // Yeniden açılışta izin bir kez daha denensin (oturum yenilenmiş olabilir).
        let mut s = self.status.lock().unwrap_or_else(|e| e.into_inner());
        if s.state != State::Missing {
            s.state = State::Missing;
        }
    }
}

/// Yönetici izniyle (UAC) kullanıcıyı Performance Log Users grubuna ekler.
#[cfg(windows)]
fn elevate_add_to_group(sid: &str) -> Result<(), String> {
    use windows::core::{w, HSTRING};
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject, INFINITE};
    use windows::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
    use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;
    if !sid.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err("Geçersiz kullanıcı kimliği".into());
    }
    let params = HSTRING::from(format!(
        "-NoProfile -WindowStyle Hidden -Command \"Add-LocalGroupMember -SID '{GROUP_SID}' -Member '{sid}' -ErrorAction SilentlyContinue; exit 0\""
    ));
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: w!("runas"),
        lpFile: w!("powershell.exe"),
        lpParameters: windows::core::PCWSTR(params.as_ptr()),
        nShow: SW_HIDE.0,
        ..Default::default()
    };
    unsafe {
        ShellExecuteExW(&mut info).map_err(|_| "Yönetici izni verilmedi".to_string())?;
        if !info.hProcess.is_invalid() {
            WaitForSingleObject(info.hProcess, INFINITE);
            let mut code = 0u32;
            let _ = GetExitCodeProcess(info.hProcess, &mut code);
            let _ = CloseHandle(info.hProcess);
            if code != 0 {
                return Err(format!("İzin ayarlanamadı (kod {code})"));
            }
        }
    }
    crate::log!("FPS ölçümü için kullanıcı Performance Log Users grubuna eklendi");
    Ok(())
}

#[cfg(not(windows))]
fn elevate_add_to_group(_sid: &str) -> Result<(), String> {
    Err("Yalnızca Windows'ta".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_frame_times_v2_and_v1() {
        let mut p = Parser::default();
        assert_eq!(p.feed("1.0,2.0"), None, "başlıksız satır");
        assert_eq!(
            p.feed("Application,ProcessID,SwapChainAddress,PresentRuntime,SyncInterval,PresentFlags,AllowsTearing,PresentMode,CPUStartTime,FrameTime,CPUBusy,CPUWait"),
            None
        );
        assert_eq!(
            p.feed(
                "VALORANT.exe,1234,0x1,DXGI,0,512,1,Hardware: Independent Flip,1.5,6.944,5.1,1.8"
            ),
            Some(6.944)
        );
        let mut v1 = Parser::default();
        v1.feed("Application,ProcessID,SwapChainAddress,Runtime,SyncInterval,PresentFlags,AllowsTearing,PresentMode,TimeInSeconds,msInPresentAPI,msBetweenPresents");
        assert_eq!(
            v1.feed("cs2.exe,99,0x2,DXGI,0,0,1,Composed: Flip,0.5,0.1,4.0"),
            Some(4.0)
        );
        assert_eq!(
            v1.feed("cs2.exe,99,0x2,DXGI,0,0,1,Composed: Flip,0.5,0.1,NA"),
            None
        );
    }

    #[test]
    fn fps_from_recent_frames() {
        let now = Instant::now();
        let mut f = VecDeque::new();
        f.push_back((now - Duration::from_secs(5), 100.0)); // eski, sayılmaz
        for _ in 0..120 {
            f.push_back((now, 1000.0 / 144.0));
        }
        let fps = fps_of(&f, now).unwrap();
        assert!((143.0..145.0).contains(&fps), "{fps}");
        assert_eq!(fps_of(&VecDeque::new(), now), None);
    }
}
