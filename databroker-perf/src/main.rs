/********************************************************************************
* Copyright (c) 2024 Contributors to the Eclipse Foundation
*
* See the NOTICE file(s) distributed with this work for additional
* information regarding copyright ownership.
*
* This program and the accompanying materials are made available under the
* terms of the Apache License 2.0 which is available at
* http://www.apache.org/licenses/LICENSE-2.0
*
* SPDX-License-Identifier: Apache-2.0
********************************************************************************/

use std::{
    collections::HashMap,
    time::{Duration, SystemTime},
};

use anyhow::{Context, Result};
use clap::Parser;
use databroker_proto::sdv::databroker::v1 as proto;
use hdrhistogram::Histogram;
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use log::error;
use tokio::{task::JoinSet, time::Instant};

mod config;
mod provider;
mod subscriber;
mod types;

#[derive(Parser)]
#[clap(author, version, about)]
struct Args {
    #[clap(long, display_order = 1, default_value_t = 1000)]
    iterations: u64,
    #[clap(long, display_order = 3, default_value = "http://127.0.0.1")]
    host: String,
    #[clap(long, display_order = 4, default_value_t = 55555)]
    port: u64,
    #[clap(long, display_order = 5, default_value_t = 10)]
    skip: u64,
    #[clap(long = "config", display_order = 6, value_name = "CONFIG_FILE")]
    config_file: Option<String>,
    #[clap(long = "verbosity", short, display_order = 7, default_value_t = log::Level::Warn)]
    verbosity_level: log::Level,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Setup logging
    stderrlog::new()
        .module(module_path!())
        .verbosity(args.verbosity_level)
        .init()?;

    let iterations = args.iterations + args.skip;
    let skip = args.skip;
    let databroker_address = format!("{}:{}", args.host, args.port);

    let signals = match args.config_file {
        Some(ref filename) => {
            let config_file = std::fs::OpenOptions::new()
                .read(true)
                .open(filename)
                .with_context(|| format!("Failed to open configuration file '{}'", filename))?;
            let config: config::Config = serde_json::from_reader(config_file)
                .with_context(|| format!("Failed to parse configuration file '{}'", filename))?;

            Ok::<Vec<config::Signal>, anyhow::Error>(config.signals)
        }
        None => {
            // Default set of signals
            Ok(vec![config::Signal {
                path: "Vehicle.Cabin.Infotainment.Media.Played.Track".to_owned(),
            }])
        }
    }?;

    let endpoint = tonic::transport::Channel::from_shared(databroker_address.clone())
        .with_context(|| "Failed to parse server url")?;
    let provider_channel = endpoint
        .connect()
        .await
        .with_context(|| "Failed to connect to server")?;

    let metadata =
        provider::get_metadata(provider_channel.clone(), signals.clone().into_iter()).await?;

    let provider = provider::Provider::new(provider_channel.clone(), metadata.clone())
        .with_context(|| "Failed to setup provider")?;

    let subscriber_channel = endpoint
        .connect()
        .await
        .with_context(|| "Failed to connect to server")?;

    let subscriber =
        subscriber::Subscriber::new(subscriber_channel, signals.clone(), metadata.clone());

    let mut hist = Histogram::<u64>::new_with_bounds(1, 60 * 60 * 1000 * 1000, 3)?;

    let start_time = SystemTime::now();

    let mut n = 0;
    let mut skipped = 0;
    let mut interval = tokio::time::interval(Duration::from_millis(1));
    let progress = ProgressBar::new(iterations);
    progress.set_style(
        ProgressStyle::with_template("[{elapsed_precise}] [{wide_bar}] {pos:>7}/{len:7}")
            .unwrap()
            .progress_chars("=> "),
    );
    loop {
        if n >= iterations {
            break;
        }
        interval.tick().await;

        let datapoints = HashMap::from_iter(metadata.iter().map(|entry| {
            let data_type = proto::DataType::from_i32(entry.1.data_type)
                .expect("proto i32 enum representation should always be valid enum");
            (
                entry.1.id,
                proto::Datapoint {
                    timestamp: None,
                    value: Some(provider::n_to_value(&data_type, n).unwrap()),
                },
            )
        }));

        let publish_task = provider.publish(datapoints);

        let mut subscriber_tasks: JoinSet<Result<Instant, subscriber::Error>> = JoinSet::new();

        for signal in &signals {
            // TODO: return an awaitable thingie (wrapping the Receiver<Instant>)
            let mut sub = subscriber.wait_for2(&signal.path)?;
            subscriber_tasks.spawn(async move {
                Ok(sub
                    .recv()
                    .await
                    .map_err(|err| subscriber::Error::RecvError(err.to_string()))?)
            });
        }

        let published = publish_task.await?;
        while let Some(received) = subscriber_tasks.join_next().await {
            match received {
                Ok(Ok(received)) => {
                    if n < skip {
                        skipped += 1;
                        continue;
                    }
                    let latency = received.duration_since(published.clone());
                    hist.record(latency.as_micros().try_into().unwrap())?;
                }
                Ok(Err(err)) => error!("{}", err.to_string()),
                Err(err) => error!("{}", err.to_string()),
            }
        }

        n += 1;
        progress.set_position(n);
    }

    progress.finish();

    let end_time = SystemTime::now();
    let total_duration = end_time.duration_since(start_time)?;
    println!("Summary:");
    println!(
        "  Sent: {} * {} signals = {}",
        n,
        signals.len(),
        n * signals.len() as u64
    );
    println!("  Skipped: {}", skipped);
    println!("  Received: {}", hist.len());
    println!(
        "  Total: {:.2} s",
        total_duration.as_millis() as f64 / 1000.0
    );
    println!("  Fastest: {:>7.3} ms", hist.min() as f64 / 1000.0);
    println!("  Slowest: {:>7.3} ms", hist.max() as f64 / 1000.0);
    println!("  Average: {:>7.3} ms", hist.mean() as f64 / 1000.0);
    println!("\nLatency histogram:");
    let step_size = (hist.max() - hist.min()) / 11;

    let buckets = hist.iter_linear(step_size);
    // skip initial empty buckets
    let buckets = buckets.skip_while(|v| v.count_since_last_iteration() == 0);
    let mut skipping_initial_empty = true;
    for v in buckets {
        let mean = v.value_iterated_to() + 1 - step_size / 2; // +1 to make range inclusive
        let count = v.count_since_last_iteration();
        if skipping_initial_empty && count == 0 {
            continue;
        } else {
            skipping_initial_empty = false;
        }
        let bars = count as f64 / (iterations * signals.len() as u64) as f64 * 100.0;
        let bar = std::iter::repeat("∎")
            .take(bars as usize)
            .collect::<String>();
        println!("  {:>7.3} ms [{:<5}] |{}", mean as f64 / 1000.0, count, bar);
    }

    println!("\nLatency distribution:");

    for q in &[10, 25, 50, 75, 90, 95, 99] {
        println!(
            "  {q}% in under {:.3} ms",
            hist.value_at_quantile(*q as f64 / 100.0) as f64 / 1000.0
        );
    }

    Ok(())
}
