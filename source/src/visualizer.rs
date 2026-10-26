//! Real WASAPI shared-mode loopback, 24 spectrum bands, maximum 60 analysis frames/second.
//! Start only while the visualizer is visible and music is playing. Never captures microphone input.
use std::{
    sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}, mpsc::{self, Sender, Receiver}},
    thread::{self, JoinHandle}, time::{Duration, Instant},
};
use windows::{
    core::{GUID, PCWSTR},
    Win32::{
        Foundation::{CloseHandle, HANDLE, WAIT_FAILED},
        Media::Audio::{IMMDeviceEnumerator, MMDeviceEnumerator, IAudioClient, IAudioCaptureClient,
            WAVEFORMATEX, eRender, eMultimedia, AUDCLNT_SHAREMODE_SHARED,
            AUDCLNT_STREAMFLAGS_LOOPBACK, AUDCLNT_STREAMFLAGS_EVENTCALLBACK},
        System::{Com::{CoInitializeEx, CoUninitialize, CoCreateInstance, CoTaskMemFree, COINIT_MULTITHREADED, CLSCTX_ALL},
            Threading::{CreateEventW, WaitForSingleObject}},
    },
};
const FFT_SIZE: usize = 2048;
const BANDS: usize = 24;
static CAPTURE_OWNED: AtomicBool = AtomicBool::new(false);
struct CaptureLease;
impl Drop for CaptureLease { fn drop(&mut self) { CAPTURE_OWNED.store(false, Ordering::Release); } }

pub struct Visualizer {
    cancelled: Arc<AtomicBool>, error: Arc<Mutex<Option<String>>>,
    worker: Option<JoinHandle<()>>, done: Receiver<()>,
}
impl Visualizer {
    pub fn start(tx: Sender<Vec<f32>>) -> Result<Self, String> {
        if CAPTURE_OWNED.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).is_err() {
            return Err("Previous audio capture is still stopping".into());
        }
        let cancelled = Arc::new(AtomicBool::new(false)); let cancel = cancelled.clone();
        let error = Arc::new(Mutex::new(None)); let errors = error.clone();
        let (ready_tx, ready_rx) = mpsc::channel(); let (done_tx, done) = mpsc::channel();
        let worker = thread::Builder::new().name("notch-wasapi".into()).spawn(move || {
            let _capture_lease = CaptureLease;
            let result = capture_loop(&tx, &cancel, &ready_tx);
            if let Err(message) = result {
                if !cancel.load(Ordering::Acquire) {
                    if let Ok(mut target) = errors.lock() { *target = Some(message.clone()); }
                    let _ = ready_tx.send(Err(message));
                }
            }
            let _ = tx.send(vec![0.0; BANDS]); let _ = done_tx.send(());
        }).map_err(|e| { CAPTURE_OWNED.store(false, Ordering::Release); e.to_string() })?;
        match ready_rx.recv_timeout(Duration::from_secs(3)) {
            Ok(Ok(())) => Ok(Self { cancelled, error, worker: Some(worker), done }),
            result => {
                cancelled.store(true, Ordering::Release);
                if worker.is_finished() { let _ = worker.join(); }
                Err(match result { Ok(Err(e)) => e, _ => "WASAPI initialization timed out".into() })
            }
        }
    }
    pub fn last_error(&self) -> Option<String> { self.error.lock().ok().and_then(|e| e.clone()) }
    pub fn is_running(&self) -> bool {
        !self.cancelled.load(Ordering::Acquire) && self.worker.as_ref().is_some_and(|w| !w.is_finished())
    }
    /// Bounded wait: an unresponsive audio driver must not hold up the native UI indefinitely.
    pub fn stop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
        let _ = self.done.recv_timeout(Duration::from_millis(200));
        if let Some(worker) = self.worker.take() { if worker.is_finished() { let _ = worker.join(); } }
    }
}
impl Drop for Visualizer { fn drop(&mut self) { self.stop(); } }

