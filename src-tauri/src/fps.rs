//! FPS ölçümü (RivaTuner yokken): Intel PresentMon. Oyuna hiçbir şey enjekte etmez; Windows'un kendi olay
//! kayıtlarından (ETW) her karenin süresini okur. CapFrameX gibi araçların kullandığı yöntem.
//!
//! - PresentMon (MIT lisanslı, ~1 MB) ilk kurulumda `Araclar\PresentMon` klasörüne indirilir; dosya SHA-256 ile
//!   doğrulanır.
//! - Windows bu kayıtları yalnız yöneticiye ya da "Performance Log Users" (S-1-5-32-559) grubuna açar.
//!   "FPS ölçümünü aç" bir kez yönetici izni (UAC) ister ve iki şey yapar:
//!   1. **Görev:** "GMID FPS" adlı, en yüksek yetkiyle ve görünmeden (pencere açmadan) çalışan bir zamanlanmış
//!      görev kaydeder. Görevi kullanıcı kendisi her an başlatabildiği için oturumu kapatıp açmaya gerek kalmaz;
//!      çıktı geçici klasördeki bir CSV dosyasından okunur.
//!   2. **Grup:** kullanıcıyı gruba ekler; Windows oturumu bir kez yenilendikten sonra PresentMon doğrudan
//!      (görevsiz) çalışır. Görev kaydı herhangi bir nedenle olmazsa bu yol kalır.
//! - Tüm programların kareleri izlenir (dwm hariç); panel açılmadan önce öndeki pencerenin süreci seçilir, o kare
//!   göndermiyorsa (başlatıcı süreci gibi) en çok kare gönderen program alınır.
//! - Panel açıkken çalışır, panel kapanınca durdurulur.

use crate::util::hide_console;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const VERSION: &str = "2.4.1";
const URL: &str =
    "https://github.com/GameTechDev/PresentMon/releases/download/v2.4.1/PresentMon-2.4.1-x64.exe";
const SHA256: &str = "d74183e7ae630f72cd3690be0373ecbfdc6cbb86578148aab8fa2a7166068f34";
/// Performance Log Users grubunun herkesçe bilinen kimliği (Windows dili fark etmez).
pub const GROUP_SID: &str = "S-1-5-32-559";
const SESSION: &str = "GMIDPanel";
/// Yönetici izni olmadan başlatılabilen zamanlanmış görev.
const TASK: &str = "GMID FPS 2";
/// Eski sürümlerin kaydettiği görevler (yeniden kurulumda silinir). İlk sürüm arka plan oturumunda (S4U)
/// çalışıyordu; bazı bilgisayarlarda hiç başlamadı.
const OLD_TASKS: &[&str] = &["GMID FPS"];
/// Görev en çok 20 dakika çalışır (Windows durdurur); panel daha uzun açıksa 19. dakikada yeniden başlatılır.
const TASK_RESTART: Duration = Duration::from_secs(19 * 60);
/// Kare göndermesine rağmen oyun sayılmayan programlar (panelin kendisi WebView2'dir).
const IGNORED: &[&str] = &[
    "msedgewebview2.exe",
    "explorer.exe",
    "searchhost.exe",
    "searchapp.exe",
    "textinputhost.exe",
    "shellexperiencehost.exe",
    "startmenuexperiencehost.exe",
    "applicationframehost.exe",
    "gmid tool.exe",
    "gmid-tool.exe",
];

#[derive(Serialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum State {
    /// PresentMon indirilmemiş.
    #[default]
    Missing,
    /// İzin yok (görev kaydedilmemiş, gruba eklenmemiş ya da oturum yenilenmemiş).
    Permission,
    Ready,
    Running,
    Error,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub state: State,
    /// Gruba eklenme istendi ama oturum henüz yenilenmedi (görev de yok).
    pub relogin: bool,
    pub message: Option<String>,
}

