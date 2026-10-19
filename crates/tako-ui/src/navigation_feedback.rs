//! Acessibilidade (UX-001) e Áudio do Tema (THEME-007): escala limitada, feedback sonoro local
//! e música de fundo por tema com fade e loop na tela cheia, sem tocar durante o jogo.

use crate::shell::{Cmd, Ctx, Settings};
use sdl3::audio::{AudioFormat, AudioSpec, AudioSpecWAV, AudioStreamOwner};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::time::Instant;

pub const TEXT_SIZE: &str = "TextSizePercent";
pub const SOUND_VOLUME: &str = "NavigationSoundVolume";
pub const TEXT_SIZES: [i32; 3] = [100, 110, 125];

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
/// - Se estiver habilitada nas configurações e com volume > 0
pub fn music_allowed(
    ctx: &Ctx,
    is_big_picture: bool,
    window_focused: bool,
    window_minimized: bool,
) -> bool {
    is_big_picture
        && window_focused
        && !window_minimized
        && !ctx.simulated()
        && ctx.running.is_none()
        && !ctx.launching
        && !ctx.leaving
        && music_enabled(&ctx.settings)
        && music_volume(&ctx.settings) > 0
}

#[derive(Default)]
struct CustomNavSounds {
    accept: Option<Vec<f32>>,
    back: Option<Vec<f32>>,
    move_sound: Option<Vec<f32>>,
}

thread_local! {
    static AUDIO: RefCell<Option<Result<AudioStreamOwner, String>>> = const { RefCell::new(None) };
    static CUSTOM_SOUNDS: RefCell<CustomNavSounds> = RefCell::new(CustomNavSounds::default());
    static MUSIC_PLAYER: RefCell<MusicPlayer> = const { RefCell::new(MusicPlayer::new()) };
}

fn load_wav_samples(path: &Path) -> Option<Vec<f32>> {
    let wav = AudioSpecWAV::load_wav(path).ok()?;
    let sdl = sdl3::init().ok()?;
    let audio = sdl.audio().ok()?;
    let src_spec = AudioSpec {
        freq: Some(wav.freq),
        channels: Some(wav.channels as i32),
        format: Some(wav.format),
    };
    let dst_spec = AudioSpec {
        freq: Some(48_000),
        channels: Some(1),
        format: Some(AudioFormat::f32_sys()),
    };
    let mut converter = audio.new_stream(Some(&src_spec), Some(&dst_spec)).ok()?;
    converter.put_data(wav.buffer()).ok()?;
    converter.flush().ok()?;
    let avail = converter.available_bytes().ok()?;
    if avail <= 0 {
        return None;
    }
    let count = (avail as usize) / std::mem::size_of::<f32>();
    let mut out = vec![0.0f32; count];
    converter.read_f32_samples(&mut out).ok()?;
    Some(out)
}

fn open() -> Result<AudioStreamOwner, String> {
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

fn tone(cmd: Cmd, volume: u8) -> Vec<f32> {
    let hz = match cmd {
        Cmd::Accept => 880.,
        Cmd::Back => 440.,
        _ => 660.,
    };
    let count = 1152;
    (0..count)
        .map(|i| {
            let envelope = (i.min(count - 1 - i) as f32 / 96.).min(1.);
            let phase = std::f32::consts::TAU * hz * i as f32 / 48_000.;
            phase.sin() * envelope * 0.12 * f32::from(volume.min(100)) / 100.
        })
        .collect()
}

fn sound_samples(cmd: Cmd, volume: u8) -> Vec<f32> {
    CUSTOM_SOUNDS.with(|cell| {
        let sounds = cell.borrow();
        let custom = match cmd {
            Cmd::Accept => sounds.accept.as_ref(),
            Cmd::Back => sounds.back.as_ref(),
            _ => sounds.move_sound.as_ref(),
        };
        if let Some(samples) = custom {
            let mult = f32::from(volume.min(100)) / 100.;
            samples.iter().map(|s| s * mult).collect()
        } else {
            tone(cmd, volume)
        }
    })
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
            let stream = open();
            if let Err(e) = &stream {
                tracing::warn!("som de navegação indisponível: {e}");
            }
            *state = Some(stream);
        }
        let Some(Ok(stream)) = state.as_ref() else {
            return;
        };
        let samples = sound_samples(cmd, volume(&ctx.settings));
        // Substituir mantém a fila limitada mesmo com repetição rápida do direcional.
        if let Err(e) = stream
            .clear()
            .and_then(|_| stream.put_data_f32(&samples))
            .and_then(|_| stream.resume())
        {
            tracing::warn!("som de navegação: {e}");
            *state = Some(Err(e.to_string()));
        }
    });
}

struct MusicPlayer {
    music_path: Option<PathBuf>,
    wav: Option<AudioSpecWAV>,
    stream: Option<AudioStreamOwner>,
    current_gain: f32,
    last_tick: Option<Instant>,
}

