
// extern crate core;

use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::io::{Read, Write};
use std::str::from_utf8;
use std::{thread, mem, os};
use std::time::{Duration, Instant};

use std::mem::MaybeUninit;
use nix::sys::socket::MsgFlags;
use nix::sys::socket::{recv, send, setsockopt as nix_setsockopt};

use std::os::unix::io::{AsRawFd, RawFd};
use nix::{errno, libc};
use nix::errno::Errno::ETIMEDOUT;
use libc::setsockopt;
use std::os::raw::{c_void, c_int};
use std::sync::mpsc;
use nix::libc::clone;

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
    match args.len() {
        1..=2 => {
            // todo
            println!("help message");
        }
        3 => {
            match args[1].as_str() {
                "s"|"S" => {
                    // todo check "0.0.0.0:8001"
                    start_server(args[2].as_str());

                }
                "c" | "C" => {
                    // todo check "192.3.2.1:8001"
                    start_client(args[2].as_str());
                    // start_client();
                }
                mode => {
                    // todo
                    println!("error param {}", mode);
                }
            }
        }
        default => {
            println!("error param");
        }
    }

}

fn start_server(addr_port: &str) {
    // addr
    let listener = TcpListener::bind(addr_port).expect("bind failed!");

    let (tx, rx) = mpsc::channel::<u32>();
    let check_intervel = Duration::from_millis(200);
    let tx_clone = tx.clone();
    // todo 引用计数 显示所有thread数
    for stream in listener.incoming() {
        let mut stream = stream.unwrap();
        let send_content = "Server hello";
        thread::spawn(move || {server_handle(stream, send_content, check_intervel);});

    }
}


fn server_handle(mut stream: TcpStream, send_str: &str, check_intervel: Duration/* milliseconds */) {
    let mut buf = [0u8; 1024];
    match stream.read(&mut buf) {
        Ok(size) => {
            if size != 0 {
                println!("recv: {}", from_utf8(&buf[..size]).unwrap());
            } else {
                println!("read closed. reached EOF, maybe[FIN]");
            }
        }
        Err(e) => {
            println!("first read error:{}", e.kind());
            panic!("read error: {}", e.kind().to_string())
        }

    }
    // write back to clint: Server hello;
    match stream.write(send_str.as_ref()) {
        Ok(_) => {
            println!("write back.")
        }
        Err(e) => {
            println!("{}", e.kind());
            panic!("send error: {}", e.kind().to_string())
        }
    }
    // thread::sleep(Duration::from_secs(5));
    let start_time = std::time::Instant::now();
    check_loop(stream, check_intervel, start_time);
}
fn start_client(addr_port: &str) {
    // let addr_port: &str = "192.168.1.2:80018";
    // 起多个线程，做线程序列，[ 5min 10min 15m 30m 1h 2h 4h 8h 12h 18h 24h 28h 36h]
    // 如果正常断开，比如15分钟，那么在某一时间段，30m 1h 2h 4h 8h 12h 18h 24h 28h 36h] 这些连接都能收到reset包正常断开。
    // 如果不能正常，就按照时间序列探测。
    let threads_lists = [30, 5u64*60, 10*60, 15*60, 30*60, 1*3600, 2*3600, 3*3600, 4*3600, 5*3600, 6*3600, 7*3600, 8* 3600,
                                12 * 3600, 18* 3600, 24 * 3600, 28 * 3600, 36 * 3600, 7200*3600].map(|t| Duration::from_secs(t));
    let check_interval = Duration::from_millis(100);
    for thread_index in 0..threads_lists.len() {
        // todo 引用计数。。。。tx rt 等等
        let probe_time = threads_lists[thread_index];
        let addr_port_move = addr_port.to_string(); // ???? todo !!!!!!!!!!!
        if !(thread_index == threads_lists.len() - 1) {
            thread::spawn(move || {start_client_sub_thread(thread_index+1,
                                                           &addr_port_move,
                                                          probe_time,
                                                          false,
                                                          None // remove optional test
            );} );
        } else {
            start_client_sub_thread(thread_index+1,
                                    &addr_port_move,
                                    threads_lists[thread_index],
                                    true,
                                    Some(check_interval) );
        }
    }
}


