use crate::check_status::CheckError;
use crate::check_unit::{check_unit, stream_rw_unit};
use crossbeam_channel::{bounded, select, unbounded, Receiver, Sender};
use libc::setsockopt;
use nix::errno;
#[cfg(target_os = "linux")]
use nix::sys::socket::setsockopt as nix_setsockopt;
#[cfg(target_os = "linux")]
use nix::sys::socket::sockopt::{KeepAlive, TcpUserTimeout};
use nix::sys::socket::{recv, send, MsgFlags};
// use std::io::Read;
use std::net::TcpStream;
use std::os::raw::{c_int, c_void};
use std::os::unix::io::AsRawFd;
use std::str::FromStr;
use std::time::{Duration, Instant};
use std::{mem, thread};

use std::thread::JoinHandle; // 参考nix sockopt 跨平台用法，再不行就用macro

pub enum Signal {
    Run(u32),
    Terminated,
}

pub fn start_client(addr_port: &str) {
    // let addr_port: &str = "192.168.1.2:80018";
    // 起多个线程，做线程序列，[ 5min 10min 15m 30m 1h 2h 4h 8h 12h 18h 24h 28h 36h]
    // 如果正常断开，比如15分钟，那么在某一时间段，30m 1h 2h 4h 8h 12h 18h 24h 28h 36h] 这些连接都能收到reset包正常断开。
    // 如果不能正常，就按照时间序列探测。
    let mut threads_lists = [0u32; 4096];
    let threads_lists_part1 = [15u32, 30, 5 * 60, 10 * 60];
    // todo
    for x in 0..threads_lists_part1.len() {
        threads_lists[x] = threads_lists_part1[x];
    }
    let summary_time = 36usize;
    for x in 0..summary_time {
        threads_lists[x + threads_lists_part1.len()] = ((x + 1) as u32) * 3600;
    }
    // println!("{:?}", threads_lists);
    // let threads_lists = [15, 5u32*60, 10*60, 15*60, 30*60, 1*3600, 2*3600, 3*3600, 4*3600, 5*3600, 6*3600, 7*3600, 8* 3600,
    //     12 * 3600, 18* 3600, 24 * 3600, 28 * 3600, 36 * 3600, 7200*3600];
    // .map(|t| Duration::from_secs(t));

    let check_interval = Duration::from_millis(100);
    let addr_port_move = addr_port.to_string();
    // 正常检测，只能检测linux链接自己的状态,和链路正常的状态
    thread::spawn(move || {

        // let stream1 = match TcpStream::connect(&addr_port_move) {
        //     Ok(t) => t,
        //     Err(e) => {
        //         println!("[{}]: {}", "Connection failed", e.kind());
        //         std::process::exit(1);
        //     }
        // };


        // let mut stream = TcpStream::connect(addr_port_move).expect("connection failed!");
        let mut stream = match TcpStream::connect(&addr_port_move) {
            Ok(t) => t,
            Err(e) => {
                println!("[{}]: {}", "Connection failed", e.kind());
                std::process::exit(1);
            }
        };
        let default_addr = std::net::SocketAddr::from_str("127.0.0.0:8001").unwrap();
        let addr = stream.peer_addr().unwrap_or(default_addr);
        // stream.set_write_timeout(Some(Duration::new(5, 0)));
        // 第一次连接时，Hello先写再读……
        let (is_ok, _) = stream_rw_unit(&mut stream, false, 0);
        if !is_ok {
            println!("first read/wirte error, Exit");
            drop(stream);
            return;
        }
        let start_time = std::time::Instant::now();
        loop {
            // stream1.set_write_timeout(None);
            thread::sleep(check_interval);
            // 一个一直空着的链接，
            let check_result = check_unit(&mut stream, start_time);

            match check_result.check_error {
                CheckError::FIN => {
                    println!(
                        "[H.FIN] {} connection duration time: {:?}-------",
                        addr,
                        check_result.probe_time.unwrap_or(Default::default())
                    );
                    drop(stream);
                    std::process::exit(0);
                }
                CheckError::RESET => {
                    println!(
                        "[H.RESET] {} connection duration time: {:?}-------",
                        "miss addr",
                        check_result.probe_time.unwrap_or(Default::default())
                    );
                    drop(stream);
                    std::process::exit(0);
                }
                CheckError::EAGAIN | CheckError::Readed => {} // checking...
                CheckError::OtherErrno(e) => {
                    println!("[H.check] others errno error: {:?}", e);
                    drop(stream);
                    std::process::exit(0);
                }
                CheckError::ReadWriteError(e) => {
                    println!("[H.check] others read error: {:?}", e.kind());
                    drop(stream);
                    std::process::exit(0);
                }
                _ => {}
            }
        }
    });

    // 发送和接受信号，信号包含探测周期信号
    let (ctrl_probe_st, ctrl_probe_rt) = unbounded::<Signal>();
    let (ctrl_sleep_thread_exit_st, ctrl_sleep_rt) = bounded::<bool>(2);
    let (result_probe_st, result_probe_rt) = unbounded::<(CheckError, u32, Duration)>();

    // probe list
    let ctrl_probe_rt_clone = ctrl_probe_rt.clone();
    let result_probe_st_clone = result_probe_st.clone();
    // 启动probe线程，建立很多链接，但是不做操作，等待控制线程发命令，收到命令时发送Ping，检测链接状态……
    // let addr_port_c = addr_port.to_string();
    probe_timing_thread(
        addr_port,
        ctrl_probe_rt_clone,
        result_probe_st_clone,
        threads_lists_part1.len() + summary_time,
    );

    // 定时，到时间以后通知
    let ctrl_sleep_rt_clone = ctrl_sleep_rt.clone();
    let ctrl_probe_st_clone = ctrl_probe_st.clone();
    // ------------------------ sleep thread，过一段时间发一个消息给探测线程，让探测线程发过去
    sleep_timing_thread(
        threads_lists,
        ctrl_sleep_rt_clone,
        ctrl_probe_st_clone,
        threads_lists_part1.len() + summary_time,
        true,
    );
    // 起一个沉睡线程////
    // let ticker = tick(Duration::from_secs(1));
    // let _ = ticker.recv();

    let mut has_probe_count = 0usize;
    let mut threads_lists_length = threads_lists_part1.len() + summary_time;
    let mut stage_one = true;
    loop {
        let probe_result = result_probe_rt.recv();

        let (e, probe_time, conn_elapsed_time) = probe_result.unwrap_or_default();
        match e {
            CheckError::EAGAIN => {
                println!(
                    "[{}s] {:?} \x1b[40;32mhas alive\x1b[0m [EAGAIN]",
                    probe_time, conn_elapsed_time
                );
                has_probe_count += 1;
                if has_probe_count >= threads_lists_length {
                    ctrl_sleep_thread_exit_st.send(true).unwrap();
                    println!("out probe time, the connection still alive. maybe never initiative disconnect."); // todo
                    std::process::exit(0);
                }
                continue; // 非常关键！
            }
            CheckError::RESET => {
                // [R]
                println!("[{}s]: {:?} connection \x1b[41;36mclosed [R]\x1b[0m Some connections were reset!", probe_time, conn_elapsed_time);
                // then do recycle probe but now exit;
            }
            CheckError::FIN => {
                // [R] //never run....
                println!("[{}s]: {:?} connection \x1b[41;36mclosed [FIN]\x1b[0m Some connections were killed!", probe_time, conn_elapsed_time);
                // then do recycle probe but now exit;
            }
            CheckError::Refused => {
                println!("[{}s]: {:?} connection \x1b[41;36mclosed [Refused]\x1b[0m Remote host: Connection refused!", probe_time, conn_elapsed_time);
            }
            CheckError::TimedOUT => {
                println!(
                    "[{}s]: {:?} \x1b[31;40m[TIMEOUT]\x1b[0m Some connections were killed!",
                    probe_time, conn_elapsed_time
                );
                // then do recycle probe but now exit;
            }
            _ => {
                println!(
                    "[{}]: {:?} probe check other error!",
                    probe_time, conn_elapsed_time
                );
                std::process::exit(0);
            }
        }

        ctrl_probe_st.send(Signal::Terminated).unwrap();
        // anyway true or false
        ctrl_sleep_thread_exit_st.send(true).unwrap();
        // 第一轮探测探测一个范围，第二轮探测更细颗粒度（60s）的断链时长
        if stage_one {
            stage_one = false;
            has_probe_count = 0;
            // todo recycle probe  not exit;
            let index_element = threads_lists.iter().position(|&x| x == probe_time).unwrap();
            // let (mut a_time, mut b_time) = (0u32, 0u32);
            if index_element == 0 {
                println!("1 min 内断开");
                std::process::exit(0);
            } else {
                let a_time: u32 = threads_lists[index_element - 1];
                let b_time: u32 = threads_lists[index_element];
                // send 第二轮探测
                let mut threads_lists: [u32; 4096] = [0; 4096];
                threads_lists_length = ((b_time - a_time) / 60 + 1) as usize;
                for x in 0..threads_lists_length {
                    threads_lists[x] = a_time + 60 * (x as u32);
                }
                println!("index_element: {} {} {} ", index_element, a_time, b_time,);
                let ctrl_probe_rt_clone = ctrl_probe_rt.clone();
                let probe_st_clone = result_probe_st.clone();
                probe_timing_thread(
                    addr_port,
                    ctrl_probe_rt_clone,
                    probe_st_clone,
                    threads_lists_length,
                );
                let ctrl_sleep_rt_clone = ctrl_sleep_rt.clone();
                let ctrl_probe_st_clone = ctrl_probe_st.clone();
                sleep_timing_thread(
                    threads_lists,
                    ctrl_sleep_rt_clone,
                    ctrl_probe_st_clone,
                    threads_lists_length,
                    false,
                );
                println!("send 第二轮探测");
            }
        } else {
            std::process::exit(0);
        }
    }
}

