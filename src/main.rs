use std::net::{SocketAddr, IpAddr};
use tokio::{net::{TcpListener, TcpStream}, io::{AsyncReadExt, AsyncWriteExt}};
use network_interface::NetworkInterface;
use network_interface::NetworkInterfaceConfig;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("starting proxy server");
    let listener = TcpListener::bind("0.0.0.0:18887").await?;

    loop {
        let (socket, address) = listener.accept().await?;

        println!("[{}] connected", address);

        tokio::spawn(async move {
            if let Err(e) = handle_client(socket).await {
                println!("[{}] an error occured; error = {:?}", address, e);
            }
        });
    }
    
}

const RESOLVE_LOCAL: bool = true;

async fn handle_client(mut socket: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
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
            let mut bert = IpAddr::from(ip);
            if RESOLVE_LOCAL {
                let network_interfaces = NetworkInterface::show();
                if network_interfaces.is_ok() {
                    let network_interfaces = network_interfaces.unwrap();
                    for itf in network_interfaces.iter() {
                        itf.addr.iter().for_each(|addr| {
                            if addr.ip() == bert {
                                println!("resolved [{}] internal to [{}] v5", addr.ip(), bert);
                                bert = IpAddr::from([127, 0, 0, 1]);
                            }
                        });
                    }
                }
            }
            SocketAddr::from((IpAddr::from(bert), port))
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
            println!("{:?} connection failed", address);
            socket.write_u8(5).await?;
            socket.write_u8(4).await?;
            socket.write_u8(0).await?;
            socket.write_u8(address_type).await?;
            socket.write_all(ipbytes.as_slice()).await?;
            socket.write_u16(port).await?;
            return Err(format!("proxy connection to [{:?}] connection failed v5", address).into());
        }

        let mut tcp_stream = tcp_stream.unwrap();

        socket.write_u8(5).await?;
        socket.write_u8(0).await?;
        socket.write_u8(0).await?;
        socket.write_u8(address_type).await?;
        socket.write_all(ipbytes.as_slice()).await?;
        socket.write_u16(port).await?;
        
        println!("connect [{:?}] to [{:?}] v5", socket, address);

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


        let mut bert = IpAddr::from(ip);
        
        if RESOLVE_LOCAL {
            let network_interfaces = NetworkInterface::show();
            if network_interfaces.is_ok() {
                let network_interfaces = network_interfaces.unwrap();
                for itf in network_interfaces.iter() {
                    itf.addr.iter().for_each(|addr| {
                        if addr.ip() == bert {
                            println!("resolved [{}] internal to [{}] v4", addr.ip(), bert);
                            bert = IpAddr::from([127, 0, 0, 1]);
                        }
                    });
                }
            }
        }
        


        let address = SocketAddr::from((IpAddr::from(bert), port));
        let tcp_stream = TcpStream::connect(address).await;
        if tcp_stream.is_err() {
            socket.write_u8(0).await?;
            socket.write_u8(92).await?;
            socket.write_u16(port).await?;
            socket.write_all(&ip).await?;
            return Err(format!("proxy connection to [{:?}] connection failed v4", address).into());
        } 
        
        let mut tcp_stream = tcp_stream.unwrap();

        socket.write_u8(0).await?;
        socket.write_u8(90).await?;
        socket.write_u16(port).await?;
        socket.write_all(&ip).await?;

        println!("connect [{:?}] to [{:?}] v4", socket, address);
        
        tokio::io::copy_bidirectional(&mut socket, &mut tcp_stream).await?;
    } else {
        return Err(format!("version not supported {}", version).into());
    }
    Ok(())

}
