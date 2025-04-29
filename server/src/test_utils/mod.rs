use std::thread;

use aws_config::{meta::region::RegionProviderChain, BehaviorVersion};
use aws_sdk_s3::{self as s3};
use testcontainers::{runners::AsyncRunner, ContainerAsync, ImageExt};
use testcontainers_modules::localstack::LocalStack;
use tokio::sync::OnceCell;
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    Mutex,
};

pub type TestResult<T> = Result<T, Box<dyn std::error::Error + 'static>>;

enum ContainerCommands {
    Stop,
}

struct Channel<T> {
    tx: Sender<T>,
    rx: Mutex<Receiver<T>>,
}

fn channel<T>() -> Channel<T> {
    let (tx, rx) = mpsc::channel(32);
    Channel {
        tx,
        rx: Mutex::new(rx),
    }
}

// Holds the shared Localstack container; if it's not held here, the container ref gets dropped and the _actual_ container
// gets stopped
static LOCALSTACK_NODE: OnceCell<ContainerAsync<LocalStack>> = OnceCell::const_new();
pub async fn localstack_node() -> &'static ContainerAsync<LocalStack> {
    LOCALSTACK_NODE
        .get_or_init(|| async {
            LocalStack::default()
                .with_env_var("SERVICES", "s3")
                .start()
                .await
                .expect("Localstack to start properly")
        })
        .await
}

// Holds a channel that we use to listen to requests to shut down the localstack container
static LOCALSTACK_CHANNEL: std::sync::OnceLock<Channel<ContainerCommands>> =
    std::sync::OnceLock::new();
fn localstack_channel() -> &'static Channel<ContainerCommands> {
    LOCALSTACK_CHANNEL.get_or_init(|| channel())
}

// Holds a channel that we use to block on to messages indicating that the localstack container has been shut down
static LOCALSTACK_SHUT_DOWN_NOTIFIER_CHANNEL: std::sync::OnceLock<Channel<()>> =
    std::sync::OnceLock::new();
fn localstack_shut_down_notifier_channel() -> &'static Channel<()> {
    LOCALSTACK_SHUT_DOWN_NOTIFIER_CHANNEL.get_or_init(|| channel())
}

// Holds a static Tokio runtime for blocking ops
static TOKIO_RUNTIME: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();
fn tokio_runtume() -> &'static tokio::runtime::Runtime {
    TOKIO_RUNTIME.get_or_init(|| tokio::runtime::Runtime::new().unwrap())
}

// Setup hooks registration
#[ctor::ctor]
fn on_startup() {
    setup_localstack();
}

// Shutdown hook registration
#[ctor::dtor]
fn on_shutdown() {
    shutdown_localstack();
}

// Function to set up localstack and a thread to listen on the shutdown command channel
fn setup_localstack() {
    thread::spawn(|| {
        tokio_runtume().block_on(start_localstack());
        // This needs to be here otherwise the container did not call the drop function before the application stops
        localstack_shut_down_notifier_channel()
            .tx
            .blocking_send(())
            .unwrap();
    });
}

// Function to send a shutdown command and block on receiving a message that it has occured
fn shutdown_localstack() {
    localstack_channel()
        .tx
        .blocking_send(ContainerCommands::Stop)
        .unwrap();
    localstack_shut_down_notifier_channel()
        .rx
        .blocking_lock()
        .blocking_recv()
        .unwrap();
}

// Start localstack
async fn start_localstack() {
    let localstack_node_container = localstack_node().await;
    let mut rx = localstack_channel().rx.lock().await;
    while let Some(command) = rx.recv().await {
        match command {
            ContainerCommands::Stop => {
                localstack_node_container.stop().await.unwrap();
                rx.close();
            }
        }
    }
}

// Holds the shared S3 client that refers to the above; use `s3_client().await` to get it
static S3_CLIENT: OnceCell<aws_sdk_s3::Client> = OnceCell::const_new();
pub async fn s3_client() -> &'static aws_sdk_s3::Client {
    S3_CLIENT
        .get_or_init(|| async {
            let node = localstack_node().await;
            let host_port = node
                .get_host_port_ipv4(4566)
                .await
                .expect("Port from Localstack to be retrievable");

            let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
            let region = region_provider.region().await.unwrap();
            let creds = s3::config::Credentials::new("fake", "fake", None, None, "test");
            let config = aws_config::defaults(BehaviorVersion::v2025_01_17())
                .region(region.clone())
                .credentials_provider(creds)
                .endpoint_url(format!("http://127.0.0.1:{host_port}"))
                .load()
                .await;

            s3::Client::new(&config)
        })
        .await
}
