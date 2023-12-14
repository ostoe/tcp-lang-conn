use std::net::SocketAddr;
use std::str::{FromStr, from_utf8};
use nix::sys::socket::{MsgFlags, recv};
use tokio::io::AsyncReadExt;
use std::os::unix::io::AsRawFd;
use nix::errno;
use crate::check_status::{WrapperMessage, CheckError};

pub(crate) async fn check_unit(stream: &mut tokio::net::TcpStream,  start_time: std::time::Instant) -> WrapperMessage {
    let mut buf:[u8;1024] = [0;1024];
    let default_addr = SocketAddr::from_str("127.0.0.0:8001").unwrap();
    let addr = stream.peer_addr().unwrap_or(default_addr);


    let mut peek_buf = [0u8; 1];

    match  recv(stream.as_raw_fd(), &mut peek_buf, MsgFlags::MSG_PEEK | MsgFlags::MSG_DONTWAIT ) {
        Ok(size) => {
            let duration_time = std::time::Instant::now().duration_since(start_time);
            if size == 0 {
                // println!("[{:?}] closed[FIN]", stream.peer_addr());
                return WrapperMessage{
                    content: "".to_string(),
                    thread_index: None,
                    probe_time: Some(duration_time),
                    check_error: CheckError::FIN
                };

            } else {
                // normal
                match stream.read(&mut buf).await {
                    Ok(size) => {
                        if size != 0 {
                            println!("recv from: {}: {}",addr.to_string(),
                                     from_utf8(&buf[..size]).unwrap_or_default());
                            return WrapperMessage{
                                content: "".to_string(),
                                thread_index: None,
                                probe_time: Some(duration_time),
                                check_error: CheckError::Readed };
                        } else {
                            return WrapperMessage{
                                content: "-------reached EOF, maybe[FIN]".to_string(),
                                thread_index: None,
                                probe_time: Some(duration_time),
                                check_error: CheckError::FIN };
                        }
                    }
                    Err(e) => {
                        return WrapperMessage{
                            content: "-------reached EOF, maybe[FIN]".to_string(),
                            thread_index: None,
                            probe_time: Some(duration_time),
                            check_error: CheckError::ReadWriteError(e) };
                    }

                }
            }
        }
        Err(e) => {
            let duration_time = std::time::Instant::now().duration_since(start_time);
            match e {
                errno::Errno::EAGAIN  => {
                    return WrapperMessage{
                        content: "".to_string(),
                        thread_index: None,
                        probe_time: None,
                        check_error: CheckError::EAGAIN };
                }
                errno::Errno::ECONNRESET => {
                    let duration_time = std::time::Instant::now().duration_since(start_time);
                    return WrapperMessage{
                        content: "-------reached EOF, maybe[RESET]".to_string(),
                        thread_index: None,
                        probe_time: Some(duration_time),
                        check_error: CheckError::RESET };
                }
                others => {
                    return WrapperMessage{
                        content: "-------reached EOF, maybe[FIN]".to_string(),
                        thread_index: None,
                        probe_time: Some(duration_time),
                        check_error: CheckError::OtherErrno(others) };

                }
            }
        }
    }

}
