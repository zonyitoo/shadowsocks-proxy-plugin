//! HTTP Proxy client server

use std::{io, net::SocketAddr, sync::Arc, time::Duration};

use log::{debug, error, trace};
use shadowsocks::{
    context::SharedContext,
    net::{AcceptOpts, ConnectOpts, TcpListener, TcpStream as ShadowTcpStream},
    relay::Address,
    ServerAddr,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream as TokioTcpStream,
    time,
};

pub struct HttpServer {
    context: SharedContext,
    listener: TcpListener,
    proxy_addr: ServerAddr,
    remote_addr: Address,
    connect_opts: ConnectOpts,
}

impl HttpServer {
    pub async fn bind(
        context: SharedContext,
        host_addr: SocketAddr,
        proxy_addr: ServerAddr,
        remote_addr: Address,
        accept_opts: AcceptOpts,
        connect_opts: ConnectOpts,
    ) -> io::Result<HttpServer> {
        let listener = TcpListener::bind_with_opts(&host_addr, accept_opts).await?;
        Ok(HttpServer {
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

        loop {
            let (client_stream, client_addr) = match self.listener.accept().await {
                Ok(v) => v,
                Err(err) => {
                    error!("failed to accept, error: {}", err);
                    time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            };

            trace!("accepted http tcp client {}", client_addr);

            let context = self.context.clone();
            let proxy_addr = proxy_addr.clone();
            let remote_addr = remote_addr.clone();
            let connect_opts = connect_opts.clone();
            tokio::spawn(async move {
                if let Err(err) = handle_http_client(
                    context,
                    client_stream,
                    client_addr,
                    proxy_addr,
                    remote_addr,
                    connect_opts,
                )
                .await
                {
                    error!("handle http tcp client failed, error: {}", err);
                }
            });
        }
    }
}

async fn handle_http_client(
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

    // 2. CONNECT remote
    let http_request = format!(
        "CONNECT {} HTTP/1.1\r\n\
         Host: {}\r\n\
         \r\n",
        remote_addr, proxy_addr
    );
    proxy_stream.write_all(http_request.as_bytes()).await?;

    let mut http_response_buffer = Vec::new();

    let mut http_headers = [httparse::EMPTY_HEADER; 64];
    let mut http_response = httparse::Response::new(&mut http_headers);

    {
        let mut buf_proxy_stream = BufReader::new(&mut proxy_stream);
        loop {
            buf_proxy_stream
                .read_until(b'\n', &mut http_response_buffer)
                .await?;

            if http_response_buffer.ends_with(b"\r\n\r\n") {
                break;
            }
        }

        match http_response.parse(http_response_buffer.as_slice()) {
            Ok(httparse::Status::Complete(_)) => {}
            Ok(httparse::Status::Partial) => {
                return Err(io::Error::new(io::ErrorKind::Other, "http parse partial"));
            }
            Err(err) => {
                error!("http connect response parse failed, error: {}", err);
                return Err(io::Error::new(io::ErrorKind::Other, err));
            }
        }
    }

    if http_response.code != Some(200) {
        error!("http connect failed. {:?}", http_response);
        return Err(io::Error::new(io::ErrorKind::Other, "http connect failed"));
    }

    debug!(
        "http tcp tunnel established, {} <-> {} via {}",
        client_addr, remote_addr, proxy_addr
    );

    // 4. Establish a tunnel
    tokio::io::copy_bidirectional(&mut client_stream, &mut proxy_stream).await?;

    Ok(())
}
