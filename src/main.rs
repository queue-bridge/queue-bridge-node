mod queuebridge {
    tonic::include_proto!("queuebridge");
}

pub use queuebridge::QueueLag;
use queuebridge::queue_bridge_balancer_client::QueueBridgeBalancerClient;
use queuebridge::{SubscribeRequest, QueueMessage, HeartbeatRequest};

use tonic::transport::{Channel, Endpoint};
// use tonic::Status;
// use tokio_stream::StreamExt;
use std::time::Duration;

use std::env;
use futures::future::try_join_all;

mod queue;
use queue::get_queue;

async fn heartbeat_loop(mut client: QueueBridgeBalancerClient<Channel>) {
    loop {
        if let Ok(queue_lags) = get_queue().lags().await {
            _ = client.heartbeat(HeartbeatRequest{ queue_lags }).await;
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

async fn subscribe_loop(
    mut client: QueueBridgeBalancerClient<Channel>,
    queue_id: String,
) {
    loop {
        match client.subscribe(SubscribeRequest{ queue_id: queue_id.clone() }).await {
            Ok(mut resp) => {
                while let Ok(Some(QueueMessage{queue_id:_, message})) = resp.get_mut().message().await {
                    println!("{}: {:?}", queue_id, message);
                    if let Err(_) = get_queue().push_back(&queue_id, &message).await {
                        println!("Push to queue error, stopping.");
                        break;
                    }
                }
            }
            Err(e) => eprintln!("stream error on {queue_id}: {e}"),
        }
        tokio::time::sleep(Duration::from_secs(1)).await; // back-off
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let servers: Vec<String> =
        env::var("SERVERS")
            .unwrap_or("127.0.0.1:50051".to_string())
            .split(',').map(|s| s.trim().to_string())
            .collect();
    let topics:  Vec<String> =
        env::var("TOPICS")
            .unwrap_or("ddj".to_string())
            .split(',').map(|t| t.trim().to_string())
            .collect();

    // Build one channel per server, enable TLS if needed.
    let mut tasks = Vec::new();
    for server in &servers {
        let channel = Endpoint::from_shared(format!("http://{server}"))?
            .tcp_keepalive(Some(std::time::Duration::from_secs(60)))
            .connect()
            .await?;

        for topic in &topics {
            tasks.push(tokio::spawn(subscribe_loop(QueueBridgeBalancerClient::new(channel.clone()), topic.clone())));
            tasks.push(tokio::spawn(heartbeat_loop(QueueBridgeBalancerClient::new(channel.clone()))));
        }
    }

    // Wait forever until any task errors hard.
    try_join_all(tasks).await?;
    Ok(())
}

#[tokio::test]
async fn test_push_message() -> Result<(), anyhow::Error> {
    let channel = Endpoint::from_shared(format!("http://127.0.0.1:50051"))?
        .tcp_keepalive(Some(std::time::Duration::from_secs(60)))
        .connect()
        .await?;

    let mut client = QueueBridgeBalancerClient::new(channel);
    client.push(QueueMessage{
        queue_id: "ddj".to_string(),
        message: "dsafasfasfa".as_bytes().to_vec()
    }).await?;

    Ok(())
}

#[test]
fn test_get_queue() -> Result<(), anyhow::Error> {
    let env = lmdb_queue::Env::new("/tmp/queue-bridge", None, None)?;
    let mut comsumer = env.comsumer("ddj", None)?;

    let mut message_count = 0;
    loop {
        let items = comsumer.pop_front_n(10)?;
        if items.len() > 0 {
            message_count += items.len();
            // if message_count % (1024 * 100) == 0 {
                println!("Got message: {}", String::from_utf8(items[0].data.clone())?);
            // }
        } else {
            println!("Read {} messages.", message_count);
            break;
        }
    }

    Ok(())
}
