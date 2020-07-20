#![warn(rust_2018_idioms)]
#![forbid(unsafe_code)]

use std::{
    collections::HashMap,
    io,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use anyhow::Result;
use futures::{future, prelude::*};
use libp2p::{
    core::{
        either::EitherError,
        muxing::StreamMuxerBox,
        transport::{boxed::Boxed, timeout::TransportTimeoutError},
        upgrade::{SelectUpgrade, Version},
        UpgradeError,
    },
    dns::{DnsConfig, DnsErr},
    identity::Keypair,
    mplex::MplexConfig,
    ping::{Ping, PingConfig},
    secio::{SecioConfig, SecioError},
    swarm::SwarmBuilder,
    yamux, Multiaddr, PeerId, Swarm, Transport,
};
use log::{warn, Level};
use structopt::StructOpt;

use rust_libp2p_tokio_socks5::TorTokioTcpConfig;

/// The ping-pong onion service address.
const ONION: &str = "/onion3/r4nttccifklkruvrztwxuhk2iy4xx7cnnex2sgogbo4zw6rnx3cq2bid:7";
const LOCAL_PORT: u16 = 7777;

/// Tor should be started with a hidden service configured. Add the following to
/// your torrc
///
///     HiddenServiceDir /var/lib/tor/hidden_service/
///     HiddenServicePort 7 127.0.0.1:7777
///
/// See https://2019.www.torproject.org/docs/tor-onion-service for details on configuring
/// tor onion services (previously tor hidden services).
#[tokio::main]
async fn main() -> Result<()> {
    simple_logger::init_with_level(Level::Debug).unwrap();

    let opt = Opt::from_args();

    let addr = opt.onion.unwrap_or_else(|| ONION.to_string());
    let addr = addr
        .parse::<Multiaddr>()
        .expect("failed to parse multiaddr");

    if opt.dialer {
        run_dialer(addr).await?;
    } else {
        run_listener(addr).await?;
    }

    Ok(())
}

#[derive(Debug, StructOpt)]
#[structopt(name = "ping-pong", about = "libp2p ping-pong application over Tor.")]
pub struct Opt {
    /// Run as the dialer i.e., do the ping
    #[structopt(short, long)]
    pub dialer: bool,

    /// Run as the listener i.e., do the pong (default)
    #[structopt(short, long)]
    pub listener: bool,

    /// Onion mulitaddr to use (only required for dialer)
    #[structopt(long)]
    pub onion: Option<String>,
}

/// Entry point to run the ping-pong application as a dialer.
async fn run_dialer(addr: Multiaddr) -> Result<()> {
    let map = HashMap::new();
    let config = PingConfig::new()
        .with_keep_alive(true)
        .with_interval(Duration::from_secs(1));
    let mut swarm = build_swarm(config, map)?;

    Swarm::dial_addr(&mut swarm, addr).unwrap();

    future::poll_fn(move |cx: &mut Context<'_>| loop {
        match swarm.poll_next_unpin(cx) {
            Poll::Ready(Some(event)) => println!("{:?}", event),
            Poll::Ready(None) => return Poll::Ready(()),
            Poll::Pending => return Poll::Pending,
        }
    })
    .await;

    Ok(())
}

/// Entry point to run the ping-pong application as a listener.
async fn run_listener(onion: Multiaddr) -> Result<()> {
    let map = onion_port_map(onion.clone());
    log::info!("Onion service: {}", onion);

    let config = PingConfig::new().with_keep_alive(true);
    let mut swarm = build_swarm(config, map)?;

    Swarm::listen_on(&mut swarm, onion.clone())?;

    future::poll_fn(move |cx: &mut Context<'_>| loop {
        match swarm.poll_next_unpin(cx) {
            Poll::Ready(Some(event)) => println!("{:?}", event),
            Poll::Ready(None) => return Poll::Ready(()),
            Poll::Pending => return Poll::Pending,
        }
    })
    .await;

    Ok(())
}

/// Build a libp2p swarm.
pub fn build_swarm(config: PingConfig, map: HashMap<Multiaddr, u16>) -> Result<Swarm<Ping>> {
    let id_keys = Keypair::generate_ed25519();
    let peer_id = PeerId::from(id_keys.public());

    let transport = build_transport(id_keys, map)?;
    let behaviour = Ping::new(config);

    let swarm = SwarmBuilder::new(transport, behaviour, peer_id)
        .executor(Box::new(TokioExecutor))
        .build();

    Ok(swarm)
}

fn onion_port_map(onion: Multiaddr) -> HashMap<Multiaddr, u16> {
    let mut map = HashMap::new();
    map.insert(onion, LOCAL_PORT);
    map
}

struct TokioExecutor;

impl libp2p::core::Executor for TokioExecutor {
    fn exec(&self, future: Pin<Box<dyn Future<Output = ()> + Send>>) {
        tokio::spawn(future);
    }
}

/// Builds a libp2p transport with the following features:
/// - TCP connectivity over the Tor network
/// - DNS name resolution
/// - Authentication via secio
/// - Multiplexing via yamux or mplex
fn build_transport(
    keypair: Keypair,
    map: HashMap<Multiaddr, u16>,
) -> anyhow::Result<PingPongTransport> {
    let transport = TorTokioTcpConfig::new().nodelay(true).onion_map(map);
    let transport = DnsConfig::new(transport)?;

    let transport = transport
        .upgrade(Version::V1)
        .authenticate(SecioConfig::new(keypair))
        .multiplex(SelectUpgrade::new(
            yamux::Config::default(),
            MplexConfig::new(),
        ))
        .map(|(peer, muxer), _| (peer, StreamMuxerBox::new(muxer)))
        .timeout(Duration::from_secs(20))
        .boxed();

    Ok(transport)
}

/// libp2p `Transport` for the ping-pong application.
pub type PingPongTransport = Boxed<
    (PeerId, StreamMuxerBox),
    TransportTimeoutError<
        EitherError<
            EitherError<DnsErr<io::Error>, UpgradeError<SecioError>>,
            UpgradeError<EitherError<io::Error, io::Error>>,
        >,
    >,
>;
