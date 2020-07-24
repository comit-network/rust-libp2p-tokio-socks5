Rust-libp2p TCP/IP via a SOCKS5 proxy
=====================================

Contains an implementation of the
[rust-libp2p](https://github.com/libp2p/rust-libp2p) `Transport` that
can be used to redirect traffic over a SOCKS5 proxy.

Provides the `Socks5TokioTcpConfig` type which can be use when
building a swarm.

Example transport creation:
```
/// Builds a libp2p transport with the following features:
/// - TCP connectivity over the Tor network
/// - DNS name resolution
/// - Authentication via secio
/// - Multiplexing via yamux or mplex
fn build_transport(
    keypair: Keypair,
    addr: Multiaddr,
) -> anyhow::Result<PingPongTransport> {
    let mut map = HashMap::new();
    map.insert(addr, LOCAL_PORT);

    let tcp = Socks5TokioTcpConfig::default().nodelay(true).onion_map(map);
    let transport = DnsConfig::new(tcp)?;

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
```

Currently functionality is limited to using the `Tor` daemon as the
SOCKS5 proxy (see below for reason).

Usage
-----

See `examples/ping.rs` for a complete running example using Tor.

You should configure your Tor onion service to redirect traffic to
some local port. The onion address and port will be needed when
creating the transport as show above.

Example Tor configuration:

     HiddenServiceDir /var/lib/tor/hidden_service/
     HiddenServicePort 7 127.0.0.1:7777

Check the hidden service data directory for a file called `hostname`,
this contains the onion address for the service.

SOCKS5 vs Tor
-------------

This repository is named `SOCKS5` instead of `Tor` because technically
there is no reason to be `Tor` specific. In actuality the transport
created with `Socks5TokioTcpConfig` will only currently work with a
`Tor` proxy because of the address munging we do before passing the
target address to the proxy. PRs welcome, please see
https://github.com/comit-network/rust-libp2p-tokio-socks5/issues/1.
