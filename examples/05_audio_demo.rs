//! 05 - Audio Demo
//!
//! Demonstrates the 4D spatial audio system.
//!
//! This example demonstrates:
//! - Creating an AudioEngine4D
//! - Loading sounds from files
//! - Playing sounds on different buses (Sfx, Music, Ambient)
//! - Spatial audio with 4D positioning
//! - Updating listener position
//! - Bus volume control
//!
//! Run with: `cargo run --example 05_audio_demo`
//!
//! # Note
//!
//! This example requires audio files to be present. Create a `sounds/` directory
//! and add some .ogg or .wav files to test with. Without audio files, the example
//! will demonstrate the API but won't produce sound.

use rust4d_audio::{AudioBus, AudioEngine4D, SpatialConfig};
use rust4d_math::Vec4;
use std::thread;
use std::time::Duration;

fn main() {
    env_logger::init();
    println!("Rust4D Audio Demo");
    println!("=================\n");

    // Create the audio engine
    let engine_result = AudioEngine4D::new();
    let mut engine = match engine_result {
        Ok(e) => {
            println!("[OK] Audio engine initialized");
            e
        }
        Err(e) => {
            eprintln!("[ERROR] Failed to initialize audio engine: {}", e);
            eprintln!("This may be due to missing audio hardware or drivers.");
            eprintln!("The rest of this demo will demonstrate the API without actual audio.\n");
            return;
        }
    };

    // Demonstrate bus volume control
    println!("\n--- Bus Volume Control ---");
    println!("Setting master volume to 80%");
    engine.set_master_volume(0.8);

    println!("Setting SFX volume to 100%");
    engine.set_bus_volume(AudioBus::Sfx, 1.0);

    println!("Setting Music volume to 50%");
    engine.set_bus_volume(AudioBus::Music, 0.5);

    println!("Setting Ambient volume to 30%");
    engine.set_bus_volume(AudioBus::Ambient, 0.3);

    // Try to load some test sounds
    println!("\n--- Loading Sounds ---");

    // Common test sound paths - modify these to match your sound files
    let test_paths = [
        "sounds/beep.ogg",
        "sounds/explosion.ogg",
        "sounds/music.ogg",
        "assets/sounds/test.wav",
    ];

    let mut loaded_sound = None;
    for path in &test_paths {
        match engine.load_sound(path) {
            Ok(handle) => {
                println!("[OK] Loaded sound: {}", path);
                loaded_sound = Some((path.to_string(), handle));
                break;
            }
            Err(e) => {
                println!("[SKIP] {} - {}", path, e);
            }
        }
    }

    // If we loaded a sound, demonstrate playback
    if let Some((path, handle)) = loaded_sound {
        println!("\n--- Playback Demo ---");

        // Non-spatial playback on SFX bus
        println!("Playing '{}' on SFX bus (non-spatial)...", path);
        if let Err(e) = engine.play(&handle, AudioBus::Sfx) {
            eprintln!("  Error: {}", e);
        }
        thread::sleep(Duration::from_millis(500));

        // Spatial playback demo
        println!("\n--- Spatial Audio Demo ---");

        // Set listener at origin
        let listener_pos = Vec4::new(0.0, 0.0, 0.0, 0.0);
        engine.update_listener(listener_pos);
        println!("Listener at: {:?}", listener_pos);

        // Play sound to the right (positive X)
        let right_pos = Vec4::new(10.0, 0.0, 0.0, 0.0);
        println!("Playing sound at {:?} (to the right)...", right_pos);
        let config = SpatialConfig::new(right_pos)
            .with_min_distance(1.0)
            .with_max_distance(50.0);
        if let Err(e) = engine.play_spatial(&handle, config, AudioBus::Sfx) {
            eprintln!("  Error: {}", e);
        }
        thread::sleep(Duration::from_millis(500));

        // Play sound to the left (negative X)
        let left_pos = Vec4::new(-10.0, 0.0, 0.0, 0.0);
        println!("Playing sound at {:?} (to the left)...", left_pos);
        let config = SpatialConfig::new(left_pos);
        if let Err(e) = engine.play_spatial(&handle, config, AudioBus::Sfx) {
            eprintln!("  Error: {}", e);
        }
        thread::sleep(Duration::from_millis(500));

        // Play sound in the 4th dimension (W axis)
        let w_pos = Vec4::new(0.0, 0.0, 0.0, 15.0);
        println!("Playing sound at {:?} (in 4D - W axis)...", w_pos);
        let config = SpatialConfig::new(w_pos);
        if let Err(e) = engine.play_spatial(&handle, config, AudioBus::Sfx) {
            eprintln!("  Error: {}", e);
        }
        thread::sleep(Duration::from_millis(500));

        // Demonstrate moving listener
        println!("\n--- Moving Listener Demo ---");
        for i in 0..5 {
            let new_listener = Vec4::new(i as f32 * 2.0, 0.0, 0.0, 0.0);
            engine.update_listener(new_listener);
            println!("Listener moved to: {:?}", new_listener);

            // Play at fixed position - volume should change as listener moves
            let fixed_pos = Vec4::new(10.0, 0.0, 0.0, 0.0);
            let config = SpatialConfig::new(fixed_pos);
            if let Err(e) = engine.play_spatial(&handle, config, AudioBus::Sfx) {
                eprintln!("  Error: {}", e);
            }
            thread::sleep(Duration::from_millis(300));
        }
    } else {
        println!("\n[INFO] No sound files found. To test audio playback:");
        println!("  1. Create a 'sounds/' directory in the project root");
        println!("  2. Add some .ogg or .wav files");
        println!("  3. Update the test_paths array in this example");
    }

    // Demonstrate the API even without sounds
    println!("\n--- API Summary ---");
    println!("AudioEngine4D methods demonstrated:");
    println!("  - new() -> Create audio engine");
    println!("  - load_sound(path) -> Load sound file");
    println!("  - play(handle, bus) -> Non-spatial playback");
    println!("  - play_spatial(handle, config, bus) -> 4D spatial playback");
    println!("  - play_oneshot(handle, bus) -> One-shot playback");
    println!("  - play_oneshot_spatial(handle, config, bus) -> One-shot spatial");
    println!("  - set_bus_volume(bus, volume) -> Control bus volume");
    println!("  - set_master_volume(volume) -> Control master volume");
    println!("  - update_listener(position) -> Update listener 4D position");
    println!("  - listener_position() -> Get current listener position");
    println!("  - stop_all() -> Stop all sounds (limited implementation)");
    println!("  - stop_bus(bus) -> Stop sounds on a bus (limited implementation)");

    println!("\nAudioBus variants:");
    println!("  - Master (controls all output)");
    println!("  - Sfx (sound effects, default for play())");
    println!("  - Music (background music)");
    println!("  - Ambient (environmental sounds)");

    println!("\nSpatialConfig options:");
    println!("  - new(position) -> Create with 4D position");
    println!("  - with_min_distance(f32) -> Full volume within this range");
    println!("  - with_max_distance(f32) -> Silent beyond this range");

    println!("\nCurrent listener position: {:?}", engine.listener_position());
    println!("\nAudio demo complete!");
}
