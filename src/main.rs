mod queuebridge {
    tonic::include_proto!("queuebridge");
}

use queuebridge::queue_bridge_balancer_client::QueueBridgeBalancerClient;
use queuebridge::{SubscribeRequest, QueueMessage};
use tonic::transport::{Channel, Endpoint};
// use tonic::Status;
// use tokio_stream::StreamExt;
use std::time::Duration;

use std::env;
use futures::future::try_join_all;

async fn subscribe_loop(
    mut client: QueueBridgeBalancerClient<Channel>,
    queue_id: String,
) {
    loop {
        match client.subscribe(SubscribeRequest{ queue_id: queue_id.clone() }).await {
            Ok(mut resp) => {
                while let Ok(Some(QueueMessage{queue_id:_, message})) = resp.get_mut().message().await {
                    println!("{}: {:?}", queue_id, message);
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
            let client = queuebridge::queue_bridge_balancer_client::QueueBridgeBalancerClient::new(channel.clone());
            tasks.push(tokio::spawn(subscribe_loop(client, topic.clone())));
        }
    }

    // Wait forever until any task errors hard.
    try_join_all(tasks).await?;
    Ok(())
}