struct Apartment;
impl Apartment {
    fn new() -> windows::core::Result<Self> {
        unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok()?; } Ok(Self)
    }
}
impl Drop for Apartment { fn drop(&mut self) { unsafe { CoUninitialize(); } } }
struct Event(HANDLE);
impl Drop for Event { fn drop(&mut self) { unsafe { let _ = CloseHandle(self.0); } } }
struct FormatMemory(*mut WAVEFORMATEX);
impl Drop for FormatMemory { fn drop(&mut self) { unsafe { CoTaskMemFree(Some(self.0.cast())); } } }
struct StartedAudio(IAudioClient);
impl Drop for StartedAudio { fn drop(&mut self) { unsafe { let _ = self.0.Stop(); } } }

#[derive(Clone, Copy, Debug)]
struct Format { channels: usize, rate: u32, align: usize, bytes: usize, floating: bool }
fn parse_format(bytes: &[u8]) -> Result<Format, String> {
    if bytes.len() < 18 { return Err("Incomplete WAVEFORMATEX".into()); }
    let word = |i| u16::from_le_bytes([bytes[i], bytes[i+1]]);
    let mut tag = word(0); let channels = word(2) as usize;
    let rate = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    let align = word(12) as usize; let bits = word(14) as usize;
    if tag == 0xfffe {
        if word(16) < 22 || bytes.len() < 40 { return Err("Incomplete WAVEFORMATEXTENSIBLE".into()); }
        let subformat = GUID::from_values(u32::from_le_bytes(bytes[24..28].try_into().unwrap()),
            word(28), word(30), bytes[32..40].try_into().unwrap());
        if subformat == GUID::from_u128(0x00000003_0000_0010_8000_00aa00389b71) { tag = 3; }
        else if subformat == GUID::from_u128(0x00000001_0000_0010_8000_00aa00389b71) { tag = 1; }
        else { return Err(format!("Unsupported audio subformat: {subformat:?}")); }
    }
    if !(1..=32).contains(&channels) || !(8000..=384000).contains(&rate) ||
        !((tag == 3 && bits == 32) || (tag == 1 && [16,24,32].contains(&bits))) || align < channels * (bits / 8) {
        return Err(format!("Unsupported mix format (tag={tag}, bits={bits}, channels={channels}, rate={rate})"));
    }
    Ok(Format { channels, rate, align, bytes: bits / 8, floating: tag == 3 })
}
fn decode_sample(bytes: &[u8], floating: bool) -> f32 {
    let value = if floating { f32::from_le_bytes(bytes[..4].try_into().unwrap()) }
    else { match bytes.len() {
        2 => i16::from_le_bytes(bytes.try_into().unwrap()) as f32 / 32768.0,
        3 => ((i32::from(bytes[0]) | i32::from(bytes[1]) << 8 | i32::from(bytes[2]) << 16) << 8 >> 8) as f32 / 8388608.0,
        4 => i32::from_le_bytes(bytes.try_into().unwrap()) as f32 / 2147483648.0,
        _ => 0.0,
    }};
    if value.is_finite() { value.clamp(-1.0, 1.0) } else { 0.0 }
}

