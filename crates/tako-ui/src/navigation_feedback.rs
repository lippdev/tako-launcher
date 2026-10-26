//! Acessibilidade (UX-001) e Áudio do Tema (THEME-007): escala limitada, feedback sonoro local
//! e música de fundo por tema com fade e loop na tela cheia, sem tocar durante o jogo.

use crate::shell::{Cmd, Ctx, Settings};
use sdl3::audio::{AudioFormat, AudioSpec, AudioSpecWAV, AudioStreamOwner};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, LazyLock, RwLock};
use std::time::{Duration, Instant};

pub const TEXT_SIZE: &str = "TextSizePercent";
pub const SOUND_VOLUME: &str = "NavigationSoundVolume";
pub const TEXT_SIZES: [i32; 3] = [100, 110, 125];
const MAX_DECODED_MUSIC_BYTES: usize = 128 * 1024 * 1024;

pub const THEME_MUSIC_VOLUME: &str = "ThemeMusicVolume";
pub const THEME_MUSIC_ENABLED: &str = "ThemeMusicEnabled";

pub fn text_size(settings: &Settings) -> i32 {
    settings
        .get(TEXT_SIZE)
        .and_then(|s| s.parse().ok())
        .filter(|n| TEXT_SIZES.contains(n))
        .unwrap_or(100)
}

pub fn volume(settings: &Settings) -> u8 {
    settings
        .get(SOUND_VOLUME)
        .and_then(|s| s.parse().ok())
        .filter(|n| *n <= 100)
        .unwrap_or(0)
}

pub fn music_volume(settings: &Settings) -> u8 {
    settings
        .get(THEME_MUSIC_VOLUME)
        .and_then(|s| s.parse().ok())
        .filter(|n| *n <= 100)
        .unwrap_or(70)
}

pub fn music_enabled(settings: &Settings) -> bool {
    settings.bool(THEME_MUSIC_ENABLED, true)
}

fn allowed(ctx: &Ctx) -> bool {
    volume(&ctx.settings) > 0
        && !ctx.simulated()
        && ctx.running.is_none()
        && !ctx.launching
        && !ctx.leaving
}

/// A música de fundo do tema toca exclusivamente:
/// - Na tela cheia (Big Picture)
/// - Com a janela visível, com foco e sem estar minimizada
/// - Fora de jogos (nunca ao abrir, nem durante o jogo)
/// - Se não houver varredura de jogos (scanning), intro ou tako://play pendente
/// - Se estiver habilitada nas configurações e com volume > 0
pub fn music_allowed(
    ctx: &Ctx,
    is_big_picture: bool,
    window_focused: bool,
    window_minimized: bool,
) -> bool {
    let has_pending_play = matches!(
        ctx.pending_link,
        Some(tako_core::links::Link::Game { play: true, .. })
    );
    is_big_picture
        && window_focused
        && !window_minimized
        && !ctx.simulated()
        && ctx.running.is_none()
        && !ctx.launching
        && !ctx.leaving
        && !ctx.scanning
        && !ctx.intro
        && !has_pending_play
        && music_enabled(&ctx.settings)
        && music_volume(&ctx.settings) > 0
}

#[derive(Default, Clone)]
struct CustomNavSounds {
    accept: Option<Arc<[f32]>>,
    back: Option<Arc<[f32]>>,
    move_sound: Option<Arc<[f32]>>,
}

static CUSTOM_SOUNDS: RwLock<CustomNavSounds> = RwLock::new(CustomNavSounds {
    accept: None,
    back: None,
    move_sound: None,
});

static NAV_GENERATION: AtomicU64 = AtomicU64::new(0);

const TONE_COUNT: usize = 1152;

fn compute_tone_samples(cmd: Cmd) -> [f32; TONE_COUNT] {
    let hz = match cmd {
        Cmd::Accept => 880.,
        Cmd::Back => 440.,
        _ => 660.,
    };
    let mut out = [0.0f32; TONE_COUNT];
    for (i, sample) in out.iter_mut().enumerate() {
        let envelope = (i.min(TONE_COUNT - 1 - i) as f32 / 96.).min(1.);
        let phase = std::f32::consts::TAU * hz * i as f32 / 48_000.;
        *sample = phase.sin() * envelope * 0.12;
    }
    out
}

static DEFAULT_ACCEPT: LazyLock<[f32; TONE_COUNT]> =
    LazyLock::new(|| compute_tone_samples(Cmd::Accept));
static DEFAULT_BACK: LazyLock<[f32; TONE_COUNT]> =
    LazyLock::new(|| compute_tone_samples(Cmd::Back));
static DEFAULT_MOVE: LazyLock<[f32; TONE_COUNT]> =
    LazyLock::new(|| compute_tone_samples(Cmd::Right));

