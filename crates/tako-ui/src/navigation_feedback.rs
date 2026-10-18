//! Acessibilidade (UX-001): escala limitada e feedback sonoro local, sem tocar durante a sessão.

use crate::shell::{Cmd, Ctx, Settings};
use sdl3::audio::{AudioFormat, AudioSpec, AudioStreamOwner};
use std::cell::RefCell;

pub const TEXT_SIZE: &str = "TextSizePercent";
pub const SOUND_VOLUME: &str = "NavigationSoundVolume";
pub const TEXT_SIZES: [i32; 3] = [100, 110, 125];

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

fn allowed(ctx: &Ctx) -> bool {
    volume(&ctx.settings) > 0
        && !ctx.simulated()
        && ctx.running.is_none()
        && !ctx.launching
        && !ctx.leaving
}

thread_local! {
    static AUDIO: RefCell<Option<Result<AudioStreamOwner, String>>> = const { RefCell::new(None) };
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

/// Libera também a saída de áudio ao desligar o som ou começar um jogo.
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

pub fn play(ctx: &Ctx, cmd: Cmd) {
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
        // Substituir mantém a fila limitada mesmo com repetição rápida do direcional.
        if let Err(e) = stream
            .clear()
            .and_then(|_| stream.put_data_f32(&tone(cmd, volume(&ctx.settings))))
            .and_then(|_| stream.resume())
        {
            tracing::warn!("som de navegação: {e}");
            *state = Some(Err(e.to_string()));
        }
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
    fn invalid_settings_fall_back_and_tones_are_short_bounded_and_fade_out() {
        let mut settings = Settings::new(None);
        settings.set(TEXT_SIZE, "99999");
        settings.set(SOUND_VOLUME, "255");
        assert_eq!(text_size(&settings), 100);
        assert_eq!(volume(&settings), 0);
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
