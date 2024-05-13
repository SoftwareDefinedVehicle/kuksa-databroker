use std::collections::HashMap;

use clap::Parser;
use hdrhistogram::Histogram;
use sampler::Sampler;
use tokio::{join, sync::mpsc};

mod provider;
mod sampler;
mod subscriber;

#[derive(Parser)]
#[clap(author, version, about)]
struct Config {
    #[clap(long, display_order = 1, default_value_t = 1000)]
    iterations: u64,
    #[clap(long, display_order = 2, default_value_t = 32)]
    sample_size: u64,
    #[clap(long, display_order = 3, default_value = "http://127.0.0.1")]
    databroker_host: String,
    #[clap(long, display_order = 4, default_value_t = 55555)]
    databroker_port: u64,
}

#[tokio::main]
async fn main() {
    let config = Config::parse();
    let iterations = config.iterations;
    let sample_size = config.sample_size;
    let databroker_address: &'static str = Box::leak(
        format!("{}:{}", config.databroker_host, config.databroker_port).into_boxed_str(),
    );
    let (subscriber_tx, mut subscriber_rx) = mpsc::channel(100);
    let (provider_tx, mut provider_rx) = mpsc::channel(100);
    let subscriber_sampler = Sampler::new(iterations, sample_size, subscriber_tx);
    let provider_sampler = Sampler::new(iterations, sample_size, provider_tx);

    let _subscriber = tokio::spawn(async move {
        match subscriber::subscribe(subscriber_sampler, &databroker_address).await {
            Ok(_) => {}
            Err(err) => {
                println!("{}", err);
            }
        }
    });

    let _provider = tokio::spawn(async move {
        provider::provide(provider_sampler, &databroker_address).await;
    });

    let mut hist = Histogram::<u64>::new_with_bounds(1, 60 * 60 * 1000 * 1000, 3).unwrap();

    loop {
        match join!(provider_rx.recv(), subscriber_rx.recv()) {
            (Some(Ok(provider_samples)), Some(Ok(subscriber_samples))) => {
                // println!("subscriber_samples: {subscriber_samples:?}");
                // println!("provider_samples: {provider_samples:?}");

                let mut samples = HashMap::with_capacity(sample_size.try_into().unwrap());
                for sample in subscriber_samples {
                    samples.insert(sample.cycle, sample.timestamp);
                }
                for sample in provider_samples {
                    match samples.get(&sample.cycle) {
                        Some(end_time) => {
                            let start_time = sample.timestamp;
                            let latency = end_time.duration_since(start_time);
                            // println!("({}) latency: {}", sample.cycle, latency.as_nanos());
                            hist.record(latency.as_micros().try_into().unwrap())
                                .unwrap();
                        }
                        None => {
                            // eprintln!("missing sample {}", sample.cycle);
                        }
                    }
                }
            }
            (None, None) => {
                // Done
                break;
            }
            (_, _) => {
                eprintln!("error");
                break;
            }
        }
    }

    println!("# of samples: {}", hist.len());
    println!(
        "25'th percentile: {:.2} ms",
        hist.value_at_quantile(0.25) as f64 / 1000.0
    );
    println!(
        "50'th percentile: {:.2} ms",
        hist.value_at_quantile(0.50) as f64 / 1000.0
    );
    println!(
        "90'th percentile: {:.2} ms",
        hist.value_at_quantile(0.90) as f64 / 1000.0
    );
    println!(
        "99'th percentile: {:.2} ms",
        hist.value_at_quantile(0.99) as f64 / 1000.0
    );
    println!(
        "99.9'th percentile: {:.2} ms",
        hist.value_at_quantile(0.999) as f64 / 1000.0
    );

    // match join!(subscriber, provider) {
    //     (Ok(_), Ok(_)) => todo!(),
    //     (Ok(_), Err(_)) => todo!(),
    //     (Err(_), Ok(_)) => todo!(),
    //     (Err(_), Err(_)) => todo!(),
    // }
}
