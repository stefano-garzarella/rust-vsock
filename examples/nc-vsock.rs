use nix::sys::socket;
use vsock::Vsock;
use clap::{Arg, App, value_t, crate_authors, crate_version};


fn main() {
    let cmd_args = App::new("nc-vsock")
        .version(crate_version!())
        .author(crate_authors!())
        .about("VSOCK demo app that sent a string to nc like tool")
        .arg(
            Arg::with_name("port")
                .long("port")
                .short("p")
                .takes_value(true)
                .required(true)
                .help("Port to lister"),
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
    vsock.connect(cid, port).unwrap();

    let (name_cid, name_port) = vsock.getsockname().unwrap();

    println!("Connected to {0} port {1}", name_cid, name_port);

    let buf = "Hello world!\n";
    vsock.send(buf.as_bytes(), socket::MsgFlags::empty()).unwrap();
    print!("{}", buf);
}
