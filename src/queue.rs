use std::path::Path;
use std::collections::BTreeMap;
use std::sync::{OnceLock, Arc};
use tokio::sync::Mutex;
use lmdb_queue::{Env, topic::{Topic, Producer}};

use super::QueueLag;

static ENV: OnceLock<Env> = OnceLock::new();
pub fn init_env<P: AsRef<Path>>(path: P) {
    let _ = ENV.set(Env::new(path, None, None).unwrap());
}

static QUEUE: OnceLock<Queue> = OnceLock::new();

pub struct Queue<'env> {
    env: &'env Env,
    topics: Arc<Mutex<BTreeMap<String, Producer<'env>>>>,
}

pub fn get_queue<'env>() -> &'static Queue<'env> {
    QUEUE.get_or_init(|| Queue::new().unwrap())
}

impl <'env> Queue<'env> {
    fn new() -> Result<Self, anyhow::Error> {
        Ok(Self {
          env: ENV.get().unwrap(),
          topics: Arc::new(Mutex::new(BTreeMap::new()))
        })
    }

    pub async fn init_topic(&self, topic_name: &str) -> Result<(), anyhow::Error> {
        let mut topics = self.topics.lock().await;
        topics.entry(topic_name.to_string()).or_insert_with(move || self.env.producer(topic_name, None).unwrap());
        
        Ok(())
    }

    pub async fn push_back(&self, topic_name: &str, msg: &[u8]) -> Result<(), anyhow::Error> {
        let mut topics = self.topics.lock().await;
        let producer = topics.entry(topic_name.to_string()).or_insert_with(move || self.env.producer(topic_name, None).unwrap());
        producer.push_back(msg)?;
        
        Ok(())
    }

    pub async fn lags(&self) -> Result<Vec<QueueLag>, anyhow::Error> {
        let topics = self.topics.lock().await;
        let lags: Vec<QueueLag> = topics.iter()
            .map(|(name, producer)|
                QueueLag{ queue_id: name.clone(), lag: producer.lag().unwrap_or_default() as i64 }
            ).collect();
        Ok(lags)
    }
}