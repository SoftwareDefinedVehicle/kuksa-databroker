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

#[derive(Clone)]
pub struct Signal {
    pub id: i32,
    pub path: String,
}

pub struct Metadata {}

impl Signal {
    pub fn new(id: i32, path: impl ToString) -> Self {
        Self {
            path: path.to_string(),
            id,
        }
    }
}