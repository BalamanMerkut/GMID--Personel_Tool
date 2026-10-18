//! Ses Paneli motoru (WASAPI): gerçek mikrofon + çalan sesler → sanal kablo; çalan sesler → kulaklık.
//! Bütün ses nesneleri bu iş parçacığında yaşar; dışarıyla yalnız komut kanalıyla konuşulur.
#![allow(non_snake_case)]

use super::{is_virtual, to_i16, Cmd, Devices, Endpoint, Levels, Voice, RATE};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use windows::core::{Result as WinResult, HSTRING};
use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
use windows::Win32::Media::Audio::{
    eCapture, eConsole, eRender, EDataFlow, IAudioCaptureClient, IAudioClient, IAudioRenderClient,
    IMMDevice, IMMDeviceEnumerator, MMDeviceEnumerator, AUDCLNT_BUFFERFLAGS_SILENT,
    AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM,
    AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY, DEVICE_STATE_ACTIVE, WAVEFORMATEX, WAVE_FORMAT_PCM,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_MULTITHREADED, STGM_READ,
};

/// Motorla dış dünya arasında paylaşılanlar.
pub struct Shared {
    pub playing: Arc<Mutex<Vec<String>>>,
    pub alive: Arc<AtomicBool>,
    pub error: Arc<Mutex<Option<String>>>,
    pub app: AppHandle,
}

/// Kablo ve kulaklık arabelleğinde tutulan ses (gecikme ≈ bu kadar).
const TARGET_MS: u32 = 40;
/// Mikrofon kuyruğu bundan uzarsa (saatler kayınca) baştan kırpılır.
const MIC_MAX_MS: u32 = 120;
const MIC_KEEP_MS: u32 = 40;

fn frames(ms: u32) -> usize {
    (RATE / 1000 * ms) as usize
}

fn format(channels: u16) -> WAVEFORMATEX {
    WAVEFORMATEX {
        wFormatTag: WAVE_FORMAT_PCM as u16,
        nChannels: channels,
        nSamplesPerSec: RATE,
        nAvgBytesPerSec: RATE * 2 * channels as u32,
        nBlockAlign: 2 * channels,
        wBitsPerSample: 16,
        cbSize: 0,
    }
}

fn enumerator() -> WinResult<IMMDeviceEnumerator> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
    }
}

unsafe fn device_id(dev: &IMMDevice) -> WinResult<String> {
    let p = dev.GetId()?;
    let s = p.to_string().unwrap_or_default();
    CoTaskMemFree(Some(p.0 as *const _));
    Ok(s)
}

unsafe fn friendly_name(dev: &IMMDevice) -> WinResult<String> {
    let store = dev.OpenPropertyStore(STGM_READ)?;
    Ok(store.GetValue(&PKEY_Device_FriendlyName)?.to_string())
}

fn list(en: &IMMDeviceEnumerator, flow: EDataFlow) -> WinResult<Vec<Endpoint>> {
    unsafe {
        let default = en
            .GetDefaultAudioEndpoint(flow, eConsole)
            .ok()
            .and_then(|d| device_id(&d).ok());
        let coll = en.EnumAudioEndpoints(flow, DEVICE_STATE_ACTIVE)?;
        let mut out = Vec::new();
        for i in 0..coll.GetCount()? {
            let dev = coll.Item(i)?;
            let id = device_id(&dev)?;
            let name = friendly_name(&dev).unwrap_or_else(|_| "Adsız cihaz".into());
            out.push(Endpoint {
                is_default: default.as_deref() == Some(id.as_str()),
                virtual_cable: is_virtual(&name),
                id,
                name,
            });
        }
        Ok(out)
    }
}

pub fn devices() -> WinResult<Devices> {
    let en = enumerator()?;
    let outputs = list(&en, eRender)?;
    let inputs = list(&en, eCapture)?;
    // Önce VB-CABLE'ın kendi girişi, yoksa Voicemeeter'ınki.
    let cable = outputs
        .iter()
        .find(|d| d.name.to_lowercase().contains("cable input"))
        .or_else(|| outputs.iter().find(|d| d.virtual_cable))
        .cloned();
    let cable_mic = match &cable {
        Some(c) if c.name.to_lowercase().contains("voicemeeter") => {
            "VoiceMeeter Output".to_string()
        }
        Some(_) => "CABLE Output".to_string(),
        None => String::new(),
    };
    let default_out = outputs.iter().find(|d| d.is_default);
    Ok(Devices {
        cable,
        cable_mic,
        default_output_is_cable: default_out.is_some_and(|d| d.virtual_cable),
        default_output_name: default_out.map(|d| d.name.clone()).unwrap_or_default(),
        default_input_is_cable: inputs.iter().any(|d| d.is_default && d.virtual_cable),
        inputs,
    })
}