fn start_client_sub_thread(thread_index: usize, addr_port: &str, probe_time: Duration, is_control_thread: bool, check_interval: Option<Duration>) {
    let mut stream = TcpStream::connect(addr_port).expect("connection failed!");
    // for stream in listener.incoming() {
    //     let mut stream = stream.unwrap();
    // stream.set_write_timeout(Some(Duration::new(5, 0)));

    let client_hello = format!("[{}]Client hello", thread_index);

    match stream.write(client_hello.as_bytes()) {
        Ok(_) => {
            println!("[{}]client send.", thread_index);
        }
        Err(e) => {
            if !is_control_thread {
                println!("[{}]first send error: {:?}",thread_index, e.kind());
                return;
            } else {
                println!("control_thread:[{}]first send error: {:?}",thread_index, e.kind());
                std::process::exit(1);
            }

        }
    }
    let mut buf = [0u8; 1024];
    // 过了很久以后这一次发包，1。 如果正常回包，说明链接活着，2。 如果超时，说明挂了，3。 如果收到了reset，说明就是HCS的情况
    match stream.read(&mut buf) {
        Ok(size) => {
            if !is_control_thread {
                if size != 0 {
                    println!("[{}]read: {}", thread_index, from_utf8(&buf[..size]).unwrap());
                } else {
                    println!("[{}]read closed[FIN]", thread_index);
                    return;
                }
            } else {
                if size != 0 {
                    println!("control_thread[{}]read: {}", thread_index, from_utf8(&buf[..size]).unwrap());
                } else {
                    println!("control_thread[{}]read closed[FIN]", thread_index);
                    std::process::exit(0);
                }
            }
        }
        Err(e) => {
            println!("[{}]client read error: {:?}",thread_index, e.kind());
        }

    }

    // 设置 定shide probe thread
    if !is_control_thread {
        thread::sleep(probe_time);

        let write_content = "heartbeat".as_bytes();
        println!("------[{}]will probe------", thread_index);
        stream.set_write_timeout(Some(Duration::new(1, 0))); // 无效参数，仅仅针对本地写到缓存，而不是完整的链路
        // 根据平台区分

        let tcp_user_timeout = 1u32; // 重传超时时间，不是次数，大概是发四次包的样子，0x18 tcp_user_timeout macos tcpxxx: 0x80
        // 都可用，但是单位不一样，linux millens ref: https://man7.org/linux/man-pages/man7/tcp.7.html：
        //  it specifies the maximum
        //  amount of time in milliseconds that transmitted data may
        //  remain unacknowledged, or bufferred data may remain
        //  untransmitted (due to zero window size) before TCP will
        //  forcibly close the corresponding connection and return
        //  ETIMEDOUT to the application.  If the option value is
        //  specified as 0, TCP will use the system default.
        //
        // macos为second
        if cfg!(target_os = "linux") {
            // unsafe {
            //     let a = setsockopt(stream.as_raw_fd(), 0x06, 0x12,
            //                        &tcp_user_timeout as *const u32 as *const c_void,  mem::size_of::<c_int>() as u32);
            //     println!("lib set sockopt error: {}", a);
            // }
            // 两种方法都可以；
            use nix::sys::socket::sockopt::TcpUserTimeout;
            // //  这玩意只不过写了个 宏调用 libc，做了一些封装
            match nix_setsockopt(stream.as_raw_fd(), TcpUserTimeout, &(tcp_user_timeout as u32)) {
                Ok(_) => {},
                Err(e) => {println!("nix_lib set sockopt error: {:?}", e)},
            }
        } else if cfg!(target_os = "macos") {
            unsafe {
                let a = setsockopt(stream.as_raw_fd(), 0x06, 0x80,
                                   &tcp_user_timeout as *const u32 as *const c_void,  mem::size_of::<c_int>() as u32);
                println!("lib set sockopt error: {}", a);
            }
        } else {
            println!("Unsupported platform!");
            std::process::exit(1);
        }



        match send(stream.as_raw_fd(), format!("[{}]one hello", thread_index).as_bytes(), MsgFlags::empty()) {
            Ok(size) => {
                if size == 0 {
                    println!("send --> closed[FIN]")
                } else {
                    println!("send --> peek: {}", size);
                }
            }
            Err(e) => {
                match e {
                    errno::Errno::EAGAIN | errno::Errno::EWOULDBLOCK => {
                        println!("operation would block, Try again, [EAGAIN]")
                    }
                    errno::Errno::ECONNRESET => {
                        println!("connection closed [R]")
                    }
                    others => {println!("other error: {}", others)}
                }
            }
        }

        // match stream.write(b"second write") {
        //         Ok(size) => {
        //             println!("write back. {}", size);
        //         }
        //         Err(e) => {
        //             println!("{:?}", e.kind());
        //         }
        //     }


        println!("-----check----");

        thread::sleep(Duration::from_secs(3));
        // 如果检测时，tcp孩子重试，则此处的错误为：EAGAIN！！！所以一定要确保检测时，已经重试完毕。
        // 从抓包情况来看，重试的完如果不同系统就直接发reset包，而程序结束时发[FIN]包，
        // 至于先发reset还是[FIN]，如果正常通信的情况下，互相发完fin，就完了，不会发reset包。
        // 非正常情况，程序的fin和系统的reset各发各的，互不影响。但是先发reset就不发fin了，反过来不成立
        // 似乎linux不发送reset--
        {
            let mut peek_buf = [0u8; 1];
            match recv(stream.as_raw_fd(), &mut peek_buf, MsgFlags::MSG_PEEK | MsgFlags::MSG_DONTWAIT) {
                Ok(size) => {
                    if size == 0 {
                        println!("check closed[FIN]")
                    } else {
                        println!("check peek:{}", size);
                    }
                }
                Err(e) => {
                    match e {
                        errno::Errno::EAGAIN | errno::Errno::EWOULDBLOCK => {
                            // println!("operation would block, Try again, [EAGAIN]");
                            println!("[{}] {:?} \x1b[40;32mhas alive\x1b[0m [EAGAIN]", thread_index, probe_time);
                        }
                        errno::Errno::ECONNRESET => {
                            // color wrong! todo -
                            println!("[{}]: {:?} connection \x1f[40;32mclosed\x1f[0m [R] some connection was killed!", thread_index, probe_time);
                            // println!("connection closed [R]")
                        }
                        errno::Errno::ETIMEDOUT => {
                            println!("[{}]: {:?} \x1f[40;32mNO response!!!\x1f[0m some connection was killed!", thread_index, probe_time);
                            // todo recycle probe but now exit;
                            std::process::exit(1);

                        }
                        others => { println!("[{}]: {:?} other error: {}!", thread_index, probe_time, others); }
                    }
                }
            }
        }
        return;
    }
    else {
        check_loop(stream, check_interval.unwrap(), Instant::now());
    }

}

