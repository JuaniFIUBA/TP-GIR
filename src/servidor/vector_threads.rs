use std::{
    sync::{Arc, Mutex},
    thread::JoinHandle,
};

pub type VectorThreads = Arc<Mutex<Vec<JoinHandle<Result<(), String>>>>>;
