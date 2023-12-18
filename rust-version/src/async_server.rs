use crate::async_check_unit;
use crate::check_status::CheckError;
use socket2::{SockRef, TcpKeepalive};
use std::net::TcpListener;
use std::os::fd::{AsFd, AsRawFd};
use std::str::{from_utf8, FromStr};
use std::time::Duration;

use std::io::Write;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub async fn start_async_server(addr_port: &str) -> std::io::Result<()> {
    // -> Result<(), Box<dyn std::error::Error>> {
    // addr

    let listener = match tokio::net::TcpListener::bind(addr_port).await {
        Ok(l) => l,
        Err(e) => {
            println!("[{}]: {}", "Bind Port Failed", e.kind());
            std::process::exit(1);
        }
    };
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

//   first_conn_keepalive_handle();
    //
    loop {
        // stream1.peer_addr();
        let (mut stream, _addr) = listener.accept().await?;
        if let Err(_) = stream_count_tx.send(true).await {
            println!("receiver dropped");
            return Ok(());
        }
        let stream_count_tx_c = stream_count_tx.clone();
        tokio::spawn(async move {
            let default_addr = std::net::SocketAddr::from_str("127.0.0.0:8001").unwrap();
            let addr = stream.peer_addr().unwrap_or(default_addr);
            // new connection: rev Ping Send Pong
            let mut buf = [0u8; 1024];
            // let n = stream.read(&mut buf).await;
            // let (mut reader, writer) = stream.split();
            match stream.read(&mut buf).await {
                Ok(size) => {
                    if size != 0 {
                        print!("<<< {} | ", from_utf8(&buf[..size]).unwrap_or_default());
                    } else {
                        println!("read closed. reached EOF, maybe[FIN]");
                        return;
                    }
                }
                Err(e) => {
                    println!("first read error:{}", e.kind());
                    return;
                }
            }
            // writer.write(&"Pong".as_bytes()).await;
            // let bytes_copied = tokio::io::copy("P", writer);

            match stream.write("Server Hello".as_bytes()).await {
                Ok(_) => {
                    println!("{} >>>", "Server Hello")
                }
                Err(e) => {
                    println!("first write error:{}", e.kind());
                    return;
                }
            }
            // stream_count_tx_c.send(false).await.unwrap();

            // thread::sleep(Duration::from_secs(5));
            let start_time = std::time::Instant::now();
            loop {
                tokio::time::interval(Duration::from_millis(300))
                    .tick()
                    .await;
                // thread::sleep(check_interval);
                let check_result = async_check_unit::check_unit(&mut stream, start_time).await;
                match check_result.check_error {
                    CheckError::FIN => {
                        println!(
                            "[FIN] {} connection duration time: {:?}-------",
                            addr,
                            check_result.probe_time.unwrap_or(Default::default())
                        );
                    }
                    CheckError::RESET => {
                        println!(
                            "[RESET] {} connection duration time: {:?}-------",
                            addr,
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


async fn first_conn_keepalive_handle(listener: tokio::net::TcpListener) {
       // 首个keepalive包，做通讯
    let (mut stream1, _addr) = listener.accept().await.unwrap();
        let (stream, _addr) =  listener.accept().await.unwrap();
    let stream: std::net::TcpStream = stream.into_std().unwrap();
    let socket: socket2::Socket = socket2::Socket::from(stream);
    let keepalive = TcpKeepalive::new()
                    .with_time(Duration::from_secs(4))
                    .with_interval(Duration::from_secs(1))
                    .with_retries(4);
    socket.set_tcp_keepalive(&keepalive).unwrap();
    let stream: std::net::TcpStream = socket.into();
    let stream: tokio::net::TcpStream = tokio::net::TcpStream::from_std(stream).unwrap();


    // pub fn to_socket(stream: &TcpStream) -> TcpSocket {
    //     use std::os::unix::io::{AsRawFd, FromRawFd};
    //     let fd = stream.as_raw_fd();
    //     let dup_fd = unsafe { libc::dup(fd) };
    //     unsafe { TcpSocket::from_raw_fd(dup_fd) }
    // }
    // let socket = to_socket(&stream);
    // socket.set_keepalive_params( TcpKeepalive::new().with_time(Duration::from_secs(10)) );
         

    // set keepalive
    // let sock_ref = socket2::SockRef::from(&stream1);
    // let ka: socket2::TcpKeepalive = socket2::TcpKeepalive::new()
    //     // [#cfg!([target_os="unix"])]
    //     .with_time(Duration::from_secs(10)) // KEEPALIVE_TIME
    //     .with_interval(Duration::from_secs(1)); // TCP_KEEPINTVL
    //                                             // .with_retries(1024); // TCP_KEEPCNT
    // sock_ref.set_tcp_keepalive(&ka).unwrap();

    unsafe {
        // set_keep_alive.==true
        let fd = stream1.as_raw_fd();
        let a = libc::setsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_KEEPALIVE,
            1 as *const u32 as *const core::ffi::c_void,
            std::mem::size_of::<core::ffi::c_int>() as u32,
        );
        if a != 0 {
            println!("libc set sockopt **SO_KEEPALIVE** error: {}", a);
        }

        // set KEEPALIVE_TIME   idetime
        let a = libc::setsockopt(
            fd,
            libc::IPPROTO_TCP,
            0x10, //libc::TCP_KEEPALIVE,
            10 as *const u32 as *const core::ffi::c_void, //sec
            std::mem::size_of::<core::ffi::c_int>() as u32,

        );
        if a != 0 {
            println!("libc set sockopt **TCP_KEEPALIVE** error: {}", a);
        }

        // set TCP_KEEPINTVL 
        let a = libc::setsockopt(
            fd,
            libc::IPPROTO_TCP,
            libc::TCP_KEEPINTVL,
            10 as *const libc::c_void, // sec
            std::mem::size_of::<core::ffi::c_int>() as u32,

        );
        if a != 0 {
            println!("libc set sockopt **TCP_KEEPINTVL** error: {}", a);
        }

        // TCP_KEEPCNT
        let a = libc::setsockopt(
            fd,
            libc::IPPROTO_TCP, //0x06, //
            libc::TCP_KEEPCNT, // optname: TcpUserTimeout
            65534 as *const libc::c_void, // count
            std::mem::size_of::<core::ffi::c_int>() as u32,
        );
        if a != 0 {
            println!("libc set sockopt **TCP_KEEPCNT** error: {}", a);
        }
    }

    let mut buf = [0u8; 1024];
    match stream1.read(&mut buf).await {
        Ok(size) => {
            if size != 0 {
                print!("<<< {} | ", from_utf8(&buf[..size]).unwrap_or_default());
            }
        }
        Err(e) => {
            println!("first read error:{}", e.kind());
        }
    }
    match stream1.write("Server Hello".as_bytes()).await {
        Ok(_) => {
            println!("{} >>>", "Server Hello");
            // stream_count_tx.send(true).await;
        }
        Err(e) => {
            println!("first write error:{}", e.kind());
        }
    }

}