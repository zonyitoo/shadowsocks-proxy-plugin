use std::{
    env,
    io::{self, IsTerminal},
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

#[cfg(feature = "mimalloc")]
use mimalloc::MiMalloc;

use opt::ProxyProtocol;
use shadowsocks::{config::ServerType, context::Context, lookup_then, relay::Address};
use time::UtcOffset;
use tracing_subscriber::{filter::EnvFilter, fmt::time::OffsetTime, FmtSubscriber};

use self::http::HttpServer;
use self::opt::PluginOpts;
use self::socks5::Socks5Server;

mod http;
mod opt;
mod socks5;

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[tokio::main]
async fn main() -> io::Result<()> {
    let mut builder = FmtSubscriber::builder()
        .with_level(true)
        .with_timer(match OffsetTime::local_rfc_3339() {
            Ok(t) => t,
            Err(..) => {
                // Reinit with UTC time
                OffsetTime::new(
                    UtcOffset::UTC,
                    time::format_description::well_known::Rfc3339,
                )
            }
        })
        .with_env_filter(EnvFilter::from_default_env())
        .compact();

    // NOTE: ansi is enabled by default.
    // Could be disabled by `NO_COLOR` environment variable.
    // https://no-color.org/
    if !std::io::stdout().is_terminal() {
        builder = builder.with_ansi(false);
    }

    builder.init();

    let remote_host = env::var("SS_REMOTE_HOST").expect("require SS_REMOTE_HOST");
    let remote_port = env::var("SS_REMOTE_PORT").expect("require SS_REMOTE_PORT");
    let local_host = env::var("SS_LOCAL_HOST").expect("require SS_LOCAL_HOST");
    let local_port = env::var("SS_LOCAL_PORT").expect("require SS_LOCAL_PORT");

    let remote_port = remote_port
        .parse::<u16>()
        .expect("SS_REMOTE_PORT must be a valid port");
    let local_port = local_port
        .parse::<u16>()
        .expect("SS_LOCAL_PORT must be a valid port");

    let mut plugin_opts = PluginOpts::default();
    if let Ok(opts) = env::var("SS_PLUGIN_OPTIONS") {
        plugin_opts = PluginOpts::from_str(&opts).expect("unrecognized SS_PLUGIN_OPTIONS");
    }

    let mut context = Context::new(ServerType::Local);
    context.set_ipv6_first(plugin_opts.ipv6_first.unwrap_or(false));
    let context = Arc::new(context);

    let remote_addr = match remote_host.parse::<IpAddr>() {
        Ok(remote_ip) => Address::SocketAddress(SocketAddr::new(remote_ip, remote_port)),
        Err(_) => Address::DomainNameAddress(remote_host, remote_port),
    };

    let accept_opts = plugin_opts.as_accept_opts();
    let connect_opts = plugin_opts.as_connect_opts();

    match plugin_opts.proxy_protocol {
        ProxyProtocol::Socks5 => {
            let (_, server) = lookup_then!(context, &local_host, local_port, |host_addr| {
                Socks5Server::bind(
                    context.clone(),
                    host_addr,
                    plugin_opts.proxy_addr.clone(),
                    remote_addr.clone(),
                    accept_opts.clone(),
                    connect_opts.clone(),
                )
                .await
            })?;
            server.run().await;
        }
        ProxyProtocol::Http => {
            let (_, server) = lookup_then!(context, &local_host, local_port, |host_addr| {
                HttpServer::bind(
                    context.clone(),
                    host_addr,
                    plugin_opts.proxy_addr.clone(),
                    remote_addr.clone(),
                    accept_opts.clone(),
                    connect_opts.clone(),
                )
                .await
            })?;
            server.run().await;
        }
    }

    Ok(())
}