fn check_loop(mut stream: TcpStream, check_interval: Duration, start_time: std::time::Instant) {
    let mut buf:[u8;1024] = [0;1024];
    loop {
        thread::sleep(check_interval);
        let mut peek_buf = [0u8; 1];
        match  recv(stream.as_raw_fd(), &mut peek_buf, MsgFlags::MSG_PEEK | MsgFlags::MSG_DONTWAIT ) {
            Ok(size) => {
                if size == 0 {
                    println!("[{:?}] closed[FIN]", stream.peer_addr());
                    let duration_time = std::time::Instant::now().duration_since(start_time);
                    println!("[Result] connection alive time: {:?}-------", duration_time);
                    std::process::exit(0);
                } else {
                    // normal
                    // println!("peek:{}", size);
                    match stream.read(&mut buf) {
                        Ok(size) => {
                            if size != 0 {
                                println!("recv: {}", from_utf8(&buf[..size]).unwrap());
                            } else {
                                println!("read closed. reached EOF, maybe[FIN]");
                                let duration_time = std::time::Instant::now().duration_since(start_time);
                                println!("[Result] connection alive time: {:?}-------", duration_time);
                                std::process::exit(0);
                            }
                        }
                        Err(e) => {
                            println!("first read error:{}", e.kind());
                            panic!("read error: {}", e.kind().to_string())
                        }

                    }
                }
            }
            Err(e) => {
                match e {
                    errno::Errno::EAGAIN | errno::Errno::EWOULDBLOCK => {
                        // print!("operation would block, Try again, [EAGAIN]\r")
                    }
                    errno::Errno::ECONNRESET => {
                        println!("[{:?}] connection closed [R]", stream.peer_addr().unwrap_or(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080)));
                        let duration_time = std::time::Instant::now().duration_since(start_time);
                        println!("[Result] connection alive time: {:?}-------", duration_time);
                        std::process::exit(0);
                    }
                    others => {println!("other error[check_loop]: {}", others); std::process::exit(1);}

                }
            }
        }
    }
}

