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

use databroker_proto::v1 as proto;

use tokio::{sync::mpsc, time::Instant};
use tokio_stream::wrappers::ReceiverStream;

use std::{collections::HashMap, time::Duration};

use crate::sampler;

// pub(crate) struct Provider {}

// impl Provider {
//     pub fn new() -> Self {}
// }

fn create_payload(value: impl ToString, id: i32) -> proto::StreamDatapointsRequest {
    proto::StreamDatapointsRequest {
        datapoints: HashMap::from([(
            id,
            proto::Datapoint {
                timestamp: None,
                value: Some(proto::datapoint::Value::StringValue(value.to_string())),
            },
        )]),
    }
}

pub(crate) async fn provide(sampler: sampler::Sampler, databroker_address: &str) {
    let endpoint = match tonic::transport::Channel::from_shared(databroker_address.to_owned()) {
        Ok(endpoint) => endpoint,
        Err(_) => {
            panic!(
                "Error creating endpoint: {:?}",
                databroker_address.to_owned()
            );
        }
    };
    let connect = endpoint.connect().await;
    match connect {
        Ok(channel) => {
            let mut client = proto::collector_client::CollectorClient::new(channel);

            let datapoint1_id = match client
                .register_datapoints(tonic::Request::new(proto::RegisterDatapointsRequest {
                    list: vec![proto::RegistrationMetadata {
                        name: "Vehicle.Cabin.Infotainment.Media.Played.Track".to_owned(),
                        description: "Vehicle.Cabin.Infotainment.Media.Played.Track".to_owned(),
                        data_type: proto::DataType::String as i32,
                        change_type: proto::ChangeType::Continuous as i32,
                    }],
                }))
                .await
            {
                Ok(metadata) => {
                    metadata.into_inner().results["Vehicle.Cabin.Infotainment.Media.Played.Track"]
                }
                Err(err) => {
                    println!("Couldn't retrieve metadata: {err:?}");
                    -1
                }
            };

            if datapoint1_id == -1 {
                return;
            }

            let (tx, rx) = mpsc::channel(10);

            let _stream = tokio::spawn(async move {
                match client.stream_datapoints(ReceiverStream::new(rx)).await {
                    Ok(response) => {
                        let mut stream = response.into_inner();

                        while let Ok(message) = stream.message().await {
                            match message {
                                Some(message) => {
                                    for error in message.errors {
                                        println!(
                                            "Error setting datapoint {}: {:?}",
                                            error.0,
                                            proto::DatapointError::from_i32(error.1)
                                        )
                                    }
                                }
                                None => {
                                    break;
                                }
                            }
                        }
                    }
                    Err(err) => {
                        println!("{}", err.message());
                    }
                }
            });

            let mut n = 0;
            loop {
                let sample_size = sampler.sample_size;
                let mut samples = Vec::with_capacity(sample_size.try_into().unwrap());
                for _ in 0..sample_size {
                    n += 1;
                    if n > sampler.iterations {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(1)).await;
                    let now = Instant::now();
                    let sample = sampler::Sample {
                        cycle: n,
                        timestamp: now,
                    };
                    match tx.send(create_payload(n, datapoint1_id)).await {
                        Ok(_) => samples.push(sample),
                        Err(err) => eprint!("{err}"),
                    };
                }

                match sampler.send(samples).await {
                    Ok(_) => {
                        if n > sampler.iterations {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }

            // match stream.await {
            //     Ok(_) => {}
            //     Err(err) => eprint!("{err}"),
            // };
        }
        Err(err) => {
            println!("{err}");
        }
    }
}