/// 过一段时间找一个stream发一个包
pub fn sleep_timing_thread(
    threads_lists: [u32; 4096],
    ctrl_terminal_rt: Receiver<bool>,
    ctrl_probe_st: Sender<Signal>,
    stream_pool_real_len: usize,
    _is_be_control: bool,
) -> JoinHandle<()> {
    // ------------------------ sleep thread
    let sleep_probe_thread = thread::spawn(move || {
        let mut stream_index = 0;
        loop {
            // 0usize <= thread_index <---- comparison is useless due to type limits
            if stream_index < stream_pool_real_len {
                let target_sleep_time;
                if stream_index == 0 {
                    target_sleep_time = threads_lists[stream_index];
                } else {
                    target_sleep_time =
                        threads_lists[stream_index] - threads_lists[stream_index - 1];
                }
                let ticker = crossbeam_channel::tick(Duration::from_secs(target_sleep_time as u64));
                select! {
                    recv(ticker) -> _t => {

                        // probe the connection
                        ctrl_probe_st.send(Signal::Run(target_sleep_time)).unwrap();
                        stream_index += 1;
                    },
                    // 自己是否要终止。
                    recv(ctrl_terminal_rt) -> _ => {
                        println!("Sleep thread break");
                        break;  // Err(TryRecvError::Empty) => {}
                    }
                }
                // // 醒来看看自己是否要终止。
                // if is_be_control && ctrl_terminal_rt.is_ready() {
                //     ctrl_terminal_rt.recv().unwrap();
                //     println!("sleep thread break");
                //     break;  // Err(TryRecvError::Empty) => {}
                // }
            } else {
                ctrl_terminal_rt.recv().unwrap();
                println!("Out probe time --> sleep thread exit;");  // std::process::exit(0);
                return;
            }
        }
    }); // ------------------------
    return sleep_probe_thread;
}

