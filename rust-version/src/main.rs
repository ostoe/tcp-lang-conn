
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::str::from_utf8;
use std::{thread, mem, os};
use std::time::Duration;

use std::mem::MaybeUninit;
use nix::sys::socket::MsgFlags;
use nix::sys::socket::{recv, send, setsockopt as nix_setsockopt};

use std::os::unix::io::{AsRawFd, RawFd};
use nix::{errno, libc};
use nix::errno::Errno::ETIMEDOUT;
use libc::setsockopt;
use std::os::raw::{c_void, c_int};

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

// sockopt_impl!(
//     // #[cfg_attr(docsrs, doc(cfg(feature = "net")))]
//     /// Specifies the maximum amount of time in milliseconds that transmitted
//     /// data may remain unacknowledged before TCP will forcibly close the
//     /// corresponding connection
//     TcpUserTimeout, Both, libc::IPPROTO_TCP, libc::TCP_USER_TIMEOUT, u32);

fn main()  {


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



    match send(stream.as_raw_fd(), b"one hello", MsgFlags::empty()) {
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
