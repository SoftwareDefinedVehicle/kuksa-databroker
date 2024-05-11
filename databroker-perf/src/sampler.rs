use tokio::{sync::mpsc::Sender, time::Instant};

pub(crate) struct Sampler {
    pub sample_size: u64,
    pub iterations: u64,
    output: Sender<Result<Vec<Sample>, Error>>,
}

#[derive(Debug)]
pub(crate) struct Sample {
    pub cycle: u64,
    pub timestamp: Instant,
}

#[derive(Debug)]
pub(crate) enum Error {}

pub struct SendError {}

impl Sampler {
    pub fn new(
        iterations: u64,
        sample_size: u64,
        output: Sender<Result<Vec<Sample>, Error>>,
    ) -> Self {
        Self {
            iterations,
            sample_size,
            output,
        }
    }

    pub async fn send(&self, samples: Vec<Sample>) -> Result<(), SendError> {
        self.output
            .send(Ok(samples))
            .await
            .map_err(|_| SendError {})
    }
}
