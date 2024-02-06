use anyhow::Result;
use clap::Parser;
use std::net::{SocketAddr, Ipv4Addr};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tokio::net::UdpSocket;
use tokio_tun::Tun;

const MTU: usize = 1460;
const PORT: u16 = 40890;
const MAX_SIZE: usize = 1600;
const FIXED_HDR_SIZE: usize = 20;

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
    Ipv4Addr::new(172, 16, 16, 172)
}

fn tun_peer_addr() -> Ipv4Addr {
    Ipv4Addr::new(172, 16, 16, 16)
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
        let len = tun.recv(&mut buf[FIXED_HDR_SIZE..]).await?;
        println!("{:?} bytes received from Tun", len);
        let _ = sock.send(&buf[..len+FIXED_HDR_SIZE]).await?;
    }
}

async fn udp_task(sock: Arc<UdpSocket>, tun: Arc<Tun>) -> Result<()> {
    let mut buf = [0; MAX_SIZE];
    loop {
        let (len, addr) = sock.recv_from(&mut buf).await?;
        println!("{:?} bytes received from {:?}", len, addr);
        let len = len - FIXED_HDR_SIZE;

        let _ = tun.send(&buf[FIXED_HDR_SIZE..len]).await?;
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Arguments::parse();

    let sock = build_udp(args.port, args.remote).await?;
    let tun = build_tun(args.tun_name).await?;

    let sock = Arc::new(sock);
    let tun = Arc::new(tun);

    tokio::spawn(udp_task(sock.clone(), tun.clone()));
    tokio::spawn(tun_task(sock, tun));

    sleep(Duration::from_secs(60)).await;
    Ok(())
}
