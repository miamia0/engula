// Copyright 2021 The Engula Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::{
    collections::{hash_map, HashMap},
    sync::Arc,
};

use tokio::sync::Mutex;

use super::{
    error::{Error, Result},
    object::MemObject,
};
use crate::{async_trait, Bucket, ObjectUploader};

type Objects = Arc<Mutex<HashMap<String, MemObject>>>;

#[derive(Clone)]
pub struct MemBucket {
    objects: Objects,
}

impl Default for MemBucket {
    fn default() -> Self {
        MemBucket {
            objects: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl Bucket<MemObject> for MemBucket {
    type ObjectUploader = MemObjectUploader;

    async fn object(&self, name: &str) -> Result<MemObject> {
        let objects = self.objects.lock().await;
        match objects.get(name) {
            Some(object) => Ok(object.clone()),
            None => Err(Error::NotFound(format!("object '{}'", name))),
        }
    }

    async fn upload_object(&self, name: &str) -> Result<Self::ObjectUploader> {
        Ok(MemObjectUploader::new(
            name.to_owned(),
            self.objects.clone(),
        ))
    }

    async fn delete_object(&self, name: &str) -> Result<()> {
        let mut objects = self.objects.lock().await;
        match objects.remove(name) {
            Some(_) => Ok(()),
            None => Err(Error::NotFound(format!("object '{}'", name))),
        }
    }
}

pub struct MemObjectUploader {
    name: String,
    data: Vec<u8>,
    objects: Objects,
}

impl MemObjectUploader {
    fn new(name: String, objects: Objects) -> MemObjectUploader {
        MemObjectUploader {
            name,
            data: Vec::new(),
            objects,
        }
    }
}

#[async_trait]
impl ObjectUploader for MemObjectUploader {
    type Error = Error;

    async fn write(&mut self, buf: &[u8]) -> Result<()> {
        self.data.extend_from_slice(buf);
        Ok(())
    }

    async fn finish(self) -> Result<usize> {
        let len = self.data.len();
        let object = MemObject::new(self.data);
        let mut objects = self.objects.lock().await;
        match objects.entry(self.name.clone()) {
            hash_map::Entry::Vacant(ent) => {
                ent.insert(object.clone());
                Ok(len)
            }
            hash_map::Entry::Occupied(_) => {
                Err(Error::AlreadyExists(format!("object '{}'", self.name)))
            }
        }
    }
}
