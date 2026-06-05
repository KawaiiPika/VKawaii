use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};

pub struct AudioLipSync {
    pub current_volume: Arc<Mutex<f32>>,
    // Holding the Stream so it doesn't get Dropped
    _stream: Option<cpal::Stream>,
}

impl AudioLipSync {
    pub fn new() -> Result<Self> {
        let current_volume = Arc::new(Mutex::new(0.0));
        Ok(Self {
            current_volume,
            _stream: None,
        })
    }

    pub fn start_listening(&mut self) -> Result<()> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .expect("No input device available");
        let config = device.default_input_config()?;

        let volume_clone = self.current_volume.clone();

        let stream = device.build_input_stream(
            &config.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let mut sum = 0.0;
                for &sample in data {
                    sum += sample * sample;
                }
                let rms = (sum / data.len() as f32).sqrt();

                if let Ok(mut vol) = volume_clone.lock() {
                    // Smoothing the Volume or Mapping it to a [0.0, 1.0] Range for jaw Opening
                    *vol = rms.min(1.0);
                }
            },
            |err| eprintln!("An error occurred on the input audio stream: {}", err),
            None, // No timeout
        )?;

        stream.play()?;
        self._stream = Some(stream);

        Ok(())
    }

    /// Getting the Current Mapped mouth Opening Value (0.0 to 1.0)
    pub fn get_mouth_open(&self) -> f32 {
        if let Ok(vol) = self.current_volume.lock() {
            // Applying Some Multiplier so Speaking Softly still Opens the mouth
            (*vol * 5.0).min(1.0)
        } else {
            0.0
        }
    }
}
