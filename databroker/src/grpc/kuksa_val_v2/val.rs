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

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::mpsc::Sender;

use databroker_proto::kuksa::val::v2 as proto;
use kuksa::proto::v2::open_provider_stream_request;
use kuksa::proto::v2::open_provider_stream_response;
use kuksa::proto::v2::OpenProviderStreamResponse;
use kuksa::proto::v2::PublishValuesResponse;
use tokio::{select, sync::mpsc};
use databroker_proto::kuksa::val::v2::open_provider_stream_request::Action::PublishValuesRequest;
use databroker_proto::kuksa::val::v2::open_provider_stream_request::Action::BatchActuateStreamResponse;
use databroker_proto::kuksa::val::v2::open_provider_stream_request::Action::ProvidedActuation;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::Stream;
use tonic::Code;
use tonic::Response;
use tonic::Status;
use tracing::debug;

use crate::broker::DataBroker;
use crate::permissions;
use crate::{broker::{self, AuthorizedAccess}, permissions::Permissions};

#[tonic::async_trait]
impl proto::val_server::Val for broker::DataBroker {
    async fn get_value(
        &self,
        request: tonic::Request<proto::GetValueRequest>,
    ) -> Result<tonic::Response<proto::GetValueResponse>, tonic::Status>
    {
        Err(tonic::Status::new(Code::Unimplemented, "Unimplemented"))
    }
    
    async fn get_values(
        &self,
        request: tonic::Request<proto::GetValuesRequest>,
    ) -> Result<tonic::Response<proto::GetValuesResponse>, tonic::Status>
    {
        Err(tonic::Status::new(Code::Unimplemented, "Unimplemented"))
    }
    
    async fn list_values(
        &self,
        request: tonic::Request<proto::ListValuesRequest>,
    ) -> Result<tonic::Response<proto::ListValuesResponse>, tonic::Status>
    {
        Err(tonic::Status::new(Code::Unimplemented, "Unimplemented"))
    }

    type SubscribeStream = Pin<
        Box<
            dyn Stream<Item = Result<proto::SubscribeResponse, tonic::Status>>
                + Send
                + Sync
                + 'static,
        >,
    >;
    
    async fn subscribe(
        &self,
        request: tonic::Request<proto::SubscribeRequest>,
    ) -> Result<tonic::Response<Self::SubscribeStream>, tonic::Status>
    {
        Err(tonic::Status::new(Code::Unimplemented, "Unimplemented"))
    }

    async fn actuate(
        &self,
        request: tonic::Request<proto::ActuateRequest>,
    ) -> Result<tonic::Response<proto::ActuateResponse>, tonic::Status>
    {
        Err(tonic::Status::new(Code::Unimplemented, "Unimplemented"))
    }

    async fn batch_actuate(
        &self,
        request: tonic::Request<proto::BatchActuateRequest>,
    ) -> Result<tonic::Response<proto::BatchActuateResponse>, tonic::Status>
    {
        Err(tonic::Status::new(Code::Unimplemented, "Unimplemented"))
    }
    
    async fn list_metadata(
        &self,
        request: tonic::Request<proto::ListMetadataRequest>,
    ) -> Result<tonic::Response<proto::ListMetadataResponse>, tonic::Status>
    {
        Err(tonic::Status::new(Code::Unimplemented, "Unimplemented"))
    }

    async fn publish_value(
        &self,
        request: tonic::Request<proto::PublishValueRequest>,
    ) -> Result<tonic::Response<proto::PublishValueResponse>, tonic::Status>
    {
        Err(tonic::Status::new(Code::Unimplemented, "Unimplemented"))
    }

    type OpenProviderStreamStream = Pin<
        Box<
            dyn Stream<Item = Result<proto::OpenProviderStreamResponse, tonic::Status>>
                + Send
                + Sync
                + 'static,
        >,
    >;

    async fn open_provider_stream(
        &self,
        request: tonic::Request<tonic::Streaming<proto::OpenProviderStreamRequest>>,
    ) -> Result<tonic::Response<Self::OpenProviderStreamStream>, tonic::Status>
    {
        debug!(?request);
        let permissions = match request.extensions().get::<Permissions>() {
            Some(permissions) => {
                debug!(?permissions);
                permissions.clone()
            }
            None => return Err(tonic::Status::unauthenticated("Unauthenticated")),
        };

        let mut stream = request.into_inner();

        let mut shutdown_trigger = self.get_shutdown_trigger();

        // Copy (to move into task below)
        let broker = self.clone();

        // Create error stream (to be returned)
        let (error_sender, error_receiver) = mpsc::channel(10);

        // Listening on stream
        tokio::spawn(async move {
            let permissions = permissions;
            loop {
                select! {
                    message = stream.message() => {
                        match message {
                            Ok(request) => {
                                match request {
                                    Some(req) => {
                                        match req.action {
                                            Some(ProvidedActuation(value)) => {

                                            },
                                            Some(PublishValuesRequest(publish_values_request)) => {
                                                publish_values(&broker, &publish_values_request, &permissions, &error_sender);
                                            },
                                            Some(BatchActuateStreamResponse(value)) => {

                                            },
                                            None => {

                                            },
                                        }
                                    },
                                    None => {
                                        debug!("provider: no more messages");
                                        break;
                                    }
                                }
                            },
                            Err(err) => {
                                debug!("provider: connection broken: {:?}", err);
                                break;
                            },
                        }
                    },
                    _ = shutdown_trigger.recv() => {
                        debug!("provider: shutdown received");
                        break;
                    }
                }
            }
        });

        // Return the error stream
        Ok(Response::new(Box::pin(ReceiverStream::new(error_receiver))))
    }

    async fn get_server_info(
        &self,
        request: tonic::Request<proto::GetServerInfoRequest>,
    ) -> Result<tonic::Response<proto::GetServerInfoResponse>, tonic::Status>
    {
        Err(tonic::Status::new(Code::Unimplemented, "Unimplemented"))
    }
}

async fn publish_values(
    broker: &DataBroker,
    request: &databroker_proto::kuksa::val::v2::PublishValuesRequest,
    permissions: &Permissions,
    error_sender: &Sender<Result<proto::OpenProviderStreamResponse, Status>>
) {
        let broker = broker.authorized_access(&permissions);

        let ids: Vec<(i32, broker::EntryUpdate)> = request.datapoints
            .iter()
            .map(|(id, datapoint)|
                (
                    *id,
                    broker::EntryUpdate {
                        path: None,
                        datapoint: Some(broker::Datapoint::from(datapoint)),
                        actuator_target: None,
                        entry_type: None,
                        data_type: None,
                        description: None,
                        allowed: None,
                        unit: None,
                    }
                )
            )
            .collect();

            match broker
                .update_entries(ids)
                .await
            {
                Ok(_) => {}
                Err(err) => {
                    let publish_values_response = PublishValuesResponse {
                        request_id: 123,
                        status: HashMap::new(),
                    };

                    let action = open_provider_stream_response::Action::PublishValuesResponse(publish_values_response);

                    let response = OpenProviderStreamResponse {
                        action: Some(action),
                    };
                    error_sender.send(Ok(response));
                }
            }
}