use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use tokio;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use wg_p2p::parse::parse_ini;
use wg_p2p::unmap_addr;

#[derive(Debug, Clone, Default)]
struct PeerInfo {
    addrs: Vec<std::net::SocketAddr>,
    contacts: Vec<String>,
}

async fn serve(peers: HashMap<IpAddr, String>) -> anyhow::Result<()> {
    let mut state = std::collections::HashMap::<String, PeerInfo>::new();
    for (_, p) in peers.iter() {
        state.insert(p.clone(), PeerInfo::default());
    }
    let state = std::sync::Arc::new(tokio::sync::RwLock::new(state));
    let server = tokio::net::TcpListener::bind("[::]:30567").await?;
    loop {
        let (client, addr) = server.accept().await?;
        eprintln!("{:?} connected", addr);
        let ip = addr.ip();
        let ip = unmap_addr(ip);
        if let Some(client_pubkey) = peers.get(&ip) {
            let client_pubkey = client_pubkey.to_owned();
            let state = std::sync::Arc::clone(&state);
            tokio::task::spawn(async move {
                let res = handle(client, client_pubkey, state).await;
                eprintln!("{:?} disconnected", addr);

                if let Err(err) = res {
                    eprintln!("{:?}", err);
                }
            });
        } else {
            eprintln!("{:?} unknown", addr);
            drop(client);
        }
    }
}

type StatePtr = std::sync::Arc<tokio::sync::RwLock<HashMap<String, PeerInfo>>>;

async fn handle(
    mut client: tokio::net::TcpStream,
    client_pubkey: String,
    state: StatePtr,
) -> anyhow::Result<()> {
    loop {
        let mut command_space = [0; 2];
        client.read_exact(&mut command_space).await?;
        match &command_space {
            b"? " => {
                let mut pubkey_nl = [0; 45];
                client.read_exact(&mut pubkey_nl).await?;
                let query_pubkey = std::str::from_utf8(&pubkey_nl[..44])?;
                let rstate = state.read().await;
                if let Some(info) = rstate.get(query_pubkey) {
                    let info = info.clone();
                    drop(rstate);
                    if info.contacts.contains(&client_pubkey) {
                        for a in info.addrs {
                            let mut line = a.to_string();
                            line.push('\n');
                            client.write_all(line.as_bytes()).await?;
                        }
                    }
                    let rstate = state.read().await;
                    if let Some(qinfo) = rstate.get(query_pubkey) {
                        if !qinfo.contacts.contains(&client_pubkey) {
                            drop(rstate);
                            state
                                .write()
                                .await
                                .get_mut(query_pubkey)
                                .unwrap()
                                .contacts
                                .push(client_pubkey.clone());
                        }
                    }
                    client.write_all(b"\n").await?;
                }
            }
            b"A " => {
                let mut line = String::new();
                loop {
                    if line.len() >= 1000 {
                        return Err(
                            std::io::Error::new(std::io::ErrorKind::Other, "too long").into()
                        );
                    }
                    let mut buf = [0];
                    client.read_exact(&mut buf).await?;
                    if buf[0] == b'\n' {
                        break;
                    }
                    line.push(buf[0] as char);
                }
                let mut addrs = vec![];
                for word in line.trim().split(' ') {
                    let addr = word.parse::<SocketAddr>()?;
                    addrs.push(addr);
                }
                state.write().await.get_mut(&client_pubkey).unwrap().addrs = addrs;
            }
            _ => {
                client.write_all(b"unknown command\n").await?;
            }
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Hello, world!");
    let peers = parse_ini("/etc/wireguard/wg_bruckbude.peers")?;
    println!("{:?}", peers);
    serve(peers).await?;
    Ok(())
}