#[cfg(test)]
pub fn tone(cmd: Cmd, volume: u8) -> Vec<f32> {
    let base: &[f32] = match cmd {
        Cmd::Accept => &DEFAULT_ACCEPT[..],
        Cmd::Back => &DEFAULT_BACK[..],
        _ => &DEFAULT_MOVE[..],
    };
    let mult = f32::from(volume.min(100)) / 100.;
    base.iter().map(|s| s * mult).collect()
}

/// Decodifica um arquivo WAV em Rust puro, convertendo para mono 48 kHz f32.
/// Suporta PCM de 8, 16, 24 e 32 bits, além de IEEE float de 32 bits.
/// Não utiliza SDL para garantir execução segura em threads de segundo plano.
pub fn decode_wav_to_mono_48k(bytes: &[u8]) -> Option<Vec<f32>> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return None;
    }
    let mut offset = 12;
    let mut fmt_opt: Option<(u16, u16, u32, u16)> = None; // (format, channels, freq, bits)
    let mut data_opt: Option<(&[u8], usize)> = None;

    while offset + 8 <= bytes.len() {
        let chunk_id = &bytes[offset..offset + 4];
        let chunk_size = u32::from_le_bytes([
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ]) as usize;
        offset += 8;

        if chunk_id == b"fmt " {
            if chunk_size < 16 || offset + 16 > bytes.len() {
                return None;
            }
            let audio_format = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
            let channels = u16::from_le_bytes([bytes[offset + 2], bytes[offset + 3]]);
            let freq = u32::from_le_bytes([
                bytes[offset + 4],
                bytes[offset + 5],
                bytes[offset + 6],
                bytes[offset + 7],
            ]);
            let bits = u16::from_le_bytes([bytes[offset + 14], bytes[offset + 15]]);

            let effective_fmt =
                if audio_format == 0xFFFE && chunk_size >= 40 && offset + 26 <= bytes.len() {
                    u16::from_le_bytes([bytes[offset + 24], bytes[offset + 25]])
                } else {
                    audio_format
                };

            fmt_opt = Some((effective_fmt, channels, freq, bits));
        } else if chunk_id == b"data" {
            let actual_len = chunk_size.min(bytes.len().saturating_sub(offset));
            data_opt = Some((&bytes[offset..offset + actual_len], actual_len));
            if fmt_opt.is_some() {
                break;
            }
        }

        offset += chunk_size + (chunk_size & 1);
    }

    let (format, channels, freq, bits) = fmt_opt?;
    let (data, data_len) = data_opt?;

    if channels == 0 || channels > 8 || freq == 0 || data_len == 0 {
        return None;
    }

    let bytes_per_sample = (bits / 8) as usize;
    if bytes_per_sample == 0 {
        return None;
    }
    let frame_size = (channels as usize) * bytes_per_sample;
    let num_frames = data_len / frame_size;
    // Duração pelos quadros reais, antes de alocar: o `byte_rate` do cabeçalho pode mentir.
    if num_frames == 0 || num_frames as u64 > 2 * freq as u64 {
        return None;
    }

    let mut mono_samples = Vec::with_capacity(num_frames);
    for f in 0..num_frames {
        let f_bytes = &data[f * frame_size..(f + 1) * frame_size];
        let mut sum = 0.0f32;
        for ch in 0..channels as usize {
            let s = &f_bytes[ch * bytes_per_sample..(ch + 1) * bytes_per_sample];
            let val = match (format, bits) {
                (1, 8) => (s[0] as f32 - 128.0) / 128.0,
                (1, 16) => i16::from_le_bytes([s[0], s[1]]) as f32 / 32768.0,
                (1, 24) => {
                    let b = [s[0], s[1], s[2], if s[2] & 0x80 != 0 { 0xFF } else { 0x00 }];
                    i32::from_le_bytes(b) as f32 / 8388608.0
                }
                (1, 32) => i32::from_le_bytes([s[0], s[1], s[2], s[3]]) as f32 / 2147483648.0,
                (3, 32) => f32::from_le_bytes([s[0], s[1], s[2], s[3]]),
                _ => return None,
            };
            sum += val;
        }
        mono_samples.push((sum / channels as f32).clamp(-1.0, 1.0));
    }

    const TARGET_FREQ: u32 = 48_000;
    const MAX_NAV_SAMPLES: usize = TARGET_FREQ as usize * 2; // máx 2 segundos

    let out = if freq == TARGET_FREQ {
        mono_samples
    } else {
        let ratio = freq as f64 / TARGET_FREQ as f64;
        let target_len = ((mono_samples.len() as f64) / ratio).round() as usize;
        let mut resampled = Vec::with_capacity(target_len);
        for i in 0..target_len {
            let src_idx = (i as f64) * ratio;
            let idx0 = (src_idx.floor() as usize).min(mono_samples.len() - 1);
            let idx1 = (idx0 + 1).min(mono_samples.len() - 1);
            let frac = (src_idx - idx0 as f64) as f32;
            resampled.push(mono_samples[idx0] * (1.0 - frac) + mono_samples[idx1] * frac);
        }
        resampled
    };

    if out.is_empty() || out.len() > MAX_NAV_SAMPLES {
        return None;
    }

    Some(out)
}

