use std::thread;

use testcontainers::{runners::AsyncRunner, ContainerAsync, ImageExt};
use testcontainers_modules::localstack::LocalStack;
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    Mutex, OnceCell,
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