fn capture_loop(tx: &Sender<Vec<f32>>, cancel: &AtomicBool, ready: &Sender<Result<(), String>>) -> Result<(), String> {
    let _apartment = Apartment::new().map_err(|e| e.to_string())?;
    unsafe {
        let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|e| e.to_string())?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia).map_err(|e| e.to_string())?;
        let audio: IAudioClient = device.Activate(CLSCTX_ALL, None).map_err(|e| e.to_string())?;
        let memory = FormatMemory(audio.GetMixFormat().map_err(|e| e.to_string())?);
        // WASAPI owns a complete CoTaskMem WAVEFORMATEX plus cbSize extra bytes.
        let header = std::slice::from_raw_parts(memory.0.cast::<u8>(), 18);
        let extra = u16::from_le_bytes([header[16], header[17]]) as usize;
        let format = parse_format(std::slice::from_raw_parts(memory.0.cast::<u8>(), 18 + extra))?;
        audio.Initialize(AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK | AUDCLNT_STREAMFLAGS_EVENTCALLBACK,
            0, 0, memory.0, None).map_err(|e| e.to_string())?;
        let event = Event(CreateEventW(None, false, false, PCWSTR::null()).map_err(|e| e.to_string())?);
        audio.SetEventHandle(event.0).map_err(|e| e.to_string())?;
        let capture: IAudioCaptureClient = audio.GetService().map_err(|e| e.to_string())?;
        if cancel.load(Ordering::Acquire) { return Ok(()); }
        audio.Start().map_err(|e| e.to_string())?;
        let _started = StartedAudio(audio.clone());
        let _ = ready.send(Ok(()));
        let mut ring = [0.0; FFT_SIZE]; let mut cursor = 0usize;
        let mut previous = vec![0.0; BANDS];
        let clock = Instant::now(); let mut next_frame = Duration::ZERO; let mut last_packet = Instant::now();
        while !cancel.load(Ordering::Acquire) {
            if WaitForSingleObject(event.0, 20) == WAIT_FAILED { return Err("WASAPI event wait failed".into()); }
            let mut available = capture.GetNextPacketSize().map_err(|e| e.to_string())?;
            while available > 0 && !cancel.load(Ordering::Acquire) {
                let mut pointer = std::ptr::null_mut(); let mut frames = 0; let mut flags = 0;
                capture.GetBuffer(&mut pointer, &mut frames, &mut flags, None, None).map_err(|e| e.to_string())?;
                let result = (|| -> Result<(), String> {
                    let size = (frames as usize).checked_mul(format.align).ok_or("Audio packet overflow")?;
                    if size > 16 * 1024 * 1024 { return Err("Audio packet too large".into()); }
                    let silent = flags & 2 != 0;
                    if !silent && size > 0 && pointer.is_null() { return Err("Missing audio packet buffer".into()); }
                    let bytes = if silent || size == 0 { &[][..] } else { std::slice::from_raw_parts(pointer, size) };
                    if flags & 1 != 0 { ring.fill(0.0); }
                    for frame in 0..frames as usize {
                        let mut mono = 0.0;
                        if !silent {
                            for channel in 0..format.channels {
                                let start = frame * format.align + channel * format.bytes;
                                mono += decode_sample(&bytes[start..start+format.bytes], format.floating);
                            }
                            mono /= format.channels as f32;
                        }
                        ring[cursor] = mono; cursor = (cursor + 1) & (FFT_SIZE - 1);
                    }
                    Ok(())
                })();
                let released = capture.ReleaseBuffer(frames);
                result?; released.map_err(|e| e.to_string())?;
                last_packet = Instant::now(); available = capture.GetNextPacketSize().map_err(|e| e.to_string())?;
            }
            if clock.elapsed() < next_frame || cancel.load(Ordering::Acquire) { continue; }
            next_frame = clock.elapsed() + Duration::from_nanos(1_000_000_000 / 60 + 1);
            if last_packet.elapsed() > Duration::from_millis(150) { ring.fill(0.0); }
            previous = spectrum(&ring, cursor, format.rate, &previous);
            if tx.send(previous.clone()).is_err() { break; }
        }
    }
    Ok(())
}