fn load_wav_samples(path: &Path) -> Option<Vec<f32>> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.is_empty() || bytes.len() > tako_themes::MAX_THEME_MUSIC_BYTES as usize {
        return None;
    }
    let samples = decode_wav_to_mono_48k(&bytes)?;
    Some(samples)
}

thread_local! {
    static AUDIO: RefCell<Option<Result<AudioStreamOwner, String>>> = const { RefCell::new(None) };
    static MUSIC_PLAYER: RefCell<MusicPlayer> = RefCell::new(MusicPlayer::new());
}

fn open_nav_audio() -> Result<AudioStreamOwner, String> {
    let sdl = sdl3::init().map_err(|e| e.to_string())?;
    let audio = sdl.audio().map_err(|e| e.to_string())?;
    let spec = AudioSpec {
        freq: Some(48_000),
        channels: Some(1),
        format: Some(AudioFormat::f32_sys()),
    };
    let device = audio
        .open_playback_device(&spec)
        .map_err(|e| e.to_string())?;
    device
        .open_device_stream(Some(&spec))
        .map_err(|e| e.to_string())
}

/// Libera a saída de áudio ao desligar o som ou começar um jogo.
pub fn sync(ctx: &Ctx) {
    if !allowed(ctx) {
        AUDIO.with(|audio| {
            audio.borrow_mut().take();
        });
    }
}

pub fn play(ctx: &Ctx, cmd: Cmd) {
    // A entrada que só pula a intro não navega, então não faz som.
    if ctx.intro {
        return;
    }
    if !allowed(ctx) {
        sync(ctx);
        return;
    }
    AUDIO.with(|audio| {
        let mut state = audio.borrow_mut();
        if state.is_none() {
            let stream = open_nav_audio();
            if let Err(e) = &stream {
                tracing::warn!("som de navegação indisponível: {e}");
            }
            *state = Some(stream);
        }
        let Some(Ok(stream)) = state.as_ref() else {
            return;
        };

        let gain = f32::from(volume(&ctx.settings).min(100)) / 100.;
        let _ = stream.set_gain(gain);

        let Ok(sounds) = CUSTOM_SOUNDS.read() else {
            return;
        };
        let custom = match cmd {
            Cmd::Accept => sounds.accept.as_deref(),
            Cmd::Back => sounds.back.as_deref(),
            _ => sounds.move_sound.as_deref(),
        };

        let samples: &[f32] = match custom {
            Some(s) => s,
            None => match cmd {
                Cmd::Accept => &DEFAULT_ACCEPT[..],
                Cmd::Back => &DEFAULT_BACK[..],
                _ => &DEFAULT_MOVE[..],
            },
        };

        // Substituir mantém a fila limitada mesmo com repetição rápida do direcional, sem alocar novo Vec.
        if let Err(e) = stream
            .clear()
            .and_then(|_| stream.put_data_f32(samples))
            .and_then(|_| stream.resume())
        {
            tracing::warn!("som de navegação: {e}");
            *state = Some(Err(e.to_string()));
        }
    });
}

#[derive(Clone)]
pub struct DecodedMusic {
    pub freq: i32,
    pub format: AudioFormat,
    pub channels: u8,
    pub buffer: Arc<[u8]>,
}

fn decoded_music_size_allowed(size: usize) -> bool {
    size <= MAX_DECODED_MUSIC_BYTES
}

fn load_music_file(path: &Path) -> Result<DecodedMusic, String> {
    let meta = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if meta.len() == 0 {
        return Err("arquivo vazio".into());
    }
    if meta.len() > tako_themes::MAX_THEME_MUSIC_BYTES {
        return Err(format!(
            "música excede limite de {} MB",
            tako_themes::MAX_THEME_MUSIC_BYTES / (1024 * 1024)
        ));
    }
    let wav = AudioSpecWAV::load_wav(path).map_err(|e| e.to_string())?;
    if !decoded_music_size_allowed(wav.buffer().len()) {
        return Err(format!(
            "música decodificada excede limite de {} MB",
            MAX_DECODED_MUSIC_BYTES / (1024 * 1024)
        ));
    }
    let buffer: Arc<[u8]> = Arc::from(wav.buffer());
    Ok(DecodedMusic {
        freq: wav.freq,
        format: wav.format,
        channels: wav.channels,
        buffer,
    })
}

pub struct MusicPlayer {
    pub music_path: Option<PathBuf>,
    pub music_data: Option<DecodedMusic>,
    loading_rx: Option<Receiver<Result<DecodedMusic, String>>>,
    stream: Option<AudioStreamOwner>,
    pub cursor: usize,
    pub current_gain: f32,
    last_tick: Option<Instant>,
    failed_open_at: Option<Instant>,
    pub failed_path: Option<PathBuf>,
}

