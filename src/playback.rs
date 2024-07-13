// src/playback.rs
use log::{debug, info};
use tokio::sync::{mpsc, Mutex};
use std::sync::Arc;
use tokio::time::{self, Duration};
use std::error::Error;

use crate::common::*;
use crate::midi::MidiHandler;
use crate::sequencers::euclidean::EuclideanSequencerInput;
use crate::sequencers::markov::MarkovSequencerInput;

pub async fn start_playback_loop(
    mut midi_handler: MidiHandler,
    _tx: mpsc::Sender<Input>,
    mut rx: mpsc::Receiver<Input>,
    shared_state: Arc<Mutex<SharedState>>
) -> Result<(), Box<dyn Error>> {
    info!("Starting playback loop");
    tokio::spawn(async move {
        loop {
            if let Some(input) = rx.recv().await {
                let mut state = shared_state.lock().await;
                match input {
                    Input::Bpm(bpm) => {
                        info!("Changing BPM to {}", bpm);
                        state.bpm = bpm;
                    },
                    Input::Sequence(sequence) => {
                        info!("Changing sequence");
                        state.sequence = sequence;
                    },
                    Input::Shutdown => {
                        info!("Shutting down playback loop");
                        midi_handler.send_all_notes_off().expect("Failed to send all notes off");
                    },
                    Input::TogglePlayback => {
                        info!("Toggling playback");
                        state.playing = !state.playing;
                    },
                    Input::IncreaseBpm => state.bpm += 1.0,
                    Input::DecreaseBpm => state.bpm -= 1.0,
                    Input::Euclidean(euclidean_input) => {
                        match euclidean_input {
                            EuclideanSequencerInput::IncreaseSteps => {
                                // Handle increasing steps in Euclidean sequencer
                            },
                            EuclideanSequencerInput::DecreaseSteps => {
                                // Handle decreasing steps in Euclidean sequencer
                            },
                            // Handle other Euclidean inputs...
                            _ => {}
                        }
                    },
                    Input::Markov(markov_input) => {
                        match markov_input {
                            _ => {}
                        }
                    },
                }
            }

            let state = shared_state.lock().await;
            if !state.playing {
                time::sleep(Duration::from_millis(10)).await;
                continue;
            }

            debug!("Playing {:?}", state);
            for i in 0..state.sequence.notes.len() {
                let pitch = state.sequence.notes[i].pitch;
                let duration = Duration::from_millis(state.sequence.notes[i].duration as u64);
                let velocity = state.sequence.notes[i].velocity;
                midi_handler.send_note_on(pitch, velocity).expect("Failed to send NOTE ON");
                time::sleep(duration).await;
                midi_handler.send_note_off(pitch).expect("Failed to send NOTE OFF");
            }
        }
    });

    Ok(())
}
