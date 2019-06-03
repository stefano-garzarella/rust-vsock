use nix::sys::socket;
use nix::sys::epoll;
use std::io;
use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;
use vsock::{Vsock, VsockCid};
use clap::{Arg, App, value_t, crate_authors, crate_version};

const EVENT_REMOTE_IN: u64 = 1;
const EVENT_STDIN_IN: u64 = 2;

fn nc_vsock(vsock: &Vsock) {
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
            Arg::with_name("listen_port")
                .long("listen")
                .short("l")
                .takes_value(true)
                .group("input")
                .help("Bind and listen for incoming connections"),
        )
        .arg(
            Arg::with_name("remote_cid")
                .takes_value(true)
                .index(1)
                .requires("remote_port")
                .group("input")
                .help("Remote cid to connect to"),
        )
        .arg(
            Arg::with_name("remote_port")
                .takes_value(true)
                .index(2)
                .help("Remote port to connect to"),
        )
        .get_matches();

    let mut vsock = Vsock::new();

    if cmd_args.is_present("listen_port") {
        let port = value_t!(cmd_args.value_of("listen_port"), i32).unwrap();

        vsock.bind(VsockCid::any(), port).unwrap();
        vsock.listen(1).expect("Unable to listen");
        vsock = vsock.accept().unwrap();

        let (name_cid, name_port) = vsock.getpeername().unwrap();
        println!("Connection from cid {0} port {1}...", name_cid, name_port);
    } else {
        let port = value_t!(cmd_args.value_of("remote_port"), i32).unwrap();
        let cid = value_t!(cmd_args.value_of("remote_cid"), i32).unwrap();

        vsock.connect(cid, port).expect("Unable to connect");

        let (name_cid, name_port) = vsock.getsockname().unwrap();
        println!("Connection to cid {0} port {1}...", name_cid, name_port);
    }

    nc_vsock(&vsock);
}
