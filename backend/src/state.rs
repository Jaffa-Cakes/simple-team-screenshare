use std::{collections::HashMap, sync::Arc};

use bytes::Bytes;
use thiserror::Error;
use tokio::sync::{
    Mutex,
    broadcast::{Receiver, Sender, channel},
};

pub struct State {
    streams: HashMap<String, Sender<Bytes>>,
    streams_changed: Sender<Vec<String>>,
}

#[derive(Error, Debug)]
pub enum AddStreamError {
    #[error("Stream ID already exists")]
    StreamIdAlreadyExists,
}

#[derive(Error, Debug)]
pub enum RemoveStreamError {
    #[error("Stream ID not found")]
    StreamIdNotFound,
}

impl State {
    pub fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            streams: HashMap::default(),
            streams_changed: channel(100).0,
        }))
    }

    pub fn add_stream(&mut self, stream_id: String) -> Result<(), AddStreamError> {
        if self.streams.contains_key(&stream_id) {
            return Err(AddStreamError::StreamIdAlreadyExists);
        }

        let (sender, _) = channel(10_000);
        self.streams.insert(stream_id.clone(), sender);
        let _ = self
            .streams_changed
            .send(self.streams.keys().cloned().collect());

        Ok(())
    }

    pub fn remove_stream(&mut self, stream_id: &str) -> Result<(), RemoveStreamError> {
        if self.streams.remove(stream_id).is_none() {
            return Err(RemoveStreamError::StreamIdNotFound);
        }
        let _ = self
            .streams_changed
            .send(self.streams.keys().cloned().collect());

        Ok(())
    }

    pub fn get_stream_sender(&mut self, stream_id: &str) -> Option<Sender<Bytes>> {
        self.streams.get(stream_id).cloned()
    }

    pub fn get_stream_receiver(&mut self, stream_id: &str) -> Option<Receiver<Bytes>> {
        self.streams.get(stream_id).map(|sender| sender.subscribe())
    }

    pub fn get_streams_changed_receiver(&mut self) -> Receiver<Vec<String>> {
        self.streams_changed.subscribe()
    }

    pub fn get_stream_ids(&self) -> Vec<String> {
        self.streams.keys().cloned().collect()
    }
}
