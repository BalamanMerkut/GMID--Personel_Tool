//! Windows Core Audio uygulaması.
#![allow(non_snake_case)]

use super::{app_label, AppVolume, AudioState, Cmd, Device, MicOp, MicStatus};
use crate::db::Db;
use std::sync::mpsc::{self, Sender};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use windows::core::Interface;
use windows::core::{
    implement, interface, IUnknown, IUnknown_Vtbl, Result as WinResult, GUID, HRESULT, HSTRING,
    PCWSTR,
};
use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::Media::Audio::Endpoints::{
    IAudioEndpointVolume, IAudioEndpointVolumeCallback, IAudioEndpointVolumeCallback_Impl,
};
use windows::Win32::Media::Audio::{
    eCapture, eCommunications, eConsole, eMultimedia, eRender, EDataFlow, ERole, IMMDevice,
    IMMDeviceEnumerator, IMMNotificationClient, IMMNotificationClient_Impl, MMDeviceEnumerator,
    AUDIO_VOLUME_NOTIFICATION_DATA, DEVICE_STATE, DEVICE_STATE_ACTIVE,
};
use windows::Win32::Media::Audio::{
    AudioSessionStateActive, AudioSessionStateExpired, IAudioSessionControl2,
    IAudioSessionManager2, ISimpleAudioVolume,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_MULTITHREADED, STGM_READ,
};

/// Kendi yaptığımız ses değişikliklerini tanımak için olay kimliği.
const EVENT_CTX: GUID = GUID::from_u128(0x7a3c1d52_4b8e_4f0a_9d6e_2c5b8a1f0e93);
/// Belgelenmemiş "PolicyConfig" istemcisi: Windows'un kendi ses ayarlarının varsayılan cihazı
/// değiştirmek için kullandığı arayüz (SoundSwitch ve benzerleri de bunu kullanır).
const CLSID_POLICY_CONFIG: GUID = GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);
/// Kilit bu kadar sapmaya izin verir (yuvarlama farkları için).
const LOCK_TOLERANCE: f32 = 0.004;

#[interface("f8679f50-850a-41cf-9c72-430f290290c8")]
unsafe trait IPolicyConfig: IUnknown {
    fn GetMixFormat(&self, device: PCWSTR, format: *mut *mut core::ffi::c_void) -> HRESULT;
    fn GetDeviceFormat(
        &self,
        device: PCWSTR,
        default: i32,
        format: *mut *mut core::ffi::c_void,
    ) -> HRESULT;
    fn ResetDeviceFormat(&self, device: PCWSTR) -> HRESULT;
    fn SetDeviceFormat(
        &self,
        device: PCWSTR,
        endpoint: *mut core::ffi::c_void,
        mix: *mut core::ffi::c_void,
    ) -> HRESULT;
    fn GetProcessingPeriod(
        &self,
        device: PCWSTR,
        default: i32,
        default_period: *mut i64,
        min_period: *mut i64,
    ) -> HRESULT;
    fn SetProcessingPeriod(&self, device: PCWSTR, period: *mut i64) -> HRESULT;
    fn GetShareMode(&self, device: PCWSTR, mode: *mut core::ffi::c_void) -> HRESULT;
    fn SetShareMode(&self, device: PCWSTR, mode: *mut core::ffi::c_void) -> HRESULT;
    fn GetPropertyValue(
        &self,
        device: PCWSTR,
        key: *const core::ffi::c_void,
        value: *mut core::ffi::c_void,
    ) -> HRESULT;
    fn SetPropertyValue(
        &self,
        device: PCWSTR,
        key: *const core::ffi::c_void,
        value: *mut core::ffi::c_void,
    ) -> HRESULT;
    fn SetDefaultEndpoint(&self, device: PCWSTR, role: ERole) -> HRESULT;
    fn SetEndpointVisibility(&self, device: PCWSTR, visible: i32) -> HRESULT;
}

/// Mikrofon seviyesi değişince Windows'un çağırdığı nesne.
#[implement(IAudioEndpointVolumeCallback)]
struct VolumeCallback {
    tx: Mutex<Sender<Cmd>>,
}

impl IAudioEndpointVolumeCallback_Impl for VolumeCallback_Impl {
    fn OnNotify(&self, data: *mut AUDIO_VOLUME_NOTIFICATION_DATA) -> WinResult<()> {
        // Hızlı dön: asıl iş ses iş parçacığında yapılır.
        if let Some(d) = unsafe { data.as_ref() } {
            if d.guidEventContext != EVENT_CTX {
                if let Ok(tx) = self.tx.lock() {
                    let _ = tx.send(Cmd::MicNotify(d.fMasterVolume));
                }
            }
        }
        Ok(())
    }
}

