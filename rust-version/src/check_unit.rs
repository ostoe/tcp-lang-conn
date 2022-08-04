use std::net::{TcpStream, SocketAddr};
use std::str::{FromStr, from_utf8};
use nix::sys::socket::{MsgFlags, recv};
use std::os::unix::io::AsRawFd;
use std::io::{Read, Write};
use nix::errno;
use crate::check_status::{WrapperMessage, CheckError};

// 返回false，代表立即返回不继续执行！
pub fn stream_rw_unit(stream: &mut TcpStream, is_server: bool, thread_index: usize) -> (bool, Option<CheckError>) {
    let mut buf = [0u8; 1024];
    let mut sequence_is_read_for_server = [true, false];
    let mut send_content= "Server hello".to_string();
    if !is_server {
        sequence_is_read_for_server = [false, true];
        send_content = format!("[{}]Client hello", thread_index);
    }
    for x in sequence_is_read_for_server {
        if x {
            match stream.read(&mut buf) {
                Ok(size) => {
                    if size != 0 {
                        println!("recv: {}", from_utf8(&buf[..size]).unwrap_or_default());
                    } else {
                        return (false, Some(CheckError::FIN));
                        // ("read closed. reached EOF, maybe[FIN]");
                    }
                }
                Err(e) => {
                    return (false, Some(CheckError::ReadWriteError(e)));
                    // println!("first read error:{}", e.kind());
                }
            }
        } else {
            // write back to clint: Server hello;
            match stream.write(send_content.as_ref()) {
                Ok(_) => {
                    println!("write back.")
                }
                Err(e) => {
                    return (false, Some(CheckError::ReadWriteError(e)));
                }
            }
        }
    }

    return (true, None);
}


pub(crate) fn check_unit(stream: &mut TcpStream,  start_time: std::time::Instant) -> WrapperMessage {
    let mut buf:[u8;1024] = [0;1024];
    let default_addr = SocketAddr::from_str("127.0.0.0:8001").unwrap();
    let mut peek_buf = [0u8; 1];
    match  recv(stream.as_raw_fd(), &mut peek_buf, MsgFlags::MSG_PEEK | MsgFlags::MSG_DONTWAIT ) {
        Ok(size) => {
            let duration_time = std::time::Instant::now().duration_since(start_time);
            if size == 0 {
                // println!("[{:?}] closed[FIN]", stream.peer_addr());
                return WrapperMessage{
                    addr: stream.peer_addr(),
                    content: "".to_string(),
                    thread_index: None,
                    probe_time: Some(duration_time),
                    check_error: CheckError::FIN
                };

            } else {
                // normal
                match stream.read(&mut buf) {

                    Ok(size) => {
                        if size != 0 {
                            println!("recv from: {}: {}",stream.peer_addr().unwrap_or(default_addr).to_string(),
                                     from_utf8(&buf[..size]).unwrap_or_default());
                            return WrapperMessage{
                                addr: stream.peer_addr(),
                                content: "".to_string(),
                                thread_index: None,
                                probe_time: Some(duration_time),
                                check_error: CheckError::Readed };
                        } else {
                            return WrapperMessage{
                                addr: stream.peer_addr(),
                                content: "-------reached EOF, maybe[FIN]".to_string(),
                                thread_index: None,
                                probe_time: Some(duration_time),
                                check_error: CheckError::FIN };
                        }
                    }
                    Err(e) => {
                        return WrapperMessage{
                            addr: stream.peer_addr(),
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
                        addr: stream.peer_addr(),
                        content: "".to_string(),
                        thread_index: None,
                        probe_time: None,
                        check_error: CheckError::EAGAIN };
                }
                errno::Errno::ECONNRESET => {
                    let duration_time = std::time::Instant::now().duration_since(start_time);
                    return WrapperMessage{
                        addr: stream.peer_addr(),
                        content: "-------reached EOF, maybe[FIN]".to_string(),
                        thread_index: None,
                        probe_time: Some(duration_time),
                        check_error: CheckError::RESET };
                }
                others => {
                    return WrapperMessage{
                        addr: stream.peer_addr(),
                        content: "-------reached EOF, maybe[FIN]".to_string(),
                        thread_index: None,
                        probe_time: Some(duration_time),
                        check_error: CheckError::OtherErrno(others) };

                }
            }
        }
    }

}
//
// pub fn check_loop(mut stream: TcpStream, check_interval: Duration, start_time: std::time::Instant, tx: mpsc::Sender<Message>) {
//     let mut buf:[u8;1024] = [0;1024];
//     let default_addr = SocketAddr::from_str("127.0.0.0:8001").unwrap();
//     loop {
//         thread::sleep(check_interval);
//         let mut peek_buf = [0u8; 1];
//         match  recv(stream.as_raw_fd(), &mut peek_buf, MsgFlags::MSG_PEEK | MsgFlags::MSG_DONTWAIT ) {
//             Ok(size) => {
//                 if size == 0 {
//                     // println!("[{:?}] closed[FIN]", stream.peer_addr());
//                     let duration_time = std::time::Instant::now().duration_since(start_time);
//                     tx.send(  Message::FIN( WrapperMessage{addr: stream.peer_addr(),
//                         content:format!("connection duration time: {:?}-------", duration_time) } ))
//                         .unwrap_or_default();
//                     return;
//                 } else {
//                     // normal
//                     match stream.read(&mut buf) {
//                         Ok(size) => {
//                             if size != 0 {
//                                 println!("recv from: {}: {}",stream.peer_addr().unwrap_or(default_addr).to_string(),
//                                          from_utf8(&buf[..size]).unwrap_or_default());
//                             } else {
//                                 let duration_time = std::time::Instant::now().duration_since(start_time);
//                                 tx.send(  Message::FIN( WrapperMessage{addr: stream.peer_addr(),
//                                     content:format!("connection duration time: {:?}<-------reached EOF, maybe[FIN]", duration_time) } ))
//                                     .unwrap_or_default();
//                                 return;
//
//                             }
//                         }
//                         Err(e) => {
//                             tx.send(  Message::CheckError( WrapperMessage{addr: stream.peer_addr(), content:format!("check read error: {:?}", e.kind()) } ))
//                                 .unwrap_or_default();
//                             return;
//                         }
//
//                     }
//                 }
//             }
//             Err(e) => {
//                 match e {
//                     errno::Errno::EAGAIN | errno::Errno::EWOULDBLOCK => {
//                         // print!("operation would block, Try again, [EAGAIN]\r")
//                     }
//                     errno::Errno::ECONNRESET => {
//                         let duration_time = std::time::Instant::now().duration_since(start_time);
//                         tx.send(  Message::RESET( WrapperMessage{addr: stream.peer_addr(),
//                             content:format!("connection duration time: {:?}", duration_time) } ))
//                             .unwrap_or_default();
//                         return;
//                     }
//                     others => {
//                         tx.send(  Message::CheckError( WrapperMessage{addr: stream.peer_addr(), content:format!("other error[check_loop]: {:?}", e) } ))
//                             .unwrap_or_default();
//                         return;
//
//                     }
//                 }
//             }
//         }
//     }
// }
//
//

