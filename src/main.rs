use anyhow::Result;
use clap::Parser;
use std::net::{SocketAddr, Ipv4Addr};
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio_tun::Tun;
use pnet::packet::ipv4::MutableIpv4Packet;
// use pnet::packet::udp::MutableUdpPacket;

const MTU: usize = 1460;
const PORT: u16 = 40890;
const MAX_SIZE: usize = 1600;

#[derive(Parser, Debug)]
struct Arguments {
    #[clap(short, long, default_value_t=PORT)]
    port: u16,
    #[clap(short, long, default_value="192.168.1.1:40890")]
    remote: SocketAddr,
    #[clap(short, long, default_value = "udprelay")]
    tun_name: String,
}

fn tun_local_addr() -> Ipv4Addr {
    Ipv4Addr::new(169, 254, 1, 1)
}

fn tun_peer_addr() -> Ipv4Addr {
    Ipv4Addr::new(169, 254, 1, 2)
}

async fn build_tun(name: String) -> Result<Tun> {
    let tun = Tun::builder()
        .name(&name[..])
        .tap(false)
        .packet_info(false)
        .mtu(MTU as _)
        .address(tun_local_addr())
        .destination(tun_peer_addr())
        .up()
        .try_build()?;
    Ok(tun)
}

async fn build_udp(port: u16, peer: SocketAddr) -> Result<UdpSocket> {
    let sockaddr = "0.0.0.0:40890".parse::<SocketAddr>().unwrap();
    let sock = UdpSocket::bind(sockaddr).await?;
    sock.connect(peer).await?;
    Ok(sock)
}

async fn tun_task(sock: Arc<UdpSocket>, tun: Arc<Tun>) -> Result<()> {
    let mut buf = [0; MAX_SIZE];
    loop {
        let len = tun.recv(&mut buf).await?;
        println!("{:?} bytes received from Tun", len);

        // Reverse source and dest ip so that we can use same tunnel ip in both ends
        // No need to update checksum since we swapeed the values and not changed
        if let Some(mut pkt) = MutableIpv4Packet::new(&mut buf[..len]) {
            let source = pkt.get_source();
            pkt.set_source(pkt.get_destination());
            pkt.set_destination(source);
        } else {
            continue;
        }

        let _ = sock.send(&buf[..len]).await?;
    }
}

async fn udp_task(sock: Arc<UdpSocket>, tun: Arc<Tun>) -> Result<()> {
    let mut buf = [0; MAX_SIZE];
    loop {
        let (len, addr) = sock.recv_from(&mut buf).await?;
        println!("{:?} bytes received from {:?}", len, addr);

        let _ = tun.send(&buf[..len]).await?;
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Arguments::parse();

    let sock = build_udp(args.port, args.remote).await?;
    let tun = build_tun(args.tun_name).await?;

    let sock = Arc::new(sock);
    let tun = Arc::new(tun);

    let a = tokio::spawn(udp_task(sock.clone(), tun.clone()));
    let b = tokio::spawn(tun_task(sock, tun));

    let _ = tokio::join!(a, b);
    Ok(())
}