fn format_sample_bytes(fmt: AudioFormat) -> usize {
    match fmt {
        AudioFormat::U8 | AudioFormat::S8 => 1,
        AudioFormat::S16LE | AudioFormat::S16BE => 2,
        AudioFormat::S32LE | AudioFormat::S32BE | AudioFormat::F32LE | AudioFormat::F32BE => 4,
        _ => 2,
    }
}

impl MusicPlayer {
    pub fn new() -> Self {
        Self {
            music_path: None,
            music_data: None,
            loading_rx: None,
            stream: None,
            cursor: 0,
            current_gain: 0.0,
            last_tick: None,
            failed_open_at: None,
            failed_path: None,
        }
    }

    pub fn set_theme_music(&mut self, path: Option<PathBuf>) {
        if self.music_path != path {
            self.music_path = path;
            self.music_data = None;
            self.loading_rx = None;
            if let Some(stream) = self.stream.take() {
                let _ = stream.pause();
                let _ = stream.clear();
            }
            self.cursor = 0;
            self.current_gain = 0.0;
            self.failed_open_at = None;
            self.failed_path = None;
        }
    }

    pub fn stop(&mut self) {
        if let Some(stream) = self.stream.take() {
            let _ = stream.pause();
            let _ = stream.clear();
        }
        self.current_gain = 0.0;
        self.cursor = 0;
    }

    fn open_stream(music: &DecodedMusic) -> Result<AudioStreamOwner, String> {
        let sdl = sdl3::init().map_err(|e| e.to_string())?;
        let audio = sdl.audio().map_err(|e| e.to_string())?;
        let spec = AudioSpec {
            freq: Some(music.freq),
            channels: Some(music.channels as i32),
            format: Some(music.format),
        };
        let device = audio
            .open_playback_device(&spec)
            .map_err(|e| e.to_string())?;
        device
            .open_device_stream(Some(&spec))
            .map_err(|e| e.to_string())
    }

    fn feed_stream(cursor: &mut usize, stream: &AudioStreamOwner, music: &DecodedMusic) {
        if music.buffer.is_empty() {
            return;
        }
        let queued = stream.queued_bytes().unwrap_or(0);
        const TARGET_QUEUED_BYTES: i32 = 65_536;
        const CHUNK_SIZE: usize = 32_768;
        let frame_bytes = (music.channels as usize).max(1) * format_sample_bytes(music.format);
        let aligned_chunk = (CHUNK_SIZE - (CHUNK_SIZE % frame_bytes)).max(frame_bytes);
        if queued < TARGET_QUEUED_BYTES {
            let remaining = music.buffer.len() - *cursor;
            let mut chunk_size = aligned_chunk.min(remaining);
            chunk_size -= chunk_size % frame_bytes;
            if chunk_size == 0 {
                *cursor = 0;
            } else {
                let chunk = &music.buffer[*cursor..*cursor + chunk_size];
                let _ = stream.put_data(chunk);
                *cursor += chunk_size;
                if *cursor >= music.buffer.len() {
                    *cursor = 0;
                }
            }
        }
    }

