use crate::note::{Note, NoteDuration, Sequence};
use crate::sequencers::euclidean::config::EuclideanSequencerConfig;
use crate::sequencers::traits::Sequencer;

use crate::state::SharedState;
use anyhow::Error;
use log::debug;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc;

pub struct EuclideanSequencer {
    config: EuclideanSequencerConfig,
    config_rx: mpsc::Receiver<EuclideanSequencerConfig>,
    mixer_update_tx: mpsc::Sender<()>,
    shared_state: Arc<Mutex<SharedState>>,
}

impl EuclideanSequencer {
    pub fn new(config_rx: mpsc::Receiver<EuclideanSequencerConfig>,
               mixer_update_tx: mpsc::Sender<()>,
               shared_state: Arc<Mutex<SharedState>>) -> Self {
        let config = EuclideanSequencerConfig::new();
        EuclideanSequencer { config, config_rx, mixer_update_tx, shared_state }
    }
}

impl Sequencer for EuclideanSequencer {
    async fn generate_sequence(&self) -> Sequence {
        let mut sequence = Sequence::empty();

        if self.config.pulses == 0 {
            // Handle zero pulses case
            let note = Note::new(0, 0, NoteDuration::Sixteenth, self.shared_state.lock().await.bpm);
            sequence.notes.push(note);
            return sequence;
        }

        // Bresenham line algorithm cus it looks easier
        let beat_locations = (0..self.config.pulses)
            .map(|i| (i * self.config.steps) / self.config.pulses)
            .collect::<Vec<_>>();

        for i in 0..self.config.steps {
            let note = if beat_locations.contains(&(i % self.config.steps)) {
                Note::new(self.config.pitch, 100, NoteDuration::Sixteenth, self.shared_state.lock().await.bpm)
            } else {
                Note::new(0, 0, NoteDuration::Sixteenth, self.shared_state.lock().await.bpm)
            };
            sequence.notes.push(note);
        }
         sequence
    }

    async fn run(&mut self, sequencer_slot: usize) -> Result<(), Error> {
        loop {
            if let Some(config) = self.config_rx.recv().await {
                debug!("Euclidean sequencer received config: {:?}", config);
                self.config = config;
                let sequence = self.generate_sequence().await;
                {
                    let mut state = self.shared_state.lock().await;
                    match sequencer_slot {
                        0 => state.mixer_config.sequence_a = sequence,
                        1 => state.mixer_config.sequence_b = sequence,
                        _ => panic!("Invalid sequencer slot"),
                    }
                }
                self.mixer_update_tx.send(()).await?;
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    }
}