Rust-libp2p TCP/IP via a SOCKS5 proxy
=====================================

Contains an implementation of the
[rust-libp2p](https://github.com/libp2p/rust-libp2p) `Transport` that
can be used to redirect traffic over a SOCKS5 proxy.

Currently functionality is limited to using the `Tor` daemon as the
SOCKS5 proxy (see below for reason).

Usage
-----

See `examples/ping.rs` for a complete running example using Tor.

SOCKS5 vs Tor
-------------

This repository is named `SOCKS5` instead of `Tor` because the only
thing that makes it Tor specific is the address handling. We convert
the Multiaddr to a string in the format that the Tor daemon expects.
Other SOCKS5 proxies could be used if this address conversion logic
was abstracted away.