/// Paylaşımlı kipte açılmış bir akış.
struct Stream {
    client: IAudioClient,
    size: u32,
}

unsafe fn open(dev: &IMMDevice, channels: u16) -> WinResult<Stream> {
    let client: IAudioClient = dev.Activate(CLSCTX_ALL, None)?;
    client.Initialize(
        AUDCLNT_SHAREMODE_SHARED,
        AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY,
        // 100 ms: Windows bundan küçük de verebilir; doldurma hedefi TARGET_MS.
        1_000_000,
        0,
        &format(channels),
        None,
    )?;
    let size = client.GetBufferSize()?;
    Ok(Stream { client, size })
}

/// Bir çıkışa yazılacak kare sayısı: arabellek hedefe kadar doldurulur.
unsafe fn room(s: &Stream) -> WinResult<usize> {
    let pad = s.client.GetCurrentPadding()?;
    let target = (frames(TARGET_MS) as u32).min(s.size);
    Ok(target.saturating_sub(pad) as usize)
}

unsafe fn write(render: &IAudioRenderClient, mono: &[f32]) -> WinResult<()> {
    if mono.is_empty() {
        return Ok(());
    }
    let n = mono.len();
    let data = render.GetBuffer(n as u32)?;
    let out = std::slice::from_raw_parts_mut(data as *mut i16, n * 2);
    for (i, v) in mono.iter().enumerate() {
        let s = to_i16(*v);
        out[i * 2] = s;
        out[i * 2 + 1] = s;
    }
    render.ReleaseBuffer(n as u32, 0)
}

/// Motor iş parçacığı. Açılış sonucu `ready` ile bildirilir; sonra komutlar bitene ya da aygıt
/// kaybolana kadar döner.
pub fn run(
    cable_id: String,
    mic_id: Option<String>,
    levels: Levels,
    rx: Receiver<Cmd>,
    shared: Shared,
    ready: Sender<Result<(), String>>,
) {
    let result = unsafe { engine(&cable_id, mic_id.as_deref(), levels, &rx, &shared, &ready) };
    shared.alive.store(false, Ordering::SeqCst);
    if let Ok(mut p) = shared.playing.lock() {
        p.clear();
    }
    if let Err(e) = result {
        let msg = format!("Ses aygıtı kayboldu ya da açılamadı: {e}");
        crate::log!("Ses Paneli: {msg}");
        // Açılışta düştüyse bekleyen tarafa da bildirilir.
        let _ = ready.send(Err(msg.clone()));
        *shared.error.lock().unwrap_or_else(|e| e.into_inner()) = Some(msg);
    }
    let _ = shared.app.emit("soundpad-changed", ());
}

