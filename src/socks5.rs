//! SOCKS Version 5 Client

use std::{io, net::SocketAddr, sync::Arc, time::Duration};

use log::{debug, error, info, trace};
use shadowsocks::{
    context::SharedContext,
    net::{AcceptOpts, ConnectOpts, TcpListener, TcpStream as ShadowTcpStream},
    relay::socks5::{
        Address, Command, HandshakeRequest, HandshakeResponse, Reply, TcpRequestHeader,
        TcpResponseHeader, SOCKS5_AUTH_METHOD_NONE,
    },
    ServerAddr,
};
use tokio::{net::TcpStream as TokioTcpStream, time};

pub struct Socks5Server {
    context: SharedContext,
    listener: TcpListener,
    proxy_addr: ServerAddr,
    remote_addr: Address,
    connect_opts: ConnectOpts,
}

impl Socks5Server {
    pub async fn bind(
        context: SharedContext,
        host_addr: SocketAddr,
        proxy_addr: ServerAddr,
        remote_addr: Address,
        accept_opts: AcceptOpts,
        connect_opts: ConnectOpts,
    ) -> io::Result<Socks5Server> {
        let listener = TcpListener::bind_with_opts(&host_addr, accept_opts).await?;
        Ok(Socks5Server {
            context,
            listener,
            proxy_addr,
            remote_addr,
            connect_opts,
        })
    }

    pub async fn run(self) {
        let proxy_addr = Arc::new(self.proxy_addr);
        let remote_addr = Arc::new(self.remote_addr);
        let connect_opts = Arc::new(self.connect_opts);

        info!(
            "socks5 service started, listening on {}",
            self.listener.local_addr().unwrap()
        );

        loop {
            let (client_stream, client_addr) = match self.listener.accept().await {
                Ok(v) => v,
                Err(err) => {
                    error!("failed to accept, error: {}", err);
                    time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            };

            trace!("accepted socks5 tcp client {}", client_addr);

            let context = self.context.clone();
            let proxy_addr = proxy_addr.clone();
            let remote_addr = remote_addr.clone();
            let connect_opts = connect_opts.clone();
            tokio::spawn(async move {
                if let Err(err) = handle_socks5_client(
                    context,
                    client_stream,
                    client_addr,
                    proxy_addr,
                    remote_addr,
                    connect_opts,
                )
                .await
                {
                    error!("handle socks5 tcp client failed, error: {}", err);
                }
            });
        }
    }
}

async fn handle_socks5_client(
    context: SharedContext,
    mut client_stream: TokioTcpStream,
    client_addr: SocketAddr,
    proxy_addr: Arc<ServerAddr>,
    remote_addr: Arc<Address>,
    connect_opts: Arc<ConnectOpts>,
) -> io::Result<()> {
    // 1. Connect Proxy with TCP
    let mut proxy_stream =
        ShadowTcpStream::connect_server_with_opts(&context, &proxy_addr, &connect_opts).await?;

    // 2. SOCKS 5 Handshake
    // XXX: Only NONE method is supported currently.
    let handshake_request = HandshakeRequest::new(vec![SOCKS5_AUTH_METHOD_NONE]);
    handshake_request.write_to(&mut proxy_stream).await?;

    let handshake_response = HandshakeResponse::read_from(&mut proxy_stream).await?;
    if handshake_response.chosen_method != SOCKS5_AUTH_METHOD_NONE {
        error!(
            "socks5 only NONE method is supported, but chosen method is {}",
            handshake_response.chosen_method
        );
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "unsupported socks5 method",
        ));
    }

    // 3. TCP Connect
    let tcp_request = TcpRequestHeader::new(Command::TcpConnect, remote_addr.as_ref().clone());
    tcp_request.write_to(&mut proxy_stream).await?;

    let tcp_response = TcpResponseHeader::read_from(&mut proxy_stream).await?;
    if !matches!(tcp_response.reply, Reply::Succeeded) {
        error!("socks5 tcp request failed, {:?}", tcp_response);
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "socks5 tcp request failed",
        ));
    }

    debug!(
        "socks5 tcp tunnel established, {} <-> {} via {}",
        client_addr, remote_addr, proxy_addr
    );

    // 4. Establish a tunnel
    tokio::io::copy_bidirectional(&mut client_stream, &mut proxy_stream).await?;

    Ok(())
}
