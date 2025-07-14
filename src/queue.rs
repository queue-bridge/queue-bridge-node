use std::collections::BTreeMap;
use std::sync::{OnceLock, Arc};
use tokio::sync::Mutex;
use lmdb_queue::{Env, topic::{Topic, Producer}};

static ENV: OnceLock<Env> = OnceLock::new();
fn get_env() -> &'static Env {
    ENV.get_or_init(|| Env::new("/tmp/queue-bridge", None, None).unwrap())
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
          env: get_env(),
          topics: Arc::new(Mutex::new(BTreeMap::new()))
        })
    }

    pub async fn push_back(&self, topic_name: &str, msg: &[u8]) -> Result<(), anyhow::Error> {
        let mut topics = self.topics.lock().await;
        let producer = topics.entry(topic_name.to_string()).or_insert_with(move || self.env.producer(topic_name, None).unwrap());
        producer.push_back(msg)?;
        
        Ok(())
    }

    pub async fn lag_of(&self, topic_name: &str) -> Result<u64, anyhow::Error> {
        let mut topics = self.topics.lock().await;
        let producer = topics.entry(topic_name.to_string()).or_insert_with(move || self.env.producer(topic_name, None).unwrap());
        let lag = producer.lag()?;
        Ok(lag)
    }
}