    pub fn tick(
        &mut self,
        ctx: &Ctx,
        is_big_picture: bool,
        window_focused: bool,
        window_minimized: bool,
        now: Instant,
    ) {
        let dt = match self.last_tick {
            Some(last) => now.duration_since(last).as_secs_f32().clamp(0.0, 1.0),
            None => 0.016,
        };
        self.last_tick = Some(now);

        // Recebe música decodificada em segundo plano
        if let Some(rx) = &self.loading_rx {
            match rx.try_recv() {
                Ok(Ok(decoded)) => {
                    self.music_data = Some(decoded);
                    self.loading_rx = None;
                    self.failed_path = None;
                }
                Ok(Err(e)) => {
                    tracing::warn!("música do tema: falha ao carregar WAV: {e}");
                    self.loading_rx = None;
                    self.failed_path = self.music_path.clone();
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.loading_rx = None;
                    self.failed_path = self.music_path.clone();
                }
            }
        }

        let can_play = music_allowed(ctx, is_big_picture, window_focused, window_minimized);

        if can_play && self.music_path.is_some() {
            // Carrega em thread de fundo apenas quando for tocar e não tiver falhado para este caminho
            if self.music_data.is_none()
                && self.loading_rx.is_none()
                && let Some(path) = self.music_path.clone()
                && self.failed_path.as_ref() != Some(&path)
            {
                let (tx, rx) = channel();
                self.loading_rx = Some(rx);
                std::thread::spawn(move || {
                    let res = load_music_file(&path);
                    let _ = tx.send(res);
                });
            }
        }

        let ready_to_play = can_play && self.music_data.is_some();
        let target_gain = if ready_to_play {
            (music_volume(&ctx.settings) as f32 / 100.0).clamp(0.0, 1.0)
        } else {
            0.0
        };

        const FADE_IN_TIME: f32 = 1.0;
        const FADE_OUT_TIME: f32 = 0.8;

        if ready_to_play {
            if self.stream.is_none()
                && let Some(music) = &self.music_data
            {
                let in_backoff = self.failed_open_at.is_some_and(|fail_time| {
                    now.duration_since(fail_time) < Duration::from_secs(2)
                });
                if !in_backoff {
                    match Self::open_stream(music) {
                        Ok(s) => {
                            let _ = s.set_gain(self.current_gain);
                            let _ = s.resume();
                            self.stream = Some(s);
                            self.failed_open_at = None;
                        }
                        Err(e) => {
                            tracing::warn!("música do tema: falha ao abrir stream: {e}");
                            self.failed_open_at = Some(now);
                        }
                    }
                }
            }

            if self.current_gain < target_gain {
                self.current_gain = (self.current_gain + dt / FADE_IN_TIME).min(target_gain);
            } else if self.current_gain > target_gain {
                self.current_gain = (self.current_gain - dt / FADE_OUT_TIME).max(target_gain);
            }

            if let Some(stream) = &self.stream {
                let _ = stream.set_gain(self.current_gain);
                let _ = stream.resume();

                if let Some(music) = &self.music_data {
                    Self::feed_stream(&mut self.cursor, stream, music);
                }
            }
        } else {
            if self.current_gain > 0.0 {
                self.current_gain = (self.current_gain - dt / FADE_OUT_TIME).max(0.0);
                if let Some(stream) = &self.stream {
                    let _ = stream.set_gain(self.current_gain);
                    let _ = stream.resume();

                    // Continua enfileirando blocos durante o fade-out
                    if let Some(music) = &self.music_data {
                        Self::feed_stream(&mut self.cursor, stream, music);
                    }
                }
            }
            if self.current_gain == 0.0 {
                if let Some(stream) = self.stream.take() {
                    let _ = stream.pause();
                    let _ = stream.clear();
                }
                // Quando a música para de vez, recomeça do início
                if !can_play {
                    self.cursor = 0;
                }
            }
        }
    }
}

/// Carrega ou atualiza os sons de navegação e a música de fundo a partir do tema ativo.
pub fn set_theme(theme_dir: Option<&Path>) {
    let audio_paths = theme_dir
        .map(tako_themes::resolve_theme_audio)
        .unwrap_or_default();

    let generation = NAV_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;

    let has_any = audio_paths.accept_sound.is_some()
        || audio_paths.back_sound.is_some()
        || audio_paths.move_sound.is_some();

    if !has_any {
        if let Ok(mut sounds) = CUSTOM_SOUNDS.write() {
            *sounds = CustomNavSounds::default();
        }
    } else {
        let accept_p = audio_paths.accept_sound;
        let back_p = audio_paths.back_sound;
        let move_p = audio_paths.move_sound;

        std::thread::spawn(move || {
            let accept = accept_p
                .as_deref()
                .and_then(load_wav_samples)
                .map(Arc::from);
            let back = back_p.as_deref().and_then(load_wav_samples).map(Arc::from);
            let move_sound = move_p.as_deref().and_then(load_wav_samples).map(Arc::from);

            if let Ok(mut sounds) = CUSTOM_SOUNDS.write()
                && NAV_GENERATION.load(Ordering::SeqCst) == generation
            {
                *sounds = CustomNavSounds {
                    accept,
                    back,
                    move_sound,
                };
            }
        });
    }

    MUSIC_PLAYER.with(|player| {
        player.borrow_mut().set_theme_music(audio_paths.music);
    });
}

/// Para a música de fundo imediatamente (ao fechar o aplicativo).
pub fn stop_theme_music() {
    MUSIC_PLAYER.with(|player| {
        player.borrow_mut().stop();
    });
}

