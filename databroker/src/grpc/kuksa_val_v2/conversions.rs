// /********************************************************************************
// * Copyright (c) 2022 Contributors to the Eclipse Foundation
// *
// * See the NOTICE file(s) distributed with this work for additional
// * information regarding copyright ownership.
// *
// * This program and the accompanying materials are made available under the
// * terms of the Apache License 2.0 which is available at
// * http://www.apache.org/licenses/LICENSE-2.0
// *
// * SPDX-License-Identifier: Apache-2.0
// ********************************************************************************/

use databroker_proto::kuksa::val::v2 as proto;
use databroker_proto::kuksa::val::v2::datapoint::ValueState::Failure;
use databroker_proto::kuksa::val::v2::datapoint::ValueState::Value;

use crate::broker;

use std::convert::TryFrom;
use std::time::SystemTime;

impl From<&proto::Datapoint> for broker::Datapoint {
    fn from(datapoint: &proto::Datapoint) -> Self {
        let value = broker::DataValue::from(datapoint);
        let ts = SystemTime::now();

        match &datapoint.timestamp {
            Some(source_timestamp) => {
                let source: Option<SystemTime> = match source_timestamp.clone().try_into() {
                    Ok(source) => Some(source),
                    Err(_) => None,
                };
                broker::Datapoint {
                    ts,
                    source_ts: source,
                    value,
                }
            }
            None => broker::Datapoint {
                ts,
                source_ts: None,
                value,
            },
        }
    }
}

impl From<&proto::Datapoint> for broker::DataValue {
    fn from(datapoint: &proto::Datapoint) -> Self {
        match &datapoint.value_state {
            Some(Value(value)) => match &value.typed_value {
                Some(proto::value::TypedValue::String(value)) => broker::DataValue::String(value.to_owned()),
                Some(proto::value::TypedValue::Bool(value)) => broker::DataValue::Bool(*value),
                Some(proto::value::TypedValue::Int32(value)) => broker::DataValue::Int32(*value),
                Some(proto::value::TypedValue::Int64(value)) => broker::DataValue::Int64(*value),
                Some(proto::value::TypedValue::Uint32(value)) => broker::DataValue::Uint32(*value),
                Some(proto::value::TypedValue::Uint64(value)) => broker::DataValue::Uint64(*value),
                Some(proto::value::TypedValue::Float(value)) => broker::DataValue::Float(*value),
                Some(proto::value::TypedValue::Double(value)) => broker::DataValue::Double(*value),
                Some(proto::value::TypedValue::StringArray(array)) => broker::DataValue::StringArray(array.values.clone()),
                Some(proto::value::TypedValue::BoolArray(array)) => broker::DataValue::BoolArray(array.values.clone()),
                Some(proto::value::TypedValue::Int32Array(array)) => broker::DataValue::Int32Array(array.values.clone()),
                Some(proto::value::TypedValue::Int64Array(array)) => broker::DataValue::Int64Array(array.values.clone()),
                Some(proto::value::TypedValue::Uint32Array(array)) => broker::DataValue::Uint32Array(array.values.clone()),
                Some(proto::value::TypedValue::Uint64Array(array)) => broker::DataValue::Uint64Array(array.values.clone()),
                Some(proto::value::TypedValue::FloatArray(array)) => broker::DataValue::FloatArray(array.values.clone()),
                Some(proto::value::TypedValue::DoubleArray(array)) => broker::DataValue::DoubleArray(array.values.clone()),
                None => todo!(),   
            },
            Some(Failure(_)) => broker::DataValue::NotAvailable,
            None => broker::DataValue::NotAvailable,
        }
    }
}