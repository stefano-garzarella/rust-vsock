use nix::sys::socket;
use nix::sys::epoll;
use std::io;
use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;
use vsock::Vsock;
use clap::{Arg, App, value_t, crate_authors, crate_version};

const EVENT_REMOTE_IN: u64 = 1;
const EVENT_STDIN_IN: u64 = 2;

fn run(vsock: &Vsock) {
    let mut event;

    let epoll_fd = epoll::epoll_create().unwrap();

    event = epoll::EpollEvent::new(epoll::EpollFlags::EPOLLIN, EVENT_REMOTE_IN);
    epoll::epoll_ctl(epoll_fd, epoll::EpollOp::EpollCtlAdd, vsock.raw_fd(),
                     &mut event).unwrap();

    event = epoll::EpollEvent::new(epoll::EpollFlags::EPOLLIN, EVENT_STDIN_IN);
    epoll::epoll_ctl(epoll_fd, epoll::EpollOp::EpollCtlAdd,
                     io::stdin().as_raw_fd(),
                     &mut event).unwrap();

    loop {
        let mut events = vec![epoll::EpollEvent::empty(); 10];
        let mut buf: [u8; 1024] = unsafe { std::mem::uninitialized() };

        let nfds = match epoll::epoll_wait(epoll_fd, &mut events, -1) {
            Ok(events) => events,
            Err(_) => break,
        };

        for event in events.iter().take(nfds) {
            match event.data() {
                EVENT_REMOTE_IN => {
                    let len = match vsock.recv(&mut buf, socket::MsgFlags::empty()) {
                        Ok(len) => len,
                        Err(err) => {
                            println!("{}", err);
                            std::process::exit(1)
                        },
                    };
                    io::stdout().write(&buf[0 .. len]).unwrap();
                }
                EVENT_STDIN_IN => {
                    let len = io::stdin().read(&mut buf).unwrap();
                    vsock.send(&buf[0 .. len], socket::MsgFlags::empty())
                        .unwrap();
                }
                _ => {
                    panic!("Unknown event!");
                }
            }
        }

    }
}

fn main() {
    let cmd_args = App::new("nc-vsock")
        .version(crate_version!())
        .author(crate_authors!())
        .about("VSOCK demo app -  nc like tool")
        .arg(
            Arg::with_name("port")
                .long("port")
                .short("p")
                .takes_value(true)
                .required(true)
                .help("Port to connect"),
        )
        .arg(
            Arg::with_name("cid")
                .long("cid")
                .short("c")
                .takes_value(true)
                .required(true)
                .help("Remote cid"),
        )
        .get_matches();

    let port = value_t!(cmd_args.value_of("port"), i32).unwrap();
    let cid = value_t!(cmd_args.value_of("cid"), i32).unwrap();

    let vsock = Vsock::new();
    vsock.connect(cid, port).expect("Unable to connect");

    let (name_cid, name_port) = vsock.getsockname().unwrap();

    println!("Connected to {0} port {1}", name_cid, name_port);
    run(&vsock);
}