/// Atualiza o ciclo da música de fundo (fade-in, fade-out e loop contínuo).
pub fn tick_theme_music(
    ctx: &Ctx,
    is_big_picture: bool,
    window_focused: bool,
    window_minimized: bool,
    now: Instant,
) {
    MUSIC_PLAYER.with(|player| {
        player
            .borrow_mut()
            .tick(ctx, is_big_picture, window_focused, window_minimized, now);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_game_or_launch_always_suppresses_navigation_audio() {
        let dir = tempfile::tempdir().unwrap();
        let (_guard, mut ctx) = crate::metadata::test_ctx(dir.path(), false);
        ctx.settings.set(SOUND_VOLUME, "100");
        assert!(allowed(&ctx));
        ctx.running = Some(crate::shell::RunningGame {
            id: "game".into(),
            title: "Jogo".into(),
            pid: None,
            started: None,
        });
        assert!(!allowed(&ctx));
        ctx.running = None;
        ctx.launching = true;
        assert!(!allowed(&ctx));
        ctx.launching = false;
        ctx.leaving = true;
        assert!(!allowed(&ctx));
    }

    #[test]
    fn music_is_allowed_only_in_fullscreen_with_focus_and_not_during_games_or_launching() {
        let dir = tempfile::tempdir().unwrap();
        let (_guard, mut ctx) = crate::metadata::test_ctx(dir.path(), false);
        assert_eq!(music_volume(&ctx.settings), 70);
        assert!(music_enabled(&ctx.settings));

        assert!(music_allowed(&ctx, true, true, false));

        // Not allowed if not in Big Picture (desktop mode)
        assert!(!music_allowed(&ctx, false, true, false));

        // Not allowed if lost focus
        assert!(!music_allowed(&ctx, true, false, false));

        // Not allowed if window minimized
        assert!(!music_allowed(&ctx, true, true, true));

        // Not allowed during game launch
        ctx.launching = true;
        assert!(!music_allowed(&ctx, true, true, false));
        ctx.launching = false;

        // Not allowed during game running
        ctx.running = Some(crate::shell::RunningGame {
            id: "game".into(),
            title: "Jogo".into(),
            pid: None,
            started: None,
        });
        assert!(!music_allowed(&ctx, true, true, false));
        // Allowed again when game closes
        ctx.running = None;
        assert!(music_allowed(&ctx, true, true, false));

        // Not allowed if leaving
        ctx.leaving = true;
        assert!(!music_allowed(&ctx, true, true, false));
        ctx.leaving = false;

        // Not allowed if disabled in settings
        ctx.settings.set(THEME_MUSIC_ENABLED, "false");
        assert!(!music_allowed(&ctx, true, true, false));
        ctx.settings.set(THEME_MUSIC_ENABLED, "true");

        // Volume zero sem tocar
        ctx.settings.set(THEME_MUSIC_VOLUME, "0");
        assert_eq!(music_volume(&ctx.settings), 0);
        assert!(!music_allowed(&ctx, true, true, false));
    }

    #[test]
    fn music_allowed_respects_scanning_intro_and_pending_play() {
        let dir = tempfile::tempdir().unwrap();
        let (_guard, mut ctx) = crate::metadata::test_ctx(dir.path(), false);
        assert!(music_allowed(&ctx, true, true, false));

        // Scanning blocks music
        ctx.scanning = true;
        assert!(!music_allowed(&ctx, true, true, false));
        ctx.scanning = false;

        // Intro blocks music
        ctx.intro = true;
        assert!(!music_allowed(&ctx, true, true, false));
        ctx.intro = false;

        // Pending play link blocks music
        ctx.pending_link = Some(tako_core::links::Link::Game {
            id: "game1".into(),
            play: true,
        });
        assert!(!music_allowed(&ctx, true, true, false));

        // Pending non-play link does NOT block music
        ctx.pending_link = Some(tako_core::links::Link::Game {
            id: "game1".into(),
            play: false,
        });
        assert!(music_allowed(&ctx, true, true, false));

        ctx.pending_link = None;
        assert!(music_allowed(&ctx, true, true, false));
    }

    #[test]
    fn window_unfocused_does_not_play_music() {
        let dir = tempfile::tempdir().unwrap();
        let (_guard, ctx) = crate::metadata::test_ctx(dir.path(), false);
        // Sem foco -> false -> não toca
        assert!(!music_allowed(&ctx, true, false, false));
    }

    #[test]
    fn desktop_mode_does_not_play_music() {
        let mut player = MusicPlayer::new();
        let dir = tempfile::tempdir().unwrap();
        let (_guard, ctx) = crate::metadata::test_ctx(dir.path(), false);
        player.music_data = Some(DecodedMusic {
            freq: 44100,
            format: AudioFormat::f32_sys(),
            channels: 2,
            buffer: Arc::from(vec![0u8; 1024]),
        });
        assert!(!music_allowed(&ctx, false, true, false));
        let t0 = Instant::now();
        player.tick(&ctx, false, true, false, t0);
        assert_eq!(player.current_gain, 0.0);
    }

    #[test]
    fn music_fade_logic_progresses_smoothly() {
        let mut player = MusicPlayer::new();
        let dir = tempfile::tempdir().unwrap();
        let (_guard, mut ctx) = crate::metadata::test_ctx(dir.path(), false);
        ctx.settings.set(THEME_MUSIC_VOLUME, "100");

        // Without a wav, can_play is false, gain stays 0
        let t0 = Instant::now();
        player.tick(&ctx, true, true, false, t0);
        assert_eq!(player.current_gain, 0.0);

        // When can_play is true with loaded music, test fade-in
        player.music_data = Some(DecodedMusic {
            freq: 44100,
            format: AudioFormat::f32_sys(),
            channels: 2,
            buffer: Arc::from(vec![0u8; 1024]),
        });

        // Step by 0.5s -> gain increases by 0.5 / 1.0 = 0.5
        let t1 = t0 + Duration::from_millis(500);
        player.tick(&ctx, true, true, false, t1);
        assert!((player.current_gain - 0.5).abs() < 0.05);

        // Another 0.5s -> reaches target gain 1.0
        let t2 = t1 + Duration::from_millis(500);
        player.tick(&ctx, true, true, false, t2);
        assert!((player.current_gain - 1.0).abs() < 0.05);

        // Now test fade-out when can_play becomes false (e.g. game launched)
        ctx.running = Some(crate::shell::RunningGame {
            id: "game".into(),
            title: "Jogo".into(),
            pid: None,
            started: None,
        });

        let t3 = t2 + Duration::from_millis(400); // 0.4s
        player.tick(&ctx, true, true, false, t3);
        // Decrease by 0.4 / 0.8 = 0.5 -> 1.0 - 0.5 = 0.5
        assert!((player.current_gain - 0.5).abs() < 0.05);

        let t4 = t3 + Duration::from_millis(400); // another 0.4s -> 0.0
        player.tick(&ctx, true, true, false, t4);
        assert_eq!(player.current_gain, 0.0);
        assert_eq!(player.cursor, 0); // resets cursor on stop
    }

    #[test]
    fn set_theme_none_clears() {
        set_theme(None);
        let Ok(sounds) = CUSTOM_SOUNDS.read() else {
            return;
        };
        assert!(sounds.accept.is_none());
        assert!(sounds.back.is_none());
        assert!(sounds.move_sound.is_none());
        MUSIC_PLAYER.with(|p| {
            let player = p.borrow();
            assert!(player.music_path.is_none());
            assert!(player.music_data.is_none());
            assert_eq!(player.current_gain, 0.0);
            assert_eq!(player.cursor, 0);
        });
    }

    #[test]
    fn music_player_stop_resets_gain_and_cursor() {
        let mut player = MusicPlayer::new();
        player.current_gain = 0.7;
        player.cursor = 100;
        player.stop();
        assert_eq!(player.current_gain, 0.0);
        assert_eq!(player.cursor, 0);
    }

    #[test]
    fn invalid_settings_fall_back_and_tones_are_short_bounded_and_fade_out() {
        let mut settings = Settings::new(None);
        settings.set(TEXT_SIZE, "99999");
        settings.set(SOUND_VOLUME, "255");
        settings.set(THEME_MUSIC_VOLUME, "999");
        assert_eq!(text_size(&settings), 100);
        assert_eq!(volume(&settings), 0);
        assert_eq!(music_volume(&settings), 70);
        for cmd in [Cmd::Accept, Cmd::Back, Cmd::Right] {
            let samples = tone(cmd, 100);
            assert_eq!(samples.len(), 1152);
            assert_eq!(samples[0], 0.);
            assert_eq!(*samples.last().unwrap(), 0.);
            assert!(samples.iter().all(|s| s.is_finite() && s.abs() <= 0.12));
            assert!(samples.iter().any(|s| s.abs() > 0.1));
            assert!(tone(cmd, 0).iter().all(|s| *s == 0.));
        }
    }
    #[test]
    fn pure_wav_decoder_supports_pcm_and_float_formats() {
        fn make_wav(format: u16, channels: u16, freq: u32, bits: u16, data: &[u8]) -> Vec<u8> {
            let data_len = data.len() as u32;
            let riff_size = 36 + data_len;
            let byte_rate = freq * channels as u32 * (bits as u32 / 8);
            let block_align = channels * (bits / 8);

            let mut v = Vec::new();
            v.extend_from_slice(b"RIFF");
            v.extend_from_slice(&riff_size.to_le_bytes());
            v.extend_from_slice(b"WAVE");
            v.extend_from_slice(b"fmt ");
            v.extend_from_slice(&16u32.to_le_bytes());
            v.extend_from_slice(&format.to_le_bytes());
            v.extend_from_slice(&channels.to_le_bytes());
            v.extend_from_slice(&freq.to_le_bytes());
            v.extend_from_slice(&byte_rate.to_le_bytes());
            v.extend_from_slice(&block_align.to_le_bytes());
            v.extend_from_slice(&bits.to_le_bytes());
            v.extend_from_slice(b"data");
            v.extend_from_slice(&data_len.to_le_bytes());
            v.extend_from_slice(data);
            v
        }

        // 0. `freq` minúsculo com muitos quadros não pode virar uma alocação gigante.
        let huge = make_wav(1, 1, 1, 8, &vec![128u8; 1 << 20]);
        assert!(decode_wav_to_mono_48k(&huge).is_none());

        // 1. PCM 8-bit mono, 48 kHz
        let pcm8_data = [128u8, 255u8, 0u8, 128u8];
        let wav8 = make_wav(1, 1, 48000, 8, &pcm8_data);
        let decoded8 = decode_wav_to_mono_48k(&wav8).expect("failed pcm 8-bit");
        assert_eq!(decoded8.len(), 4);
        assert!((decoded8[0] - 0.0).abs() < 0.01);
        assert!((decoded8[1] - 0.99).abs() < 0.02);
        assert!((decoded8[2] - (-1.0)).abs() < 0.02);

        // 2. PCM 16-bit stereo, 44.1 kHz -> resampled to 48 kHz
        let mut pcm16_data = Vec::new();
        for _ in 0..4410 {
            let left: i16 = 16384; // ~0.5
            let right: i16 = 16384; // ~0.5
            pcm16_data.extend_from_slice(&left.to_le_bytes());
            pcm16_data.extend_from_slice(&right.to_le_bytes());
        }
        let wav16 = make_wav(1, 2, 44100, 16, &pcm16_data);
        let decoded16 = decode_wav_to_mono_48k(&wav16).expect("failed pcm 16-bit");
        assert!((decoded16.len() as i32 - 4800).abs() <= 2);
        assert!((decoded16[100] - 0.5).abs() < 0.01);

        // 3. PCM 24-bit mono, 48 kHz
        let val24 = 0x400000i32;
        let bytes24 = val24.to_le_bytes();
        let wav24 = make_wav(1, 1, 48000, 24, &bytes24[0..3]);
        let decoded24 = decode_wav_to_mono_48k(&wav24).expect("failed pcm 24-bit");
        assert_eq!(decoded24.len(), 1);
        assert!((decoded24[0] - 0.5).abs() < 0.01);

        // 4. PCM 32-bit mono, 48 kHz
        let val32 = 1073741824i32; // 0.5
        let wav32 = make_wav(1, 1, 48000, 32, &val32.to_le_bytes());
        let decoded32 = decode_wav_to_mono_48k(&wav32).expect("failed pcm 32-bit");
        assert_eq!(decoded32.len(), 1);
        assert!((decoded32[0] - 0.5).abs() < 0.01);

        // 5. IEEE Float 32-bit mono, 48 kHz
        let val_f32 = 0.75f32;
        let wav_f32 = make_wav(3, 1, 48000, 32, &val_f32.to_le_bytes());
        let decoded_f32 = decode_wav_to_mono_48k(&wav_f32).expect("failed float 32-bit");
        assert_eq!(decoded_f32.len(), 1);
        assert!((decoded_f32[0] - 0.75).abs() < 0.001);

        // 6. Over 2 seconds -> returns None
        let long_data = vec![0u8; 48000 * 2 * 2 + 100];
        let wav_long = make_wav(1, 1, 48000, 16, &long_data);
        assert!(decode_wav_to_mono_48k(&wav_long).is_none());

        // 7. Invalid header -> returns None
        assert!(decode_wav_to_mono_48k(b"NOT_A_WAV").is_none());
    }

    #[test]
    fn decoded_music_size_limit_accepts_128_mib_and_rejects_larger_buffers() {
        assert!(decoded_music_size_allowed(MAX_DECODED_MUSIC_BYTES));
        assert!(!decoded_music_size_allowed(MAX_DECODED_MUSIC_BYTES + 1));
    }

    #[test]
    fn invalid_music_caches_failed_path_and_prevents_retry_loops() {
        let mut player = MusicPlayer::new();
        let bad_path = PathBuf::from("/path/to/bad/music.wav");
        player.music_path = Some(bad_path.clone());
        assert_eq!(player.failed_path, None);

        let (tx, rx) = std::sync::mpsc::channel();
        player.loading_rx = Some(rx);
        tx.send(Err("arquivo corrompido".into())).unwrap();

        let dir = tempfile::tempdir().unwrap();
        let (_guard, ctx) = crate::metadata::test_ctx(dir.path(), false);
        let t0 = Instant::now();
        player.tick(&ctx, true, true, false, t0);

        assert!(player.loading_rx.is_none());
        assert_eq!(player.failed_path, Some(bad_path.clone()));

        let t1 = t0 + Duration::from_millis(16);
        player.tick(&ctx, true, true, false, t1);
        assert!(player.loading_rx.is_none());

        player.set_theme_music(Some(PathBuf::from("/path/to/new_music.wav")));
        assert_eq!(player.failed_path, None);
    }

    #[test]
    fn fade_out_continues_enqueuing_blocks_while_gain_above_zero() {
        let mut player = MusicPlayer::new();
        let dir = tempfile::tempdir().unwrap();
        let (_guard, ctx) = crate::metadata::test_ctx(dir.path(), false);

        let buf = vec![0u8; 65536];
        player.music_data = Some(DecodedMusic {
            freq: 44100,
            format: AudioFormat::S16LE,
            channels: 2,
            buffer: Arc::from(buf),
        });
        player.current_gain = 1.0;
        player.cursor = 1000;

        let t0 = Instant::now();
        let t1 = t0 + Duration::from_millis(200);
        player.last_tick = Some(t0);
        player.tick(&ctx, false, false, false, t1);

        assert!(player.current_gain > 0.0);
        assert_eq!(player.cursor, 1000);

        let t2 = t1 + Duration::from_millis(700);
        player.tick(&ctx, false, false, false, t2);
        assert_eq!(player.current_gain, 0.0);
        assert_eq!(player.cursor, 0);
    }
}