// 启动很多个连接，把连接放到池子里面
pub fn probe_timing_thread(
    addr_port: &str,
    ctrl_probe_rt: Receiver<Signal>,
    result_probe_st: Sender<(CheckError, u32, Duration)>,
    stream_pool_length: usize,
) {
    // let addr_port_move = addr_port;
    let addr_port_string: String = String::from(addr_port);
    thread::spawn(move || {
        let mut hung_stream_pool: Vec<(TcpStream, Instant)> = Default::default();
        for stream_index in 1..=stream_pool_length {
            let mut stream = match TcpStream::connect(&addr_port_string) {
                Ok(t) => t,
                Err(e) => {
                    println!("[{}]: {}", "Connection failed", e.kind());
                    std::process::exit(1);
                }
            };
            // stream.set_write_timeout(Some(Duration::new(5, 0)));
            let (is_ok, _) = stream_rw_unit(&mut stream, false, stream_index);
            if !is_ok {
                println!("first read/wirte error, Exit");
                drop(stream);
                return;
            }
            let start_time = std::time::Instant::now();
            hung_stream_pool.push((stream, start_time));
        }
        let mut stream_index = stream_pool_length;
        let mut previous_probe_time = 0u32;
        loop {
            // 等待命令，收到命令后从池子里面捞出来一个stream，发起一次探测，
            match ctrl_probe_rt.recv() {
                Ok(s) => {
                    match s {
                        Signal::Run(mut current_probe_time) => {
                            let (stream, conn_start_time) = hung_stream_pool.pop().unwrap();
                            current_probe_time += previous_probe_time;
                            previous_probe_time = current_probe_time;
                            // println!("{} {}", probe_time, previous_probe_time);
                            let write_content = "Ping";
                            println!("[{}]------will probe------", stream_index);
                            stream.set_write_timeout(Some(Duration::new(1, 0))).unwrap(); // 无效参数，仅仅针对本地写到缓存，而不是完整的链路
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

                                // //  这玩意只不过写了个 宏调用 libc，做了一些封装
                                // #[cfg(target_family = "linux")]
                                #[cfg(target_os = "linux")]
                                match nix_setsockopt(
                                    stream.as_raw_fd(),
                                    TcpUserTimeout,
                                    &(tcp_user_timeout as u32),
                                ) {
                                    Ok(_) => {}
                                    Err(e) => {
                                        println!("nix_lib set sockopt error: {:?}", e)
                                    } // ???? 遗留
                                }
                            } else if cfg!(target_os = "macos") {
                                unsafe {
                                    
                                    let a = setsockopt(
                                        stream.as_raw_fd(),
                                        0x06,
                                        0x80, // optname: TcpUserTimeout
                                        &tcp_user_timeout as *const u32 as *const c_void,
                                        mem::size_of::<c_int>() as u32,
                                    );
                                    if a != 0 {
                                        println!("libc set sockopt error: {}", a);
                                    }
                                }
                            } else {
                                println!("Unsupported platform!");
                                std::process::exit(1);
                            }

                            match send(
                                stream.as_raw_fd(),
                                format!("[{}] {}", stream_index, write_content).as_bytes(),
                                MsgFlags::empty(),
                            ) {
                                Ok(size) => {
                                    if size == 0 {
                                        println!("send --> closed[FIN]")
                                    } else {
                                        println!("send --> size: {}", size);
                                    }
                                }
                                Err(e) => {
                                    match e {
                                        // errno::Errno::EAGAIN | errno::Errno::EWOULDBLOCK => {
                                        errno::Errno::EAGAIN => {
                                            println!("operation would block, Try again, [EAGAIN]");
                                        }
                                        errno::Errno::ECONNRESET => {
                                            // tx.send(Message::FIN(WrapperMessage{addr: stream.peer_addr(), content: "reached EOF,".to_string() } ))
                                            //     .unwrap_or_default();
                                            println!("read closed. reached EOF, maybe[FIN]");
                                            // return;
                                        }
                                        others => {
                                            println!(
                                                "[{}]: {:?} probe send other error: {}!",
                                                stream_index, current_probe_time, others
                                            );
                                        }
                                    }
                                }
                            }

                            println!("[{}]-----check conn after probe----", stream_index);

                            thread::sleep(Duration::from_secs(3));
                            // 如果检测时，tcp还在重试，则此处的错误为：EAGAIN！！！所以一定要确保检测时，已经重试完毕。
                            // 从抓包情况来看，重试的完如果不同系统就直接发reset包，而程序结束时发[FIN]包，
                            // 至于先发reset还是[FIN]，如果正常通信的情况下，互相发完fin，就完了，不会发reset包。
                            // 非正常情况，程序的fin和系统的reset各发各的，互不影响。但是先发reset就不发fin了，反过来不成立
                            // 似乎linux不发送reset--

                            let mut peek_buf = [0u8; 1];
                            // Upon successful completion, recv() shall return the length of the message in bytes. 
                            // If no messages are available to be received and the peer has performed an orderly shutdown,
                            // recv() shall return 0. Otherwise, -1 shall be returned and errno set to indicate the error.
                            match recv(
                                stream.as_raw_fd(),
                                &mut peek_buf,
                                MsgFlags::MSG_PEEK | MsgFlags::MSG_DONTWAIT,
                            ) {
                                Ok(size) => {
                                    if size == 0 {
                                        println!("check nv run closed[FIN]")
                                    } else {
                                        println!("check peek:{}, everything is ok.", size);
                                        // 确保第一次 Ping Pong的时候，把stream读完，不然这里会有数据
                                        // let mut buf:[u8;1024] = [0;1024];
                                        // match stream.read(&mut buf) {
                                        //     Ok(size1) => {
                                        //         if size1 != 0 {
                                        //             println!("<<< {}", from_utf8(&buf[..size1]).unwrap_or_default());
                                        //         } else {
                                        //             println! ("read closed. reached EOF, maybe[FIN]");
                                        //         }
                                        //     }
                                        //     Err(e) => {
                                        //         println!("{:?}", e.kind());
                                        //          println!("first read error:{}", e.kind());
                                        //     }
                                        // }

                                    }
                                }
                                Err(e) => match e {
                                    errno::Errno::EAGAIN => {
                                        result_probe_st
                                            .send((
                                                CheckError::EAGAIN,
                                                current_probe_time,
                                                std::time::Instant::now()
                                                    .duration_since(conn_start_time),
                                            ))
                                            .unwrap();
                                    }
                                    errno::Errno::ECONNRESET => {
                                        result_probe_st
                                            .send((
                                                CheckError::RESET,
                                                current_probe_time,
                                                std::time::Instant::now()
                                                    .duration_since(conn_start_time),
                                            ))
                                            .unwrap();
                                    }
                                    errno::Errno::ETIMEDOUT => {
                                        result_probe_st
                                            .send((
                                                CheckError::TimedOUT,
                                                current_probe_time,
                                                std::time::Instant::now()
                                                    .duration_since(conn_start_time),
                                            ))
                                            .unwrap();
                                    }
                                    errno::Errno::ECONNREFUSED => {
                                        result_probe_st
                                            .send((
                                                CheckError::Refused,
                                                current_probe_time,
                                                std::time::Instant::now()
                                                    .duration_since(conn_start_time),
                                            ))
                                            .unwrap();
                                    }
                                    e => {
                                        println!(
                                            "[{}]: {:?} probe check other error!{}",
                                            stream_index, current_probe_time, e
                                        );
                                        std::process::exit(0);
                                    }
                                },
                            }

                            stream_index -= 1;
                        }
                        Signal::Terminated => {
                            hung_stream_pool.clear(); // close all stream; # todo re probe;
                            println!("receive Terminated --> probe thread exit;");
                            return;
                        }
                    }
                }
                Err(_) => {
                    panic!("recv closed...")
                }
            }
        }
    });
}