/// Cihaz takılıp çıkarılınca ya da varsayılan değişince Windows'un çağırdığı nesne.
#[implement(IMMNotificationClient)]
struct DeviceWatcher {
    tx: Mutex<Sender<Cmd>>,
}

impl DeviceWatcher {
    fn send(&self, cmd: Cmd) {
        if let Ok(tx) = self.tx.lock() {
            let _ = tx.send(cmd);
        }
    }
}

impl IMMNotificationClient_Impl for DeviceWatcher_Impl {
    fn OnDeviceStateChanged(&self, _id: &PCWSTR, _state: DEVICE_STATE) -> WinResult<()> {
        self.send(Cmd::DevicesChanged);
        Ok(())
    }
    fn OnDeviceAdded(&self, _id: &PCWSTR) -> WinResult<()> {
        self.send(Cmd::DevicesChanged);
        Ok(())
    }
    fn OnDeviceRemoved(&self, _id: &PCWSTR) -> WinResult<()> {
        self.send(Cmd::DevicesChanged);
        Ok(())
    }
    fn OnDefaultDeviceChanged(&self, flow: EDataFlow, role: ERole, _id: &PCWSTR) -> WinResult<()> {
        if role == eConsole {
            self.send(if flow == eCapture {
                Cmd::DefaultChanged
            } else {
                Cmd::DevicesChanged
            });
        }
        Ok(())
    }
    fn OnPropertyValueChanged(&self, _id: &PCWSTR, _key: &PROPERTYKEY) -> WinResult<()> {
        Ok(())
    }
}

pub(super) fn spawn(app: AppHandle, lock: Option<f32>) -> Sender<Cmd> {
    let (tx, rx) = mpsc::channel::<Cmd>();
    let worker_tx = tx.clone();
    std::thread::Builder::new()
        .name("gmid-ses".into())
        .spawn(move || {
            let init = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
            let worker = init.ok().and_then(|_| Worker::new(app, worker_tx, lock));
            match worker {
                Ok(mut w) => {
                    for cmd in rx {
                        w.handle(cmd);
                    }
                }
                Err(e) => {
                    let msg = format!("Ses servisi başlatılamadı: {e}");
                    crate::log!("{msg}");
                    for cmd in rx {
                        reply_error(cmd, &msg);
                    }
                }
            }
        })
        .expect("ses iş parçacığı başlatılamadı");
    tx
}

fn reply_error(cmd: Cmd, msg: &str) {
    let msg = msg.to_string();
    match cmd {
        Cmd::State(r) => drop(r.send(Err(msg))),
        Cmd::SetDefault(_, r) | Cmd::SetVolume(_, _, r) | Cmd::SetMute(_, _, r) => {
            drop(r.send(Err(msg)))
        }
        Cmd::Mic(_, r) => drop(r.send(Err(msg))),
        Cmd::Apps(r) => drop(r.send(Err(msg))),
        Cmd::SetAppVolume(_, _, r) | Cmd::SetAppMute(_, _, r) => drop(r.send(Err(msg))),
        _ => {}
    }
}

struct Worker {
    app: AppHandle,
    tx: Sender<Cmd>,
    enumerator: IMMDeviceEnumerator,
    watcher: IMMNotificationClient,
    mic: Option<(IAudioEndpointVolume, IAudioEndpointVolumeCallback)>,
    lock: Option<f32>,
    last_relock_notice: Option<Instant>,
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.unbind_mic();
        unsafe {
            let _ = self
                .enumerator
                .UnregisterEndpointNotificationCallback(&self.watcher);
        }
    }
}