unsafe fn engine(
    cable_id: &str,
    mic_id: Option<&str>,
    mut levels: Levels,
    rx: &Receiver<Cmd>,
    shared: &Shared,
    ready: &Sender<Result<(), String>>,
) -> WinResult<()> {
    let en = enumerator()?;
    let cable_dev = en.GetDevice(&HSTRING::from(cable_id))?;
    let cable = open(&cable_dev, 2)?;
    let cable_out: IAudioRenderClient = cable.client.GetService()?;

    // Kulaklık: Windows'un varsayılan çıkışı (kablonun kendisi değilse).
    let monitor = match en.GetDefaultAudioEndpoint(eRender, eConsole) {
        Ok(d) if device_id(&d).ok().as_deref() != Some(cable_id) => match open(&d, 2) {
            Ok(s) => {
                let r: IAudioRenderClient = s.client.GetService()?;
                Some((s, r))
            }
            Err(e) => {
                crate::log!("Ses Paneli: kulaklık açılamadı ({e}); sesler yalnız karşıya gider");
                None
            }
        },
        _ => None,
    };

    // Gerçek mikrofon (izin yoksa ya da yoksa sesler yine de gider).
    let mic = mic_id
        .and_then(|id| en.GetDevice(&HSTRING::from(id)).ok())
        .and_then(|d| match open(&d, 1) {
            Ok(s) => {
                let c: IAudioCaptureClient = s.client.GetService().ok()?;
                Some((s, c))
            }
            Err(e) => {
                crate::log!("Ses Paneli: mikrofon açılamadı ({e}); yalnız sesler gidecek");
                None
            }
        });

    cable.client.Start()?;
    if let Some((s, _)) = &monitor {
        s.client.Start()?;
    }
    if let Some((s, _)) = &mic {
        s.client.Start()?;
    }
    let _ = ready.send(Ok(()));

    let mut voices: Vec<Voice> = Vec::new();
    let mut mic_q: VecDeque<f32> = VecDeque::with_capacity(frames(MIC_MAX_MS) * 2);
    let mut last_playing: Vec<String> = Vec::new();
    let mut buf: Vec<f32> = Vec::with_capacity(frames(TARGET_MS));
    loop {
        // Komutlar.
        loop {
            match rx.try_recv() {
                Ok(Cmd::Play {
                    id,
                    clip,
                    gain,
                    exclusive,
                }) => {
                    if exclusive {
                        voices.clear();
                    } else {
                        voices.retain(|v| v.id != id);
                    }
                    voices.push(Voice {
                        id,
                        clip,
                        gain,
                        pos: [0, 0],
                    });
                }
                Ok(Cmd::Stop(None)) => voices.clear(),
                Ok(Cmd::Stop(Some(id))) => voices.retain(|v| v.id != id),
                Ok(Cmd::Levels(l)) => levels = l,
                Ok(Cmd::Quit) | Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    let _ = cable.client.Stop();
                    if let Some((s, _)) = &monitor {
                        let _ = s.client.Stop();
                    }
                    if let Some((s, _)) = &mic {
                        let _ = s.client.Stop();
                    }
                    return Ok(());
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
            }
        }

        // Mikrofondan gelenler kuyruğa.
        if let Some((_, cap)) = &mic {
            while cap.GetNextPacketSize()? > 0 {
                let mut data: *mut u8 = std::ptr::null_mut();
                let mut n = 0u32;
                let mut flags = 0u32;
                cap.GetBuffer(&mut data, &mut n, &mut flags, None, None)?;
                if levels.passthrough {
                    if flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0 || data.is_null() {
                        mic_q.extend(std::iter::repeat(0.0).take(n as usize));
                    } else {
                        let s = std::slice::from_raw_parts(data as *const i16, n as usize);
                        mic_q.extend(s.iter().map(|v| *v as f32 / 32768.0));
                    }
                }
                cap.ReleaseBuffer(n)?;
            }
            if mic_q.len() > frames(MIC_MAX_MS) {
                let extra = mic_q.len() - frames(MIC_KEEP_MS);
                mic_q.drain(..extra);
            }
            if !levels.passthrough {
                mic_q.clear();
            }
        }

        // Kablo: mikrofon + sesler.
        let n = room(&cable)?;
        if n > 0 {
            buf.clear();
            buf.extend((0..n).map(|_| mic_q.pop_front().unwrap_or(0.0) * levels.mic));
            for v in voices.iter_mut() {
                v.mix_into(0, &mut buf, levels.sound);
            }
            write(&cable_out, &buf)?;
        }

        // Kulaklık: yalnız sesler.
        if let Some((s, r)) = &monitor {
            let n = room(s)?;
            if n > 0 {
                buf.clear();
                buf.resize(n, 0.0);
                let gain = if levels.monitor {
                    levels.monitor_volume
                } else {
                    0.0
                };
                for v in voices.iter_mut() {
                    v.mix_into(1, &mut buf, gain);
                }
                write(r, &buf)?;
            }
        }

        // İki çıkışta da biten sesler atılır (kulaklık yoksa yalnız kablo).
        let has_monitor = monitor.is_some();
        voices.retain(|v| {
            let len = v.clip.samples.read().map(|s| s.len()).unwrap_or(0);
            let done = v.clip.done.load(Ordering::SeqCst);
            let cable_done = done && v.pos[0] >= len;
            let mon_done = !has_monitor || (done && v.pos[1] >= len);
            !(cable_done && mon_done)
        });

        let now: Vec<String> = voices.iter().map(|v| v.id.clone()).collect();
        if now != last_playing {
            if let Ok(mut p) = shared.playing.lock() {
                p.clone_from(&now);
            }
            last_playing = now;
            let _ = shared.app.emit("soundpad-changed", ());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}