fn transform(values: &mut [(f64, f64)]) {
    let n = values.len(); let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 { j ^= bit; bit >>= 1; }
        j ^= bit; if i < j { values.swap(i, j); }
    }
    let mut size = 2;
    while size <= n {
        let angle = -2.0 * std::f64::consts::PI / size as f64;
        let root = (angle.cos(), angle.sin());
        for start in (0..n).step_by(size) {
            let mut factor = (1.0, 0.0);
            for i in 0..size/2 {
                let a = values[start+i]; let v = values[start+i+size/2];
                let b = (v.0*factor.0-v.1*factor.1, v.0*factor.1+v.1*factor.0);
                values[start+i] = (a.0+b.0, a.1+b.1); values[start+i+size/2] = (a.0-b.0, a.1-b.1);
                factor = (factor.0*root.0-factor.1*root.1, factor.0*root.1+factor.1*root.0);
            }
        }
        size <<= 1;
    }
}
fn spectrum(ring: &[f32; FFT_SIZE], cursor: usize, sample_rate: u32, previous: &[f32]) -> Vec<f32> {
    let mut values = [(0.0, 0.0); FFT_SIZE];
    for (i, value) in values.iter_mut().enumerate() {
        let window = 0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / (FFT_SIZE-1) as f64).cos();
        value.0 = f64::from(ring[(cursor+i) & (FFT_SIZE-1)]) * window;
    }
    transform(&mut values);
    let upper = 18_000f64.min(sample_rate as f64 / 2.0);
    (0..BANDS).map(|band| {
        let low = 40.0 * (upper / 40.0).powf(band as f64 / BANDS as f64);
        let high = 40.0 * (upper / 40.0).powf((band+1) as f64 / BANDS as f64);
        let first = ((low * FFT_SIZE as f64 / sample_rate as f64) as usize).clamp(1, FFT_SIZE/2-1);
        let last = ((high * FFT_SIZE as f64 / sample_rate as f64).ceil() as usize).clamp(first+1, FFT_SIZE/2);
        let peak = values[first..last].iter().map(|(re,im)| re.hypot(*im) * 4.0 / FFT_SIZE as f64).fold(0.0, f64::max);
        let normalized = ((20.0 * peak.max(1e-8).log10() + 70.0) / 70.0).clamp(0.0,1.0) as f32;
        let old = previous.get(band).copied().unwrap_or_default();
        if normalized >= old { normalized } else { old * 0.87 + normalized * 0.13 }
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "opt-in real WASAPI loopback smoke test; no microphone, playback, gain or routing changes"]
    fn live_capture_stops_and_restarts_without_lingering_worker(){
        for _ in 0..3{
            let(tx,rx)=mpsc::channel();let mut capture=Visualizer::start(tx).expect("WASAPI startup");
            assert!(capture.is_running());let frame=rx.recv_timeout(Duration::from_secs(1)).expect("spectrum frame");assert_eq!(frame.len(),BANDS);
            let stop=Instant::now();capture.stop();
            while CAPTURE_OWNED.load(Ordering::Acquire)&&stop.elapsed()<Duration::from_secs(1){thread::sleep(Duration::from_millis(5));}
            assert!(!CAPTURE_OWNED.load(Ordering::Acquire),"capture lease leaked after stop");assert!(!capture.is_running());
            while rx.try_recv().is_ok(){}assert!(rx.recv_timeout(Duration::from_millis(80)).is_err());assert!(capture.last_error().is_none());
        }
    }
    #[test] fn signed_pcm_and_nan_are_handled() {
        assert_eq!(decode_sample(&[0,128], false), -1.0);
        assert_eq!(decode_sample(&[0,0,128], false), -1.0);
        assert_eq!(decode_sample(&[0,0,0,128], false), -1.0);
        assert_eq!(decode_sample(&f32::NAN.to_le_bytes(), true), 0.0);
        assert!(decode_sample(&[255,127], false) > 0.999);
    }
    #[test] fn silence_has_zero_spectrum_and_sine_has_energy() {
        let mut ring = [0.0; FFT_SIZE]; let silent = spectrum(&ring,0,48000,&[0.0; BANDS]);
        assert!(silent.iter().all(|x| *x == 0.0));
        for (i, x) in ring.iter_mut().enumerate() { *x = (2.0*std::f32::consts::PI*1000.0*i as f32/48000.0).sin() * 0.5; }
        let real = spectrum(&ring,0,48000,&[0.0; BANDS]);
        let (peak, _) = real.iter().enumerate().max_by(|a,b|a.1.total_cmp(b.1)).unwrap();
        assert!((11..=14).contains(&peak), "unexpected 1kHz band: {peak}");
        assert!(real[peak] > 0.8);
        assert!(real.iter().all(|x| x.is_finite() && (0.0..=1.0).contains(x)));
    }
    #[test] fn unsupported_format_is_rejected() {
        assert!(parse_format(&[0; 18]).is_err());
        assert!(parse_format(&[]).is_err());
    }
}
