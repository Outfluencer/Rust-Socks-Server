use std::net::{SocketAddr, IpAddr};
use tokio::{net::{TcpListener, TcpStream}, io::{AsyncReadExt, AsyncWriteExt}};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut bind = String::from("0.0.0.0:18887");
    for arg in std::env::args() {
        if arg.starts_with("bind=") {
            bind = arg.split("=").collect::<Vec<&str>>()[1].to_string();
        }
    }
    let listener = TcpListener::bind(bind).await?;
    loop {
        let (socket, address) = listener.accept().await?;
        tokio::spawn(async move {
            if let Err(_) = handle_client(socket, &address).await {
            }
        });
    }
}


async fn handle_client(mut socket: TcpStream, address_from: &SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    let version = socket.read_u8().await?;
    if version == 5 {
        let nmethods = socket.read_u8().await?;
        let mut methods = vec![0u8; nmethods as usize];
        socket.read_exact(&mut methods).await?;
        let mut method = 255u8;
        for m in methods {
            if m == 0 {
                method = 0;
                break;
            }
        }
        if method == 0 {
            socket.write_u8(5).await?;
            socket.write_u8(0).await?;
        } else {
            socket.write_u8(5).await?;
            socket.write_u8(255).await?;
        }
        let version = socket.read_u8().await?;
        if version != 5 {
            return Err(format!("version not supported {} v5", version).into());
        }
        let command = socket.read_u8().await?;
        if command != 1 {
            return Err(format!("command not supported {} v5", command).into());
        }
        let _ = socket.read_u8().await?;
        let address_type = socket.read_u8().await?;
        let ipbytes: Vec<u8>;
        let port;
        let address = if address_type == 1 {
            let mut ip = [0u8; 4];
            socket.read_exact(&mut ip).await?;
            ipbytes = ip.to_vec();
            port = socket.read_u16().await?;
            let ip_addr = IpAddr::from(ip);
            SocketAddr::from((IpAddr::from(ip_addr), port))
        } else if address_type == 3 {
            let mut len = [0u8; 1];
            socket.read_exact(&mut len).await?;
            let mut domain = vec![0u8; len[0] as usize];
            socket.read_exact(&mut domain).await?;
            ipbytes = domain.to_vec();
            let domain = String::from_utf8(domain)?;
            port = socket.read_u16().await?;
            let address = format!("{}:{}", domain, port);
            let a = tokio::net::lookup_host(address).await?.next();
            if a.is_none() {
                return Err(format!("could not lookup domain {} v5", domain).into());
            }
            a.unwrap()
        } else if address_type == 4 {
            let mut ip = [0u8; 16];
            socket.read_exact(&mut ip).await?;
            ipbytes = ip.to_vec();
            port = socket.read_u16().await?;
            SocketAddr::from((IpAddr::from(ip), port))
        } else {
            return Err(format!("address type not supported {} v5", address_type).into());
        };
        let tcp_stream = TcpStream::connect(address).await;
        if tcp_stream.is_err() {
            socket.write_u8(5).await?;
            socket.write_u8(4).await?;
            socket.write_u8(0).await?;
            socket.write_u8(address_type).await?;
            socket.write_all(ipbytes.as_slice()).await?;
            socket.write_u16(port).await?;
            return Err(format!("proxy connection to [{}] failed v5", address).into());
        }
        let mut tcp_stream = tcp_stream.unwrap();
        socket.write_u8(5).await?;
        socket.write_u8(0).await?;
        socket.write_u8(0).await?;
        socket.write_u8(address_type).await?;
        socket.write_all(ipbytes.as_slice()).await?;
        socket.write_u16(port).await?;
        tokio::io::copy_bidirectional(&mut socket, &mut tcp_stream).await?;
    }
    else if version == 4 {
        let command = socket.read_u8().await?;
        if command != 1 {
            return Err(format!("command not supported {}", command).into());
        }
        let port = socket.read_u16().await?;
        let mut ip = [0u8; 4];
        socket.read_exact(&mut ip).await?;
        loop {
            let byte = socket.read_u8().await?;
            if byte == 0 {
                break;
            }
        }
        let ip_addr = IpAddr::from(ip);
        let address = SocketAddr::from((IpAddr::from(ip_addr), port));
        let tcp_stream = TcpStream::connect(address).await;
        if tcp_stream.is_err() {
            socket.write_u8(0).await?;
            socket.write_u8(92).await?;
            socket.write_u16(port).await?;
            socket.write_all(&ip).await?;
            return Err(format!("proxy connection to [{}] failed v4", address).into());
        } 
        let mut tcp_stream = tcp_stream.unwrap();
        socket.write_u8(0).await?;
        socket.write_u8(90).await?;
        socket.write_u16(port).await?;
        socket.write_all(&ip).await?;
        tokio::io::copy_bidirectional(&mut socket, &mut tcp_stream).await?;
    } else {
        return Err(format!("version not supported {}", version).into());
    }
    Ok(())
}