fn main_1()  {
    let listener = TcpListener::bind("0.0.0.0:8008").expect("bind failed!");
    for stream in listener.incoming() {
        let mut stream = stream.unwrap();
        let mut buf = [0u8; 1024];
        match stream.read(&mut buf) {
            Ok(size) => {
                if size != 0 {
                    println!("{}", from_utf8(&buf[..size]).unwrap());
                } else {
                    println!("read closed[FIN]");
                }
            }
            Err(e) => {
                println!("{}", e.kind());
            }

        }

        match stream.write(&buf) {
            Ok(_) => {
                println!("write back.")
            }
            Err(e) => {
                println!("{}", e.kind());
            }
        }
        thread::sleep(Duration::from_secs(5));

        {

            let mut peek_buf = [0u8; 1];
            match  recv(stream.as_raw_fd(), &mut peek_buf, MsgFlags::MSG_PEEK | MsgFlags::MSG_DONTWAIT ) {
                Ok(size) => {
                    if size == 0 {
                        println!("closed[FIN]")
                    } else {
                        // normal
                        println!("peek:{}", size);

                    }
                }
                Err(e) => {
                    match e {
                        errno::Errno::EAGAIN | errno::Errno::EWOULDBLOCK => {
                            println!("operation would block, Try again, [EAGAIN]")
                        }
                        errno::Errno::ECONNRESET => {
                            println!("connection closed [R]")
                        }
                        others => {println!("other error: {}", others)}
                    }
                }
            }
        }
    }
}




