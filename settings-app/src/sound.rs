//! Very quiet interface sounds: short, dry ticks made in code and played with Windows' own PlaySound (no files, no extra program running).

use std::f32::consts::TAU;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};
use windows_sys::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY, SND_NODEFAULT};

static ON: AtomicBool = AtomicBool::new(true);

#[derive(Clone, Copy)]
pub enum Sound {
    Click,
    Nav,
    On,
    Off,
    Apply,
}

const RATE: u32 = 44_100;

/// One pen tap: a tiny dry click of filtered noise, and a short hollow knock (a damped tone with an off-key overtone, like plastic on a desk).
fn tick(freq: f32, ms: f32, amplitude: f32, decay: f32) -> Vec<f32> {
    let n = (RATE as f32 * ms / 1000.0) as usize;
    let mut seed = 0x1234_5678u32 ^ (freq as u32).wrapping_mul(2654435761);
    let mut low = 0.0f32;
    (0..n).map(|i| {
        let t = i as f32 / RATE as f32;
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let noise = (seed >> 8) as f32 / 8388608.0 - 1.0;
        low += 0.35 * (noise - low); // soften the hiss: a dull tap, not a tick
        let click = low * (-t * 900.0).exp() * 1.6;
        let knock = (-t * decay * 3.0).exp() * ((TAU * freq * t).sin() + 0.45 * (TAU * freq * 2.76 * t).sin());
        amplitude * (1.0 - (-t * 4000.0).exp()) * (click + 0.8 * knock)
    }).collect()
}

fn wav(samples: &[f32]) -> Vec<u8> {
    let data: Vec<u8> = samples.iter().flat_map(|s| ((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes()).collect();
    let mut out = Vec::with_capacity(44 + data.len());
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&RATE.to_le_bytes());
    out.extend_from_slice(&(RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&data);
    out
}

static VOLUME: AtomicU32 = AtomicU32::new(25);

/// The sounds as samples, made once (at full volume).
fn bank() -> &'static [Vec<f32>; 5] {
    static BANK: OnceLock<[Vec<f32>; 5]> = OnceLock::new();
    BANK.get_or_init(|| {
        let apply: Vec<f32> = tick(520.0, 45.0, 0.30, 55.0).into_iter().chain(std::iter::repeat_n(0.0, 1800)).chain(tick(640.0, 50.0, 0.30, 55.0)).collect();
        [
            tick(560.0, 36.0, 0.30, 90.0),  // click
            tick(720.0, 28.0, 0.24, 110.0), // moving between pages: lighter
            tick(640.0, 40.0, 0.30, 80.0),  // switch on: a little higher
            tick(460.0, 40.0, 0.30, 80.0),  // switch off: a little lower
            apply,
        ]
    })
}

/// The wave file of a sound at the current volume, kept for the life of the program (PlaySound plays from memory and needs it to stay put).
fn rendered(sound: Sound) -> &'static [u8] {
    static CACHE: OnceLock<Mutex<HashMap<(usize, u32), &'static [u8]>>> = OnceLock::new();
    let volume = volume();
    let mut cache = CACHE.get_or_init(Default::default).lock().unwrap();
    *cache.entry((sound as usize, volume)).or_insert_with(|| {
        let gain = volume as f32 / 100.0;
        let scaled: Vec<f32> = bank()[sound as usize].iter().map(|x| x * gain).collect();
        Box::leak(wav(&scaled).into_boxed_slice())
    })
}

pub fn volume() -> u32 {
    VOLUME.load(Ordering::Relaxed)
}

pub fn set_volume(percent: u32) {
    VOLUME.store(percent.min(100), Ordering::Relaxed);
}

pub fn set_enabled(on: bool) {
    ON.store(on, Ordering::Relaxed);
}

pub fn enabled() -> bool {
    ON.load(Ordering::Relaxed)
}

pub fn play(sound: Sound) {
    if !enabled() {
        return;
    }
    let data = rendered(sound);
    unsafe { PlaySoundW(data.as_ptr().cast(), std::ptr::null_mut(), SND_MEMORY | SND_ASYNC | SND_NODEFAULT) };
}
