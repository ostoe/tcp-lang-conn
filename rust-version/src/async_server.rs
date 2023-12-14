use crate::async_check_unit;
use crate::check_status::CheckError;
use std::net::SocketAddr;
use std::str::{FromStr, from_utf8};
use std::time::Duration;

use std::io::Write;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub async fn start_async_server(addr_port: &str) -> std::io::Result<()> {
    // -> Result<(), Box<dyn std::error::Error>> {
    // addr
    let listener = tokio::net::TcpListener::bind(addr_port)
        .await
        .expect("bind failed!");
    // let check_interval = tokio::time::interval(Duration::from_millis(300));
    let (stream_count_tx, mut stream_count_rx) = tokio::sync::mpsc::channel::<bool>(1024);

    tokio::spawn(async move {
        let mut check_interval = tokio::time::interval(Duration::from_millis(300));
        let mut not_release_tcpstream = 0u32;
        let mut stdout = std::io::stdout();
        loop {
            tokio::select! {
                _ = check_interval.tick() => {
                    print!("\rcurrent connection: \x1b[40;32m{}\x1b[0m", not_release_tcpstream);
                    stdout.flush().unwrap();
                }
                result = stream_count_rx.recv() => {
                    match result {
                    Some(r) => {
                        if r {not_release_tcpstream += 1;} else {not_release_tcpstream -= 1;}
                    },
                    None => {break;}
                    }
                }
            }
        }
    });

    //
    loop {
        let (mut stream, _addr) = listener.accept().await?;
        if let Err(_) = stream_count_tx.send(true).await {
            println!("receiver dropped");
            return Ok(());
        }
        let stream_count_tx_c = stream_count_tx.clone();
        tokio::spawn(async move  {
            // new connection: rev Ping Send Pong
            let mut buf = [0u8; 1024];
            // let n = stream.read(&mut buf).await;
            // let (mut reader, writer) = stream.split();
            match stream.read(&mut buf).await {
                Ok(size) => {
                    if size != 0 {
                        println!("recv: {}", from_utf8(&buf[..size]).unwrap_or_default());
                    } else {
                        println!("read closed. reached EOF, maybe[FIN]");
                        return
                    }
                }
                Err(e) => {
                    println!("first read error:{}", e.kind());
                    return
                }
            }
            // writer.write(&"Pong".as_bytes()).await;
            // let bytes_copied = tokio::io::copy("P", writer);
            
            match stream.write("Pong".as_bytes()).await {
                Ok(_) => {
                    println!("write back.")
                }
                Err(e) => {
                    println!("first write error:{}", e.kind());
                    return
                }
            }
            stream_count_tx_c.send(false).await.unwrap();
            
            // thread::sleep(Duration::from_secs(5));
            let start_time = std::time::Instant::now();
            let default_addr = SocketAddr::from_str("127.0.0.0:8001").unwrap();
            loop {
                tokio::time::interval(Duration::from_millis(300)).tick().await;
                // thread::sleep(check_interval);
                let check_result = async_check_unit::check_unit(&mut stream, start_time).await;
                match check_result.check_error {
                    CheckError::FIN => {
                        println!(
                            "[FIN] {} connection duration time: {:?}-------",
                            check_result.addr.unwrap_or(default_addr),
                            check_result.probe_time.unwrap_or(Default::default())
                        );
                    }
                    CheckError::RESET => {
                        println!(
                            "[RESET] {} connection duration time: {:?}-------",
                            "miss addr",
                            check_result.probe_time.unwrap_or(Default::default())
                        );
                    }
                    CheckError::EAGAIN | CheckError::Readed => {
                        continue;
                    } // checking...
                    CheckError::OtherErrno(e) => {
                        println!("[Check] others errno error: {:?}", e);
                    }
                    CheckError::ReadWriteError(e) => {
                        println!("[R/W Err] others read error: {:?}", e.kind());
                    }
                    _x => {
                        println!("____");
                        continue;
                    }
                }
                drop(stream);
                stream_count_tx_c.send(false).await.unwrap();
                return;
            }
        });

    }
}