fn main111()  {


    // 起多个线程，做线程序列，[ 5min 10min 15m 30m 1h 2h 4h 8h 12h 18h 24h 28h 36h]
    // 如果正常断开，比如15分钟，那么在某一时间段，30m 1h 2h 4h 8h 12h 18h 24h 28h 36h] 这些连接都能收到reset包正常断开。
    // 如果不能正常，就按照时间序列探测。
    let mut stream = TcpStream::connect("192.168.1.2:8008").expect("bind failed!");
    // for stream in listener.incoming() {
    //     let mut stream = stream.unwrap();
    let mut buf = [0u8; 1024];

    // stream.set_write_timeout(Some(Duration::new(5, 0)));

    match stream.write(b"client hello") {
        Ok(size) => {
            println!("write back. {}", size);
        }
        Err(e) => {
            println!("{:?}", e.kind());
        }
    }
    // 过了很久以后这一次发包，1。 如果正常回包，说明链接活着，2。 如果超时，说明挂了，3。 如果收到了reset，说明就是HCS的情况
    match stream.read(&mut buf) {
        Ok(size) => {
            if size != 0 {
                println!("read: {}", from_utf8(&buf[..size]).unwrap());
            } else {
                println!("read closed[FIN]");
            }
        }
        Err(e) => {
            println!("{:?}", e.kind());
        }

    }

    thread::sleep(Duration::from_secs(10));

    let write_content = "heartbeat".as_bytes();
    println!("------will write------");
    stream.set_write_timeout(Some(Duration::new(10, 0))); // 无效参数，仅仅针对本地写到缓存，而不是完整的链路
    // 根据平台区分

    let tcp_user_timeout = 1u32; // 重传超时时间，不是次数，大概是发四次包的样子，0x18 tcp_user_timeout macos tcpxxx: 0x80
    // 都可用，但是单位不一样，linux millens ref: https://man7.org/linux/man-pages/man7/tcp.7.html：
    //  it specifies the maximum
    //  amount of time in milliseconds that transmitted data may
    //  remain unacknowledged, or bufferred data may remain
    //  untransmitted (due to zero window size) before TCP will
    //  forcibly close the corresponding connection and return
    //  ETIMEDOUT to the application.  If the option value is
    //  specified as 0, TCP will use the system default.
    //
    // macos为second
    if cfg!(target_os = "linux") {
        // unsafe {
        //     let a = setsockopt(stream.as_raw_fd(), 0x06, 0x12,
        //                        &tcp_user_timeout as *const u32 as *const c_void,  mem::size_of::<c_int>() as u32);
        //     println!("lib set sockopt error: {}", a);
        // }
        // 两种方法都可以；
        use nix::sys::socket::sockopt::TcpUserTimeout;
        // //  这玩意只不过写了个 宏调用 libc，做了一些封装
        match nix_setsockopt(stream.as_raw_fd(), TcpUserTimeout, &(tcp_user_timeout as u32)) {
            Ok(_) => {},
            Err(e) => {println!("nix_lib set sockopt error: {:?}", e)},
        }
    } else if cfg!(target_os = "macos") {
        unsafe {

            let a = setsockopt(stream.as_raw_fd(), 0x06, 0x80,
                               &tcp_user_timeout as *const u32 as *const c_void,  mem::size_of::<c_int>() as u32);
            println!("lib set sockopt error: {}", a);
        }
    } else {
        println!("Unsupported platform!");
        std::process::exit(1);
    }



    match send(stream.as_raw_fd(), b"probe heartbeat", MsgFlags::empty()) {
        Ok(size) => {
            if size == 0 {
                println!("send --> closed[FIN]")
            } else {
                println!("send --> peek: {}", size);
            }
        }
        Err(e) => {
            match e {
                errno::Errno::EAGAIN | errno::Errno::EWOULDBLOCK => {
                    println!("operation would block, Try again, [EAGAIN]")
                }
                errno::Errno::ECONNRESET => {
                    println!("connection closed [R]")
                }
                others => {println!("other error: {}", others)}
            }
        }
    }

    // match stream.write(b"second write") {
    //         Ok(size) => {
    //             println!("write back. {}", size);
    //         }
    //         Err(e) => {
    //             println!("{:?}", e.kind());
    //         }
    //     }


    println!("-----check----");

    thread::sleep(Duration::from_secs(3));
    // 如果检测时，tcp孩子重试，则此处的错误为：EAGAIN！！！所以一定要确保检测时，已经重试完毕。
    // 从抓包情况来看，重试的完如果不同系统就直接发reset包，而程序结束时发[FIN]包，
    // 至于先发reset还是[FIN]，如果正常通信的情况下，互相发完fin，就完了，不会发reset包。
    // 非正常情况，程序的fin和系统的reset各发各的，互不影响。但是先发reset就不发fin了，反过来不成立
    // 似乎linux不发送reset--
    {
        let mut peek_buf = [0u8; 1];
        match recv(stream.as_raw_fd(), &mut peek_buf, MsgFlags::MSG_PEEK | MsgFlags::MSG_DONTWAIT) {
            Ok(size) => {
                if size == 0 {
                    println!("check closed[FIN]")
                } else {
                    println!("check peek:{}", size);
                }
            }
            Err(e) => {
                match e {
                    errno::Errno::EAGAIN | errno::Errno::EWOULDBLOCK => {
                        println!("operation would block, Try again, [EAGAIN]")
                    }
                    errno::Errno::ECONNRESET => {
                        println!("connection closed [R]")
                    }
                    errno::Errno::ETIMEDOUT => {
                        println!("NO response!!! some connection was killed!")
                    }
                    others => { println!("other error: {}", others) }
                }
            }
        }
    }

    // thread::sleep(Duration::from_secs(2))
    // }
    //defer: over client send [fin]
}