impl MusicPlayer {
    const fn new() -> Self {
        Self {
            music_path: None,
            wav: None,
            stream: None,
            current_gain: 0.0,
            last_tick: None,
        }
    }

    fn set_theme_music(&mut self, path: Option<PathBuf>) {
        if self.music_path != path {
            self.music_path = path.clone();
            self.stream = None;
            self.current_gain = 0.0;
            self.wav = path.and_then(|p| match AudioSpecWAV::load_wav(&p) {
                Ok(w) => Some(w),
                Err(e) => {
                    tracing::warn!("música do tema: falha ao carregar WAV: {e}");
                    None
                }
            });
        }
    }

    fn open_stream(wav: &AudioSpecWAV) -> Result<AudioStreamOwner, String> {
        let sdl = sdl3::init().map_err(|e| e.to_string())?;
        let audio = sdl.audio().map_err(|e| e.to_string())?;
        let spec = AudioSpec {
            freq: Some(wav.freq),
            channels: Some(wav.channels as i32),
            format: Some(wav.format),
        };
        let device = audio
            .open_playback_device(&spec)
            .map_err(|e| e.to_string())?;
        device
            .open_device_stream(Some(&spec))
            .map_err(|e| e.to_string())
    }

    fn tick(
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

        let can_play = self.wav.is_some()
            && music_allowed(ctx, is_big_picture, window_focused, window_minimized);

        let target_gain = if can_play {
            (music_volume(&ctx.settings) as f32 / 100.0).clamp(0.0, 1.0)
        } else {
            0.0
        };

        const FADE_IN_TIME: f32 = 1.0;
        const FADE_OUT_TIME: f32 = 0.8;

        if can_play {
            if self.stream.is_none()
                && let Some(wav) = &self.wav
            {
                match Self::open_stream(wav) {
                    Ok(s) => {
                        let _ = s.set_gain(self.current_gain);
                        let _ = s.put_data(wav.buffer());
                        let _ = s.resume();
                        self.stream = Some(s);
                    }
                    Err(e) => {
                        tracing::warn!("música do tema: falha ao abrir stream: {e}");
                        return;
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

                if let Some(wav) = &self.wav {
                    let queued = stream.queued_bytes().unwrap_or(0);
                    let threshold = ((wav.buffer().len() / 2) as i32).max(10_000);
                    if queued < threshold {
                        let _ = stream.put_data(wav.buffer());
                    }
                }
            }
        } else {
            if self.current_gain > 0.0 {
                self.current_gain = (self.current_gain - dt / FADE_OUT_TIME).max(0.0);
                if let Some(stream) = &self.stream {
                    let _ = stream.set_gain(self.current_gain);
                }
            }
            if self.current_gain == 0.0
                && let Some(stream) = self.stream.take()
            {
                let _ = stream.pause();
                let _ = stream.clear();
            }
        }
    }
}

/// Carrega ou atualiza os sons de navegação e a música de fundo a partir do tema ativo.
pub fn set_theme(theme_dir: Option<&Path>) {
    let audio_paths = theme_dir
        .map(tako_themes::resolve_theme_audio)
        .unwrap_or_default();

    CUSTOM_SOUNDS.with(|cell| {
        let mut sounds = cell.borrow_mut();
        sounds.accept = audio_paths
            .accept_sound
            .as_deref()
            .and_then(load_wav_samples);
        sounds.back = audio_paths.back_sound.as_deref().and_then(load_wav_samples);
        sounds.move_sound = audio_paths.move_sound.as_deref().and_then(load_wav_samples);
    });

    MUSIC_PLAYER.with(|player| {
        player.borrow_mut().set_theme_music(audio_paths.music);
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
        // By default, music is enabled with default volume 70
        assert_eq!(music_volume(&ctx.settings), 70);
        assert!(music_enabled(&ctx.settings));

        // When in Big Picture, focused and not minimized, without simulation, it's allowed
        // (test_ctx creates simulated: false by default when pass false)
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
    fn music_fade_logic_progresses_smoothly() {
        let mut player = MusicPlayer::new();
        let dir = tempfile::tempdir().unwrap();
        let (_guard, mut ctx) = crate::metadata::test_ctx(dir.path(), false);
        ctx.settings.set(THEME_MUSIC_VOLUME, "100");

        // Without a wav, can_play is false, gain stays 0
        let t0 = Instant::now();
        player.tick(&ctx, true, true, false, t0);
        assert_eq!(player.current_gain, 0.0);

        // When fading out from an active gain:
        player.current_gain = 0.8;
        let t1 = t0 + std::time::Duration::from_millis(400); // 0.4s
        player.tick(&ctx, true, true, false, t1); // can_play is still false without wav
        // Current gain should have decreased by 0.4 / 0.8 = 0.5 -> ~0.3
        assert!((player.current_gain - 0.3).abs() < 0.05);

        // Another 0.4s -> 0.0
        let t2 = t1 + std::time::Duration::from_millis(400);
        player.tick(&ctx, true, true, false, t2);
        assert_eq!(player.current_gain, 0.0);
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
}