/// CSV başlığından süreç, program adı ve kare süresi sütunları; satırlardan (süreç, ad, milisaniye).
#[derive(Default)]
pub struct Parser {
    cols: Option<(usize, usize, usize)>,
}

impl Parser {
    pub fn feed<'a>(&mut self, line: &'a str) -> Option<(u32, &'a str, f64)> {
        let cells: Vec<&str> = line.trim().split(',').map(str::trim).collect();
        if cells
            .first()
            .is_some_and(|c| c.eq_ignore_ascii_case("Application"))
        {
            let find = |names: &[&str]| {
                names
                    .iter()
                    .find_map(|n| cells.iter().position(|c| c.eq_ignore_ascii_case(n)))
            };
            self.cols = find(&["MsBetweenPresents", "FrameTime"])
                .zip(find(&["ProcessID"]))
                .map(|(ms, pid)| (0, pid, ms));
            return None;
        }
        let (app, pid, ms) = self.cols?;
        let v: f64 = cells.get(ms)?.parse().ok()?;
        let pid: u32 = cells.get(pid)?.parse().ok()?;
        (v > 0.0 && v < 5000.0).then_some((pid, *cells.get(app)?, v))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub at: Instant,
    pub pid: u32,
    pub ms: f64,
}

/// Son bir saniyedeki karelerden FPS: istenen süreç kare gönderiyorsa o. İstenen süreç birkaç saniye içinde
/// hiç kare göndermediyse (başlatıcı gibi, çizimi başka süreç yapıyor) en çok kare gönderen (yok sayılanlar
/// hariç); kısa süre önce gönderip durduysa (oyun küçüldü, duraklatıldı) başka bir programınki gösterilmez.
/// (fps, süreç)
pub fn pick(
    frames: &VecDeque<Frame>,
    now: Instant,
    want: Option<u32>,
    ignored: &dyn Fn(u32) -> bool,
) -> Option<(f32, u32)> {
    let mut per: HashMap<u32, (usize, f64)> = HashMap::new();
    for f in frames
        .iter()
        .filter(|f| now.duration_since(f.at) <= Duration::from_millis(1200))
    {
        let e = per.entry(f.pid).or_default();
        e.0 += 1;
        e.1 += f.ms;
    }
    if let Some(w) = want {
        let recent = per.get(&w).is_some_and(|(n, _)| *n >= 2);
        if !recent && frames.iter().any(|f| f.pid == w) {
            return None;
        }
    }
    let chosen = want
        .filter(|p| per.get(p).is_some_and(|(n, _)| *n >= 2))
        .or_else(|| {
            per.iter()
                .filter(|(pid, (n, _))| *n >= 10 && !ignored(**pid))
                .max_by_key(|(_, (n, _))| *n)
                .map(|(pid, _)| *pid)
        })?;
    let (n, sum) = per[&chosen];
    Some(((1000.0 / (sum / n as f64)) as f32, chosen))
}

enum Run {
    /// Gruptaki kullanıcı: PresentMon doğrudan, çıktısı borudan.
    Direct(Child),
    /// Zamanlanmış görev: çıktısı CSV dosyasından.
    Task(Arc<AtomicBool>),
}

#[derive(Default)]
pub struct PresentMon {
    run: Mutex<Option<Run>>,
    started: Mutex<Option<Instant>>,
    frames: Arc<Mutex<VecDeque<Frame>>>,
    names: Arc<Mutex<HashMap<u32, String>>>,
    status: Arc<Mutex<Status>>,
}

pub fn exe(root: &Path) -> PathBuf {
    root.join("Araclar")
        .join("PresentMon")
        .join("PresentMon.exe")
}

