//
// use std::str::{from_utf8, FromStr};
// use std::{thread, mem, os};
// use std::time::{Duration, Instant};
//
// use nix::sys::socket::MsgFlags;
// use nix::sys::socket::{recv, send, setsockopt as nix_setsockopt};

// use std::os::unix::io::{AsRawFd, RawFd};
// use nix::libc;
// use libc::setsockopt;
//
// use std::sync::mpsc;
#[cfg(target_family = "linux")]
use nix::sys::socket::sockopt::{KeepAlive, TcpUserTimeout}; // 参考nix sockopt 跨平台用法，再不行就用macro
                                                            // use crate::Message::Probe;
                                                            // use std::ops::Sub;
                                                            // use std::thread::JoinHandle;

// use tcp_lang_conn::async_server::start_async_server;
use tcp_lang_conn::client::start_client as start_client_lib;
// use tcp_lang_conn::check_unit::check_loop;
// use tcp_lang_conn::check_status::{Message, WrapperMessage};
// struct  Color {
//     HEADER : '\033[95m',
//     OKBLUE = '\033[94m',
//     OKCYAN = '\033[96m'
//     OKGREEN = '\033[92m'
//     WARNING = '\033[93m'
//     FAIL = '\033[91m'
//     ENDC = '\033[0m'
//     BOLD = '\033[1m'
//     UNDERLINE = '\033[4m'
// }

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // todo 引用计数 显示所有thread数
    match args.len() {
        1..=2 => {
            // todo
            println!(
                "Usage: ./tcp-lang-conn-test [s|c] [ip_address:port]\n{}\n{}\n{}\n{}",
                "Example: ",
                "1. Server(192.168.1.1) start:    ./connTest s 0.0.0.0:8001",
                "2. Client(192.168.1.2) start:    ./connTest c 192.168.1.1:8001",
                "version 2023-12-12"
            );
        }
        3 => {
            match args[1].as_str() {
                "s" | "S" => {
                    // todo check "0.0.0.0:8001"
                    // #[tokio::main]
                    // tcp_lang_conn::server::start_server(args[2].as_str());

                    tokio::runtime::Builder::new_multi_thread()
                        .enable_all()
                        .build()
                        .unwrap()
                        .block_on(async {
                            let err = tcp_lang_conn::async_server::start_async_server(args[2].as_str()).await;
                            match err {
                                Err(e) => {println!("Main Err: {}", e.kind())}
                                _ => {}
                            }
                        })
                }
                "c" | "C" => {
                    // todo check "192.3.2.1:8001"
                    start_client_lib(args[2].as_str());
                    // start_client();
                }
                mode => {
                    // todo
                    println!("error param {}", mode);
                }
            }
        }
        _ => {
            println!("error param");
        }
    }
}
//
// fn main1() {
//
//     let args: Vec<String> = std::env::args().collect();
//     let (tx, rx) = mpsc::channel::<Message>();
//     // todo 引用计数 显示所有thread数
//     thread::spawn(move || {
//         // 统一消息的处理中心！
//         let default_addr = SocketAddr::from_str("127.0.0.0:8001").unwrap();
//         loop {
//             match rx.recv() {
//                 Ok(m) => {
//                     m.handling_result(true);
//                 }
//                 _ => {}
//             }
//         }
//     });
//     match args.len() {
//         1..=2 => {
//             // todo
//             println!("help message");
//         }
//         3 => {
//             match args[1].as_str() {
//                 "s"|"S" => {
//                     // todo check "0.0.0.0:8001"
//                     start_server(args[2].as_str(), tx);
//
//                 }
//                 "c" | "C" => {
//                     // todo check "192.3.2.1:8001"
//                     start_client(args[2].as_str(), tx);
//                     // start_client();
//                 }
//                 mode => {
//                     // todo
//                     println!("error param {}", mode);
//                 }
//             }
//         }
//         default => {
//             println!("error param");
//         }
//     }
//
// }

