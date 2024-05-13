/********************************************************************************
* Copyright (c) 2022 Contributors to the Eclipse Foundation
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

use crate::sampler;
use databroker_proto::v1 as proto;
use tokio::time::Instant;

pub(crate) async fn subscribe(
    sampler: sampler::Sampler,
    databroker_address: &'static str,
) -> Result<(), Box<dyn std::error::Error>> {
    match tonic::transport::Channel::from_static(databroker_address)
        .connect()
        .await
    {
        Ok(channel) => {
            let mut client = proto::broker_client::BrokerClient::new(channel);

            let args = tonic::Request::new(proto::SubscribeRequest {
                query: String::from("SELECT Vehicle.Cabin.Infotainment.Media.Played.Track"),
            });

            match client.subscribe(args).await {
                Ok(response) => {
                    let mut stream = response.into_inner();
                    let mut n: u64 = 0;

                    loop {
                        let sample_size = sampler.sample_size;
                        let mut samples = Vec::with_capacity(sample_size.try_into().unwrap());
                        let sample_start = n;
                        let sample_end = n + sample_size as u64;
                        for _ in 0..sample_size {
                            n += 1;
                            if n > sampler.iterations {
                                break;
                            }
                            if let Some(update) = stream.message().await? {
                                for (_, datapoint) in update.fields {
                                    if let Some(value) = datapoint.value {
                                        match value {
                                            proto::datapoint::Value::FailureValue(reason) => {
                                                eprintln!("-> Failure: {reason:?}");
                                            }
                                            proto::datapoint::Value::StringValue(string_value) => {
                                                match string_value.parse::<u64>() {
                                                    Ok(cycle) => {
                                                        if cycle < sample_start
                                                            || cycle > sample_end
                                                        {
                                                            eprintln!(
                                                                "cycle: {cycle} out of bound ({sample_start}..{sample_end}), ignoring"
                                                            );
                                                            n -= 1;
                                                        } else {
                                                            let sample = sampler::Sample {
                                                                cycle,
                                                                timestamp: Instant::now(),
                                                            };
                                                            samples.push(sample);
                                                            // eprintln!("subscribe({})", cycle);
                                                        }
                                                    }
                                                    Err(err) => {
                                                        eprintln!(
                                                            "Failed to parse u64 from {}: {err}",
                                                            string_value
                                                        );
                                                    }
                                                }
                                            }
                                            _ => eprintln!("-> Other value"),
                                        }
                                    } else {
                                        eprintln!("-> Empty datapoint value");
                                    }
                                    // }
                                }
                            } else {
                                eprintln!("Server gone. Subscription stopped");
                                break;
                            }
                        }
                        match sampler.send(samples).await {
                            Ok(_) => {
                                if n > sampler.iterations {
                                    break;
                                }
                            }
                            Err(_) => {
                                eprint!("Failed to send samples");
                                break;
                            }
                        }
                    }
                }
                Err(err) => {
                    eprintln!("{}", err.message());
                }
            }
        }
        Err(err) => {
            eprintln!("{err}");
        }
    }

    Ok(())
}