/// Görevin yazdığı CSV (OneDrive'a eşitlenen klasörlerde olmasın diye geçici klasörde).
fn task_csv() -> PathBuf {
    std::env::temp_dir().join("gmid-fps.csv")
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

fn schtasks(args: &[&str]) -> bool {
    let mut cmd = Command::new("schtasks");
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    hide_console(&mut cmd);
    cmd.status().is_ok_and(|s| s.success())
}

fn task_exists() -> bool {
    cfg!(windows) && schtasks(&["/query", "/tn", TASK])
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

/// PresentMon'un ortak seçenekleri: tüm programlar (dwm hariç), yalnız kare süreleri.
fn common_args() -> Vec<String> {
    [
        "--no_console_stats",
        "--session_name",
        SESSION,
        "--stop_existing_session",
        "--no_track_display",
        "--no_track_gpu",
        "--no_track_input",
        "--exclude",
        "dwm.exe",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

/// PresentMon'un hata çıktısı (görev başlamazsa nedenini okumak için).
fn task_err() -> PathBuf {
    std::env::temp_dir().join("gmid-fps.err")
}

/// Görevin komut satırı (PowerShell tek tırnaklı dizesi içinde): pencere açmayan konsol (`conhost --headless`)
/// içinde cmd, PresentMon'u çalıştırır ve hata çıktısını dosyaya yazar.
fn task_arguments(exe: &Path) -> String {
    let mut pm = format!(
        "\"{}\" --output_file \"{}\"",
        exe.display(),
        task_csv().display()
    );
    for s in common_args() {
        pm.push(' ');
        pm.push_str(&s);
    }
    format!(
        "--headless cmd.exe /d /s /c \"{pm} 2>\"{}\"\"",
        task_err().display()
    )
}

/// Yönetici olarak çalışacak PowerShell betiği: kullanıcıyı gruba ekler, görevi kaydeder. Çıkış kodu 3: ikisi de
/// olmadı.
fn setup_script(sid: &str, exe: &Path) -> String {
    let q = |s: &str| s.replace('\'', "''");
    format!(
        r#"$ErrorActionPreference = 'SilentlyContinue'
$sid = '{sid}'
$user = (New-Object Security.Principal.SecurityIdentifier $sid).Translate([Security.Principal.NTAccount]).Value
$group = (New-Object Security.Principal.SecurityIdentifier '{GROUP_SID}').Translate([Security.Principal.NTAccount]).Value.Split('\')[-1]
Add-LocalGroupMember -SID '{GROUP_SID}' -Member $sid
if ($user) {{ net localgroup "$group" "$user" /add *> $null }}
foreach ($old in @({old})) {{ Unregister-ScheduledTask -TaskName $old -Confirm:$false }}
$ok = $false
if ($user) {{
  $a = New-ScheduledTaskAction -Execute 'conhost.exe' -Argument '{args}'
  $p = New-ScheduledTaskPrincipal -UserId $user -LogonType Interactive -RunLevel Highest
  $s = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -ExecutionTimeLimit (New-TimeSpan -Minutes 20) -MultipleInstances IgnoreNew
  $t = Register-ScheduledTask -TaskName '{TASK}' -Action $a -Principal $p -Settings $s -Force
  if ($t) {{ $ok = $true }}
}}
if (-not $ok) {{
  $members = (net localgroup "$group") -join ' '
  if (-not $user -or $members -notmatch [regex]::Escape($user.Split('\')[-1])) {{ exit 3 }}
}}
exit 0
"#,
        args = q(&task_arguments(exe)),
        old = OLD_TASKS
            .iter()
            .map(|t| format!("'{}'", q(t)))
            .collect::<Vec<_>>()
            .join(","),
    )
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
            s.state = if in_group() || task_exists() {
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
        if in_group() || task_exists() {
            return Ok(self.set(State::Ready, None));
        }
        let sid = user_sid().ok_or("Kullanıcı kimliği okunamadı")?;
        elevate(&setup_script(&sid, &path))?;
        if task_exists() {
            crate::log!("FPS ölçümü: '{TASK}' görevi kaydedildi");
            return Ok(self.set(State::Ready, None));
        }
        crate::log!(
            "FPS ölçümü: görev kaydedilemedi, kullanıcı Performance Log Users grubuna eklendi"
        );
        let mut s = self.set(
            State::Permission,
            Some("Windows oturumunu bir kez kapatıp aç (ya da bilgisayarı yeniden başlat), sonra FPS görünür.".into()),
        );
        s.relogin = true;
        Ok(s)
    }

    fn set(&self, state: State, message: Option<String>) -> Status {
        let mut st = self.status.lock().unwrap_or_else(|e| e.into_inner());
        st.state = state;
        st.message = message;
        st.clone()
    }

    /// Panel açıkken çağrılır: ölçüm çalışmıyorsa başlatır; (fps, programın adı) döner.
    pub fn sample(&self, root: &Path, pid: Option<u32>) -> Option<(f32, String)> {
        if !installed(root) {
            return None;
        }
        let mut run = self.run.lock().unwrap_or_else(|e| e.into_inner());
        let started = *self.started.lock().unwrap_or_else(|e| e.into_inner());
        let alive = match run.as_mut() {
            Some(Run::Direct(child)) => child.try_wait().ok().flatten().is_none(),
            // Görevin süre sınırı var (unutulursa dosya büyümesin): dolmadan yeniden başlatılır.
            Some(Run::Task(stop)) => {
                let fresh = started.is_some_and(|t| t.elapsed() < TASK_RESTART);
                if !fresh {
                    stop.store(true, Ordering::Relaxed);
                    schtasks(&["/end", "/tn", TASK]);
                }
                fresh && !stop.load(Ordering::Relaxed)
            }
            None => false,
        };
        if !alive {
            if let Some(Run::Direct(mut child)) = run.take() {
                let _ = child.kill();
            }
            let st = self
                .status
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .state
                .clone();
            // İzin yoksa ya da hata verdiyse her saniye yeniden denenmez (panel yeniden açılınca denenir).
            if matches!(st, State::Permission | State::Error) {
                return None;
            }
            self.frames
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clear();
            match self.start(root) {
                Ok(r) => {
                    *run = Some(r);
                    *self.started.lock().unwrap_or_else(|e| e.into_inner()) = Some(Instant::now());
                }
                Err((state, msg)) => {
                    crate::log!("FPS ölçümü başlamadı: {msg}");
                    self.set(state, Some(msg));
                    return None;
                }
            }
        }
        drop(run);
        let own = std::process::id();
        let names = self.names.lock().unwrap_or_else(|e| e.into_inner());
        let ignored = |p: u32| {
            p == own
                || names
                    .get(&p)
                    .is_some_and(|n| IGNORED.contains(&n.to_lowercase().as_str()))
        };
        let (fps, chosen) = pick(
            &self.frames.lock().unwrap_or_else(|e| e.into_inner()),
            Instant::now(),
            pid,
            &ignored,
        )?;
        Some((fps, names.get(&chosen).cloned().unwrap_or_default()))
    }

    /// Ölçüm ne zamandır çalışıyor (çalışmıyorsa `None`).
    pub fn running_for(&self) -> Option<Duration> {
        if self.run.lock().unwrap_or_else(|e| e.into_inner()).is_none() {
            return None;
        }
        self.started
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .map(|t| t.elapsed())
    }

    fn start(&self, root: &Path) -> Result<Run, (State, String)> {
        if in_group() {
            return self.spawn_direct(root).map(Run::Direct);
        }
        if task_exists() {
            return self.start_task().map(Run::Task);
        }
        Err((
            State::Permission,
            "FPS için izin gerekiyor: Oyun Modu → FPS ölçümünü aç".into(),
        ))
    }

    fn record(&self) -> impl FnMut(&str) + Send + 'static {
        let frames = self.frames.clone();
        let names = self.names.clone();
        let mut parser = Parser::default();
        move |line: &str| {
            if let Some((pid, app, ms)) = parser.feed(line) {
                names
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .entry(pid)
                    .or_insert_with(|| app.to_string());
                let mut f = frames.lock().unwrap_or_else(|e| e.into_inner());
                let now = Instant::now();
                f.push_back(Frame { at: now, pid, ms });
                while f
                    .front()
                    .is_some_and(|x| now.duration_since(x.at) > Duration::from_secs(3))
                {
                    f.pop_front();
                }
            }
        }
    }

    fn spawn_direct(&self, root: &Path) -> Result<Child, (State, String)> {
        let mut cmd = Command::new(exe(root));
        cmd.arg("--output_stdout")
            .args(common_args())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        hide_console(&mut cmd);
        let mut child = cmd
            .spawn()
            .map_err(|e| (State::Error, format!("PresentMon açılamadı: {e}")))?;
        let out = child.stdout.take();
        let err = child.stderr.take();
        self.set(State::Running, None);
        let status = self.status.clone();
        let mut record = self.record();
        std::thread::spawn(move || {
            if let Some(out) = out {
                for line in BufReader::new(out).lines().map_while(Result::ok) {
                    record(&line);
                }
            }
            // Çıktı bitti: neden?
            let mut text = String::new();
            if let Some(mut e) = err {
                let _ = e.read_to_string(&mut text);
            }
            let lower = text.to_lowercase();
            let mut s = status.lock().unwrap_or_else(|e| e.into_inner());
            if lower.contains("denied") || lower.contains("performance log users") {
                s.state = State::Permission;
                s.message = Some("FPS için izin gerekiyor: Oyun Modu → FPS ölçümünü aç".into());
                crate::log!("PresentMon: izin yok");
            } else if s.state == State::Running {
                match text
                    .lines()
                    .rev()
                    .find(|l| l.to_lowercase().contains("error"))
                {
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

    fn start_task(&self) -> Result<Arc<AtomicBool>, (State, String)> {
        let csv = task_csv();
        let _ = std::fs::remove_file(&csv);
        let _ = std::fs::remove_file(task_err());
        if !schtasks(&["/run", "/tn", TASK]) {
            return Err((
                State::Permission,
                "FPS görevi başlatılamadı: Oyun Modu → FPS ölçümünü aç ile yeniden kur".into(),
            ));
        }
        self.set(State::Running, None);
        let stop = Arc::new(AtomicBool::new(false));
        let stop2 = stop.clone();
        let status = self.status.clone();
        let mut record = self.record();
        std::thread::spawn(move || {
            // Dosya birkaç yüz milisaniye içinde oluşur.
            let begun = Instant::now();
            let file = loop {
                if stop2.load(Ordering::Relaxed) {
                    return;
                }
                if let Ok(f) = std::fs::File::open(&csv) {
                    break f;
                }
                if begun.elapsed() > Duration::from_secs(12) {
                    let why = task_failure();
                    crate::log!("PresentMon görevi 12 saniyede çıktı yazmadı: {why}");
                    let mut s = status.lock().unwrap_or_else(|e| e.into_inner());
                    s.state = State::Error;
                    s.message = Some(format!(
                        "FPS görevi çalışmadı ({why}). Windows oturumunu bir kez kapatıp açarsan doğrudan ölçülür"
                    ));
                    stop2.store(true, Ordering::Relaxed);
                    return;
                }
                std::thread::sleep(Duration::from_millis(200));
            };
            tail(file, &stop2, &mut record);
        });
        Ok(stop)
    }

    /// Panel kapanınca. Doğrudan çalışan süreçte Windows'taki izleme oturumu da kapatılır; görevde bir sonraki
    /// başlatma eskisini kapatır.
    pub fn stop(&self, root: &Path) {
        match self.run.lock().unwrap_or_else(|e| e.into_inner()).take() {
            Some(Run::Direct(mut child)) => {
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
            Some(Run::Task(stop)) => {
                stop.store(true, Ordering::Relaxed);
                std::thread::spawn(|| {
                    schtasks(&["/end", "/tn", TASK]);
                    let _ = std::fs::remove_file(task_csv());
                });
            }
            None => {}
        }
        *self.started.lock().unwrap_or_else(|e| e.into_inner()) = None;
        // Yeniden açılışta izin bir kez daha denensin (oturum yenilenmiş, görev kurulmuş olabilir).
        let mut s = self.status.lock().unwrap_or_else(|e| e.into_inner());
        if s.state != State::Missing {
            s.state = State::Missing;
        }
    }
}

/// Görev neden çıktı vermedi: PresentMon'un hata satırı ya da görevin son sonuç kodu.
fn task_failure() -> String {
    if let Ok(t) = std::fs::read_to_string(task_err()) {
        if let Some(l) = t.lines().map(str::trim).rfind(|l| !l.is_empty()) {
            return l.chars().take(160).collect();
        }
    }
    let mut cmd = Command::new("powershell");
    cmd.args([
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        &format!("(Get-ScheduledTaskInfo -TaskName '{TASK}').LastTaskResult"),
    ])
    .stdin(Stdio::null())
    .stderr(Stdio::null());
    hide_console(&mut cmd);
    match cmd.output().ok().and_then(|o| {
        String::from_utf8_lossy(&o.stdout)
            .trim()
            .parse::<i64>()
            .ok()
    }) {
        // 267009: hâlâ çalışıyor, 267011: hiç çalışmadı.
        Some(code) => format!("kod 0x{:X}", code as u32),
        None => "neden okunamadı".into(),
    }
}

/// Büyüyen bir dosyanın yeni satırlarını okur (yarım satırlar tamamlanınca).
fn tail(mut file: std::fs::File, stop: &AtomicBool, record: &mut dyn FnMut(&str)) {
    let mut pos = 0u64;
    let mut pending = String::new();
    let mut buf = Vec::new();
    while !stop.load(Ordering::Relaxed) {
        buf.clear();
        if file.seek(SeekFrom::Start(pos)).is_ok() {
            let _ = (&mut file).take(1 << 20).read_to_end(&mut buf);
        }
        if buf.is_empty() {
            std::thread::sleep(Duration::from_millis(100));
            continue;
        }
        pos += buf.len() as u64;
        pending.push_str(&String::from_utf8_lossy(&buf));
        while let Some(i) = pending.find('\n') {
            let line: String = pending.drain(..=i).collect();
            record(&line);
        }
    }
}

/// Yönetici izniyle (UAC) PowerShell betiği çalıştırır, bitmesini bekler.
fn elevate(script: &str) -> Result<(), String> {
    match crate::util::run_elevated_ps("fps-izin", script)? {
        0 => Ok(()),
        code => {
            crate::log!("FPS izni betiği {code} koduyla bitti");
            Err(format!("İzin ayarlanamadı (kod {code})"))
        }
    }
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
                "VALORANT.exe,1234,0x1,DXGI,0,512,1,Hardware: Independent Flip,1.5,6.944,5.1,1.8\r\n"
            ),
            Some((1234, "VALORANT.exe", 6.944))
        );
        // 2.x: MsBetweenPresents varsa o.
        let mut p2 = Parser::default();
        p2.feed("Application,ProcessID,SwapChainAddress,PresentRuntime,SyncInterval,PresentFlags,AllowsTearing,PresentMode,MsBetweenSimulationStart,MsBetweenPresents,FrameTime");
        assert_eq!(
            p2.feed("cod2.exe,77,0x1,D3D9,0,0,0,Composed: Flip,NA,16.6,16.7"),
            Some((77, "cod2.exe", 16.6))
        );
        let mut v1 = Parser::default();
        v1.feed("Application,ProcessID,SwapChainAddress,Runtime,SyncInterval,PresentFlags,AllowsTearing,PresentMode,TimeInSeconds,msInPresentAPI,msBetweenPresents");
        assert_eq!(
            v1.feed("cs2.exe,99,0x2,DXGI,0,0,1,Composed: Flip,0.5,0.1,4.0"),
            Some((99, "cs2.exe", 4.0))
        );
        assert_eq!(
            v1.feed("cs2.exe,99,0x2,DXGI,0,0,1,Composed: Flip,0.5,0.1,NA"),
            None
        );
    }

    #[test]
    fn picks_wanted_process_or_busiest() {
        let now = Instant::now();
        let mut f = VecDeque::new();
        let push = |f: &mut VecDeque<Frame>, pid, ms, n, ago: u64| {
            for _ in 0..n {
                f.push_back(Frame {
                    at: now - Duration::from_millis(ago),
                    pid,
                    ms,
                });
            }
        };
        push(&mut f, 1, 100.0, 5, 5000); // eski, sayılmaz
        push(&mut f, 10, 1000.0 / 144.0, 140, 100); // oyun
        push(&mut f, 20, 1000.0 / 60.0, 60, 100); // panel (WebView2)
        push(&mut f, 30, 1000.0 / 30.0, 3, 100); // az kare
        let none = |_| false;
        let (fps, pid) = pick(&f, now, Some(10), &none).unwrap();
        assert_eq!(pid, 10);
        assert!((143.0..145.0).contains(&fps), "{fps}");
        // İstenen (başlatıcı) kare göndermiyor: en çok kare gönderen.
        assert_eq!(pick(&f, now, Some(999), &none).unwrap().1, 10);
        // Oyun küçüldü (2 sn önce kare gönderiyordu): başka programınki gösterilmez.
        let mut paused = VecDeque::new();
        push(&mut paused, 10, 7.0, 100, 2500);
        push(&mut paused, 40, 16.6, 60, 100);
        assert_eq!(pick(&paused, now, Some(10), &none), None);
        // Oyun yokken panel sayılmaz.
        let mut only_panel = VecDeque::new();
        push(&mut only_panel, 20, 16.6, 60, 100);
        assert_eq!(pick(&only_panel, now, None, &|p| p == 20), None);
        assert_eq!(pick(&VecDeque::new(), now, None, &none), None);
    }

    #[test]
    fn setup_script_quotes_paths() {
        let s = setup_script(
            "S-1-5-21-1-2-3-1001",
            Path::new(r"C:\Users\O'Neil\Documents\GMID Tool\Araclar\PresentMon\PresentMon.exe"),
        );
        assert!(
            s.contains(r#"-Argument '--headless cmd.exe /d /s /c ""C:\Users\O''Neil\Documents"#),
            "{s}"
        );
        assert!(s.contains("-LogonType Interactive -RunLevel Highest"));
        assert!(s.contains("--exclude dwm.exe 2>\""));
        assert!(s.contains("Register-ScheduledTask -TaskName 'GMID FPS 2'"));
        assert!(s.contains("foreach ($old in @('GMID FPS'))"));
    }

    #[test]
    fn tail_reads_complete_lines() {
        let path = std::env::temp_dir().join(format!("gmid-tail-{}.csv", std::process::id()));
        std::fs::write(
            &path,
            "Application,ProcessID,MsBetweenPresents\na.exe,1,10\nb.exe,2,2",
        )
        .unwrap();
        let stop = AtomicBool::new(false);
        let mut lines = Vec::new();
        let file = std::fs::File::open(&path).unwrap();
        std::thread::scope(|s| {
            s.spawn(|| {
                std::thread::sleep(Duration::from_millis(350));
                stop.store(true, Ordering::Relaxed);
            });
            tail(file, &stop, &mut |l| lines.push(l.trim().to_string()));
        });
        assert_eq!(
            lines,
            ["Application,ProcessID,MsBetweenPresents", "a.exe,1,10"]
        );
        let _ = std::fs::remove_file(&path);
    }
}