//
// fn start_server(addr_port: &str, tx: mpsc::Sender<Message>) {
//     // addr
//     let listener = TcpListener::bind(addr_port).expect("bind failed!");
//     println!("Listener started");
//     let check_intervel = Duration::from_millis(200);
//     for stream in listener.incoming() {
//         let mut stream = stream.unwrap();
//
//         let tx_t = tx.clone();
//         thread::spawn(move || {
//
//             if stream_rw_unit(&mut stream, true, &tx_t, &0) { return; };
//             // thread::sleep(Duration::from_secs(5));
//             let start_time = std::time::Instant::now();
//             check_loop(stream, check_intervel, start_time, tx_t);
//         });
//
//     }
//
// }
//
//
//
// fn start_client(addr_port: &str, tx: mpsc::Sender<Message>) {
//     // let addr_port: &str = "192.168.1.2:80018";
//     // 起多个线程，做线程序列，[ 5min 10min 15m 30m 1h 2h 4h 8h 12h 18h 24h 28h 36h]
//     // 如果正常断开，比如15分钟，那么在某一时间段，30m 1h 2h 4h 8h 12h 18h 24h 28h 36h] 这些连接都能收到reset包正常断开。
//     // 如果不能正常，就按照时间序列探测。
//     let threads_lists = [15, 5u64*60, 10*60, 15*60, 30*60, 1*3600, 2*3600, 3*3600, 4*3600, 5*3600, 6*3600, 7*3600, 8* 3600,
//                                 12 * 3600, 18* 3600, 24 * 3600, 28 * 3600, 36 * 3600, 7200*3600].map(|t| Duration::from_secs(t));
//     let check_interval = Duration::from_millis(100);
//     let mut a: Vec<JoinHandle<()>> = vec![];
//     for thread_index in 0..threads_lists.len() {
//         // todo 引用计数。。。。tx rt 等等
//         let tx_t = tx.clone();
//         let probe_time = threads_lists[thread_index];
//         let addr_port_move = addr_port.to_string(); // ???? todo !!!!!!!!!!!
//         if !(thread_index == threads_lists.len() - 1) {
//             let t1 = thread::spawn(move || {start_client_sub_thread(thread_index+1,
//                                                            &addr_port_move,
//                                                           probe_time,
//                                                           false,
//                                                           None, // remove optional test,
//                                                             tx_t
//             );} );
//
//             a.push(t1);
//         } else {
//             start_client_sub_thread(thread_index+1,
//                                     &addr_port_move,
//                                     threads_lists[thread_index],
//                                     true,
//                                     Some(check_interval),
//                                                 tx_t);
//         }
//     }
// }
//
//
// pub fn start_client_sub_thread(thread_index: usize, addr_port: &str,
//                                probe_time: Duration, is_control_thread: bool,
//                                check_interval: Option<Duration>, tx: mpsc::Sender<Message>) {
//     let mut stream = TcpStream::connect(addr_port).expect("connection failed!");
//     // for stream in listener.incoming() {
//     //     let mut stream = stream.unwrap();
//     // stream.set_write_timeout(Some(Duration::new(5, 0)));
//
//     // let client_hello = format!("[{}]Client hello", thread_index);
//     if stream_rw_unit(&mut stream, false, &tx, &thread_index) { return; };
//
//
//     // 设置 定shide probe thread
//     if !is_control_thread {
//         thread::sleep(probe_time);
//         let write_content = "heartbeat".as_bytes();
//         println!("------[{}]will probe------", thread_index);
//         stream.set_write_timeout(Some(Duration::new(1, 0))); // 无效参数，仅仅针对本地写到缓存，而不是完整的链路
//         // 根据平台区分
//
//         let tcp_user_timeout = 1u32; // 重传超时时间，不是次数，大概是发四次包的样子，0x18 tcp_user_timeout macos tcpxxx: 0x80
//         // 都可用，但是单位不一样，linux millens ref: https://man7.org/linux/man-pages/man7/tcp.7.html：
//         //  it specifies the maximum
//         //  amount of time in milliseconds that transmitted data may
//         //  remain unacknowledged, or bufferred data may remain
//         //  untransmitted (due to zero window size) before TCP will
//         //  forcibly close the corresponding connection and return
//         //  ETIMEDOUT to the application.  If the option value is
//         //  specified as 0, TCP will use the system default.
//         //
//         // macos为second
//         if cfg!(target_os = "linux") {
//             // unsafe {
//             //     let a = setsockopt(stream.as_raw_fd(), 0x06, 0x12,
//             //                        &tcp_user_timeout as *const u32 as *const c_void,  mem::size_of::<c_int>() as u32);
//             //     println!("lib set sockopt error: {}", a);
//             // }
//             // 两种方法都可以；
//
//
//             // //  这玩意只不过写了个 宏调用 libc，做了一些封装
//             #[cfg(target_family = "linux")]
//             match nix_setsockopt(stream.as_raw_fd(), TcpUserTimeout, &(tcp_user_timeout as u32)) {
//                 Ok(_) => {},
//                 Err(e) => {println!("nix_lib set sockopt error: {:?}", e)}, // ???? 遗留
//             }
//         } else if cfg!(target_os = "macos") {
//             unsafe {
//                 let a = setsockopt(stream.as_raw_fd(), 0x06, 0x80,
//                                    &tcp_user_timeout as *const u32 as *const c_void,  mem::size_of::<c_int>() as u32);
//                 println!("lib set sockopt error: {}", a);
//             }
//         } else {
//             println!("Unsupported platform!");
//             std::process::exit(1);
//         }
//
//
//
//         match send(stream.as_raw_fd(), format!("[{}]one hello", thread_index).as_bytes(), MsgFlags::empty()) {
//             Ok(size) => {
//                 if size == 0 {
//                     println!("send --> closed[FIN]")
//                 } else {
//                     println!("send --> peek: {}", size);
//                 }
//             }
//             Err(e) => {
//                 match e {
//                     errno::Errno::EAGAIN | errno::Errno::EWOULDBLOCK => {
//                         println!("operation would block, Try again, [EAGAIN]");
//                     }
//                     errno::Errno::ECONNRESET => {
//                         // tx.send(Message::FIN(WrapperMessage{addr: stream.peer_addr(), content: "reached EOF,".to_string() } ))
//                         //     .unwrap_or_default();
//                         println!("read closed. reached EOF, maybe[FIN]");
//                         // return;
//                     }
//                     others => { println!("[{}]: {:?} probe send other error: {}!", thread_index, probe_time, others); }
//                 }
//             }
//         }
//
//         println!("[{}]-----check after probe----", thread_index);
//
//         thread::sleep(Duration::from_secs(3));
//         // 如果检测时，tcp孩子重试，则此处的错误为：EAGAIN！！！所以一定要确保检测时，已经重试完毕。
//         // 从抓包情况来看，重试的完如果不同系统就直接发reset包，而程序结束时发[FIN]包，
//         // 至于先发reset还是[FIN]，如果正常通信的情况下，互相发完fin，就完了，不会发reset包。
//         // 非正常情况，程序的fin和系统的reset各发各的，互不影响。但是先发reset就不发fin了，反过来不成立
//         // 似乎linux不发送reset--
//         {
//             let mut peek_buf = [0u8; 1];
//             match recv(stream.as_raw_fd(), &mut peek_buf, MsgFlags::MSG_PEEK | MsgFlags::MSG_DONTWAIT) {
//                 Ok(size) => {
//                     // never run here
//                     if size == 0 {
//                         println!("check closed[FIN]")
//                     } else {
//                         println!("check peek:{}", size); // never run
//                     }
//                 }
//                 Err(e) => {
//                     tx.send(Message::Probe(stream.peer_addr(), e, thread_index, probe_time)).unwrap();
//                 }
//             }
//         }
//         return;
//     }
//     else {
//         check_loop(stream, check_interval.unwrap(), Instant::now(), tx);
//     }
//
// }
