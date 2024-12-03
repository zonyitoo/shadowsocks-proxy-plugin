//! Plugin Options

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use serde_urlencoded::{self, de::Error as DeError};
use shadowsocks::{
    net::{AcceptOpts, ConnectOpts},
    ServerAddr,
};

/// Proxy's Protocol
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProxyProtocol {
    /// SOCKS Version 5
    #[default]
    Socks5,
    /// HTTP Proxy Protocol (HTTP/1.1)
    Http,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PluginOpts {
    /// Set `SO_MARK` socket option for outbound sockets
    #[cfg(any(target_os = "linux", target_os = "android"))]
    pub outbound_fwmark: Option<u32>,
    /// Set `SO_USER_COOKIE` socket option for outbound sockets
    #[cfg(target_os = "freebsd")]
    pub outbound_user_cookie: Option<u32>,
    /// Set `SO_BINDTODEVICE` (Linux), `IP_BOUND_IF` (BSD), `IP_UNICAST_IF` (Windows) socket option for outbound sockets
    pub outbound_bind_interface: Option<String>,
    /// Outbound sockets will `bind` to this address
    pub outbound_bind_addr: Option<IpAddr>,
    /// UDP tunnel timeout (in seconds)
    pub udp_timeout: Option<u64>,
    /// TCP Keep Alive
    pub tcp_keep_alive: Option<u64>,
    /// TCP Fast Open
    pub tcp_fast_open: Option<bool>,
    /// MPTCP
    pub mptcp: Option<bool>,
    /// IPv6 First
    pub ipv6_first: Option<bool>,
    /// Proxy's Protocol
    pub proxy_protocol: ProxyProtocol,
    /// Proxy's Address
    pub proxy_addr: ServerAddr,
}

impl Default for PluginOpts {
    fn default() -> Self {
        PluginOpts {
            #[cfg(any(target_os = "linux", target_os = "android"))]
            outbound_fwmark: None,
            #[cfg(target_os = "freebsd")]
            outbound_user_cookie: None,
            outbound_bind_interface: None,
            outbound_bind_addr: None,
            udp_timeout: None,
            tcp_keep_alive: None,
            tcp_fast_open: None,
            mptcp: None,
            ipv6_first: None,
            proxy_protocol: ProxyProtocol::Socks5,
            proxy_addr: ServerAddr::SocketAddr(SocketAddr::new(
                Ipv4Addr::new(127, 0, 0, 1).into(),
                1080,
            )),
        }
    }
}

impl PluginOpts {
    pub fn from_str(opt: &str) -> Result<PluginOpts, DeError> {
        serde_urlencoded::from_str(opt)
    }

    pub fn as_connect_opts(&self) -> ConnectOpts {
        let mut connect_opts = ConnectOpts::default();

        #[cfg(any(target_os = "linux", target_os = "android"))]
        if let Some(outbound_fwmark) = self.outbound_fwmark {
            connect_opts.fwmark = Some(outbound_fwmark);
        }

        #[cfg(target_os = "freebsd")]
        if let Some(outbound_user_cookie) = self.outbound_user_cookie {
            connect_opts.user_cookie = Some(outbound_user_cookie);
        }

        connect_opts.bind_interface = self.outbound_bind_interface.clone();
        connect_opts.bind_local_addr = self.outbound_bind_addr.map(|ip| SocketAddr::new(ip, 0));

        connect_opts.tcp.keepalive = self.tcp_keep_alive.map(|sec| Duration::from_secs(sec));
        connect_opts.tcp.fastopen = self.tcp_fast_open.unwrap_or(false);
        connect_opts.tcp.mptcp = self.mptcp.unwrap_or(false);

        connect_opts
    }

    pub fn as_accept_opts(&self) -> AcceptOpts {
        let mut accept_opts = AcceptOpts::default();

        accept_opts.tcp.keepalive = self.tcp_keep_alive.map(|sec| Duration::from_secs(sec));
        accept_opts.tcp.fastopen = self.tcp_fast_open.unwrap_or(false);
        accept_opts.tcp.mptcp = self.mptcp.unwrap_or(false);

        accept_opts
    }
}