impl Worker {
    fn new(app: AppHandle, tx: Sender<Cmd>, lock: Option<f32>) -> WinResult<Self> {
        let enumerator: IMMDeviceEnumerator =
            unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)? };
        let watcher: IMMNotificationClient = DeviceWatcher {
            tx: Mutex::new(tx.clone()),
        }
        .into();
        unsafe { enumerator.RegisterEndpointNotificationCallback(&watcher)? };
        let mut w = Self {
            app,
            tx,
            enumerator,
            watcher,
            mic: None,
            lock,
            last_relock_notice: None,
        };
        w.bind_mic();
        Ok(w)
    }

    fn handle(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::State(r) => drop(r.send(self.state().map_err(|e| e.to_string()))),
            Cmd::SetDefault(id, r) => {
                let res = self
                    .set_default(&id)
                    .map_err(|e| format!("Varsayılan cihaz değiştirilemedi: {e}"));
                drop(r.send(res));
            }
            Cmd::SetVolume(id, level, r) => {
                let res = self.set_volume(&id, level).map_err(|e| e.to_string());
                drop(r.send(res));
            }
            Cmd::SetMute(id, muted, r) => {
                let res = self
                    .volume_of(&id)
                    .and_then(|v| unsafe { v.SetMute(muted, &EVENT_CTX) })
                    .map_err(|e| e.to_string());
                drop(r.send(res));
            }
            Cmd::Mic(op, r) => drop(r.send(self.mic_op(op).map_err(|e| e.to_string()))),
            Cmd::Apps(r) => drop(r.send(self.apps().map_err(|e| e.to_string()))),
            Cmd::SetAppVolume(pid, level, r) => {
                let res = self
                    .for_app(pid, |v| unsafe { v.SetMasterVolume(level, &EVENT_CTX) })
                    .map_err(|e| e.to_string());
                drop(r.send(res));
            }
            Cmd::SetAppMute(pid, muted, r) => {
                let res = self
                    .for_app(pid, |v| unsafe { v.SetMute(muted, &EVENT_CTX) })
                    .map_err(|e| e.to_string());
                drop(r.send(res));
            }
            Cmd::MicNotify(level) => self.on_mic_changed(level),
            Cmd::DevicesChanged => self.changed(),
            Cmd::DefaultChanged => {
                self.bind_mic();
                self.changed();
            }
        }
    }

    fn changed(&self) {
        let _ = self.app.emit("audio-changed", ());
    }

    // ---- cihazlar ----

    fn state(&self) -> WinResult<AudioState> {
        Ok(AudioState {
            supported: true,
            outputs: self.devices(eRender)?,
            inputs: self.devices(eCapture)?,
            lock: self.lock,
        })
    }

    fn devices(&self, flow: EDataFlow) -> WinResult<Vec<Device>> {
        unsafe {
            let default_id = self
                .enumerator
                .GetDefaultAudioEndpoint(flow, eConsole)
                .ok()
                .and_then(|d| device_id(&d).ok());
            let coll = self
                .enumerator
                .EnumAudioEndpoints(flow, DEVICE_STATE_ACTIVE)?;
            let mut out = Vec::new();
            for i in 0..coll.GetCount()? {
                let dev = coll.Item(i)?;
                let id = device_id(&dev)?;
                let name = friendly_name(&dev).unwrap_or_else(|_| "Adsız cihaz".into());
                let (level, muted) = match dev.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None) {
                    Ok(v) => (
                        v.GetMasterVolumeLevelScalar().unwrap_or(0.0),
                        v.GetMute().map(|m| m.as_bool()).unwrap_or(false),
                    ),
                    Err(_) => (0.0, false),
                };
                out.push(Device {
                    is_default: default_id.as_deref() == Some(id.as_str()),
                    id,
                    name,
                    level,
                    muted,
                });
            }
            Ok(out)
        }
    }

    // ---- uygulama başına ses ----

    /// Varsayılan çıkıştaki ses oturumları: (süreç, sistem sesi mi, etkin mi, ses denetimi).
    fn sessions(&self) -> WinResult<Vec<(u32, bool, bool, ISimpleAudioVolume)>> {
        unsafe {
            let dev = self.enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
            let mgr: IAudioSessionManager2 = dev.Activate(CLSCTX_ALL, None)?;
            let list = mgr.GetSessionEnumerator()?;
            let mut out = Vec::new();
            for i in 0..list.GetCount()? {
                let Ok(ctl) = list.GetSession(i) else {
                    continue;
                };
                let Ok(state) = ctl.GetState() else { continue };
                if state == AudioSessionStateExpired {
                    continue;
                }
                let Ok(ctl2) = ctl.cast::<IAudioSessionControl2>() else {
                    continue;
                };
                let pid = ctl2.GetProcessId().unwrap_or(0);
                // S_OK (0) ise sistem sesleri oturumu.
                let system = ctl2.IsSystemSoundsSession() == HRESULT(0);
                let Ok(vol) = ctl.cast::<ISimpleAudioVolume>() else {
                    continue;
                };
                out.push((pid, system, state == AudioSessionStateActive, vol));
            }
            Ok(out)
        }
    }

    fn apps(&self) -> WinResult<Vec<AppVolume>> {
        let own = std::process::id();
        let mut out: Vec<AppVolume> = Vec::new();
        // GMID'in kendi sesi (oynatıcı) WebView2 alt sürecinden çıkar: "Msedgewebview2" değil "GMID Tool" yazsın.
        let mut parents = None;
        for (pid, system, active, vol) in self.sessions()? {
            let key = if system { 0 } else { pid };
            if let Some(a) = out.iter_mut().find(|a| a.pid == key) {
                a.active |= active;
                continue;
            }
            let (exe, name) = if system {
                (String::new(), "Sistem sesleri".to_string())
            } else {
                let exe = crate::games::procs::exe_of(pid).unwrap_or_default();
                if exe.is_empty() && pid != own {
                    (exe, format!("Uygulama {pid}"))
                } else {
                    let webview = exe.to_lowercase().starts_with("msedgewebview2");
                    let n = if pid == own
                        || (webview
                            && crate::games::procs::descends_from(
                                parents.get_or_insert_with(crate::games::procs::parents),
                                pid,
                                own,
                            )) {
                        "GMID Tool".to_string()
                    } else {
                        app_label(&exe)
                    };
                    (exe, n)
                }
            };
            let (level, muted) = unsafe {
                (
                    vol.GetMasterVolume().unwrap_or(1.0),
                    vol.GetMute().map(|m| m.as_bool()).unwrap_or(false),
                )
            };
            out.push(AppVolume {
                pid: key,
                name,
                exe,
                level,
                muted,
                active,
            });
        }
        // Ses çıkaranlar önce, sonra ada göre; sistem sesleri en sonda.
        out.sort_by(|a, b| {
            (a.pid == 0)
                .cmp(&(b.pid == 0))
                .then(b.active.cmp(&a.active))
                .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        Ok(out)
    }

    /// Bir uygulamanın bütün oturumlarına uygular (Discord gibi birden çok oturum açanlar için).
    fn for_app(&self, pid: u32, f: impl Fn(&ISimpleAudioVolume) -> WinResult<()>) -> WinResult<()> {
        let mut any = false;
        for (p, system, _, vol) in self.sessions()? {
            if (pid == 0 && system) || (pid != 0 && p == pid && !system) {
                f(&vol)?;
                any = true;
            }
        }
        if any {
            Ok(())
        } else {
            Err(windows::core::Error::new(
                windows::Win32::Foundation::E_INVALIDARG,
                "Uygulama artık ses çıkarmıyor",
            ))
        }
    }

    fn set_default(&self, id: &str) -> WinResult<()> {
        let policy: IPolicyConfig =
            unsafe { CoCreateInstance(&CLSID_POLICY_CONFIG, None, CLSCTX_ALL)? };
        let h = HSTRING::from(id);
        for role in [eConsole, eMultimedia, eCommunications] {
            unsafe { policy.SetDefaultEndpoint(PCWSTR(h.as_ptr()), role).ok()? };
        }
        Ok(())
    }

    fn volume_of(&self, id: &str) -> WinResult<IAudioEndpointVolume> {
        unsafe {
            let dev = self.enumerator.GetDevice(&HSTRING::from(id))?;
            dev.Activate(CLSCTX_ALL, None)
        }
    }

    fn set_volume(&mut self, id: &str, level: f32) -> WinResult<()> {
        unsafe {
            self.volume_of(id)?
                .SetMasterVolumeLevelScalar(level, &EVENT_CTX)?
        };
        // Kilitliyken kullanıcı varsayılan mikrofonu kendisi değiştirdiyse yeni seviye kilitlenir.
        if self.lock.is_some() && self.default_mic_id().as_deref() == Some(id) {
            self.set_lock(Some(level));
        }
        Ok(())
    }

    // ---- mikrofon ----

    fn default_mic_id(&self) -> Option<String> {
        unsafe {
            self.enumerator
                .GetDefaultAudioEndpoint(eCapture, eConsole)
                .ok()
                .and_then(|d| device_id(&d).ok())
        }
    }

    fn unbind_mic(&mut self) {
        if let Some((vol, cb)) = self.mic.take() {
            unsafe {
                let _ = vol.UnregisterControlChangeNotify(&cb);
            }
        }
    }

    /// Varsayılan mikrofonun seviye değişikliklerini dinlemeye başlar.
    fn bind_mic(&mut self) {
        self.unbind_mic();
        let bound: WinResult<_> = unsafe {
            self.enumerator
                .GetDefaultAudioEndpoint(eCapture, eConsole)
                .and_then(|dev| dev.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None))
                .and_then(|vol| {
                    let cb: IAudioEndpointVolumeCallback = VolumeCallback {
                        tx: Mutex::new(self.tx.clone()),
                    }
                    .into();
                    vol.RegisterControlChangeNotify(&cb)?;
                    Ok((vol, cb))
                })
        };
        self.mic = bound.ok();
        self.enforce_lock();
    }

    /// Kilit açıksa seviyeyi hedefe çeker; çektiyse `true`.
    fn enforce_lock(&self) -> bool {
        let (Some(target), Some((vol, _))) = (self.lock, &self.mic) else {
            return false;
        };
        unsafe {
            let current = vol.GetMasterVolumeLevelScalar().unwrap_or(target);
            if (current - target).abs() > LOCK_TOLERANCE {
                return vol.SetMasterVolumeLevelScalar(target, &EVENT_CTX).is_ok();
            }
        }
        false
    }

    fn on_mic_changed(&mut self, level: f32) {
        if let Some(target) = self.lock {
            if (level - target).abs() > LOCK_TOLERANCE && self.enforce_lock() {
                // Bir program sürekli kısıyorsa göstergeyi en fazla 5 sn'de bir göster.
                let quiet = self
                    .last_relock_notice
                    .is_some_and(|t| t.elapsed() < Duration::from_secs(5));
                if !quiet {
                    self.last_relock_notice = Some(Instant::now());
                    crate::overlay::show_osd(
                        &self.app,
                        "lock",
                        format!("Mikrofon %{} seviyesine geri çekildi", percent(target)),
                    );
                }
            }
        }
        self.changed();
    }

    fn set_lock(&mut self, lock: Option<f32>) {
        self.lock = lock.map(|l| l.clamp(0.0, 1.0));
        let value = self.lock.map_or("null".to_string(), |l| format!("{l}"));
        let _ = self.app.state::<Db>().set_setting("audio.micLock", &value);
        self.enforce_lock();
        self.changed();
    }

    fn mic_op(&mut self, op: MicOp) -> WinResult<MicStatus> {
        let Some((vol, _)) = self.mic.clone() else {
            return Err(windows::core::Error::new(
                windows::Win32::Foundation::E_FAIL,
                "Mikrofon bulunamadı",
            ));
        };
        unsafe {
            match op {
                MicOp::ToggleMute => {
                    let muted = vol.GetMute()?.as_bool();
                    vol.SetMute(!muted, &EVENT_CTX)?;
                }
                MicOp::Step(delta) => {
                    let level = (vol.GetMasterVolumeLevelScalar()? + delta).clamp(0.0, 1.0);
                    vol.SetMasterVolumeLevelScalar(level, &EVENT_CTX)?;
                    if self.lock.is_some() {
                        self.set_lock(Some(level));
                    }
                }
                MicOp::Max => {
                    vol.SetMasterVolumeLevelScalar(1.0, &EVENT_CTX)?;
                    if self.lock.is_some() {
                        self.set_lock(Some(1.0));
                    }
                }
                MicOp::SetLock(l) => self.set_lock(l),
                MicOp::ToggleLock(target) => {
                    let next = if self.lock.is_some() {
                        None
                    } else {
                        Some(target)
                    };
                    self.set_lock(next);
                }
            }
            self.changed();
            Ok(MicStatus {
                level: vol.GetMasterVolumeLevelScalar()?,
                muted: vol.GetMute()?.as_bool(),
                lock: self.lock,
            })
        }
    }
}

fn percent(level: f32) -> u32 {
    (level * 100.0).round() as u32
}

unsafe fn device_id(dev: &IMMDevice) -> WinResult<String> {
    let p = dev.GetId()?;
    let s = p.to_string().unwrap_or_default();
    CoTaskMemFree(Some(p.0 as *const _));
    Ok(s)
}

unsafe fn friendly_name(dev: &IMMDevice) -> WinResult<String> {
    let store = dev.OpenPropertyStore(STGM_READ)?;
    let value = store.GetValue(&PKEY_Device_FriendlyName)?;
    Ok(value.to_string())
}
